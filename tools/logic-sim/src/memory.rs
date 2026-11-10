//! Checking circuits with memory: run random operations on the tick simulator, 64 copies of the circuit at
//! once, and compare the outputs with a model of what the family should do.
//!
//! One operation:
//! 1. The level inputs (data, address) are set and the circuit is left until nothing changes.
//! 2. At most one pulse input (write, count, reset) is turned on for exactly one tick.
//! 3. The circuit is left until nothing changes, and the outputs must be what the model says.
//!
//! The delays are measured on the way: a tick is counted from the moment an input is driven, so the input
//! gate itself is the first tick, the same way the main README counts ticks for circuits without memory.
//!
//! That gives every operation all the time it wants. `pace` asks the other question: how often can operations
//! come. An operation there is a fixed number of ticks, the period: the level inputs change at its start, the
//! pulse (if there is one) comes `pulse_at` ticks later, and at its end, the moment before the next one
//! starts, the outputs have to be what the model says. `pace` gives the shortest period that works, with the
//! pulse wherever in the period it works. This is for a circuit with memory what balanced is for one without:
//! there the period is 1.

use crate::circuit::Circuit;
use crate::families::Machine;
use crate::sim::Sim;

pub struct Outcome {
    /// Operations (over all copies) that ended with a wrong output.
    pub wrong: u64,
    pub operations: u64,
    /// Most ticks from a level input changing until the outputs are final.
    pub read: u64,
    /// Most ticks from a pulse starting until the outputs are final.
    pub write: u64,
    /// Most ticks from a level input changing until everything inside is quiet: wait this long before a pulse.
    pub setup: u64,
    /// What went wrong first, in words.
    pub example: Option<String>,
}

const LANES: usize = 64;
const LIMIT: u64 = 400;

struct Random(u64);
impl Random {
    fn word(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545F4914F6CDD1D)
    }
}

/// Tick until quiet. Returns (ticks until quiet, ticks until the outputs were what they should be for good).
fn run_out(sim: &mut Sim, c: &Circuit, want: &[u128], mut first: impl FnMut(&mut Sim)) -> Result<(u64, u64, u64), String> {
    let (mut ticks, mut right_from) = (0, 0);
    let mut bad_lanes;
    loop {
        let changed = sim.tick();
        ticks += 1;
        if ticks == 1 {
            first(sim);
        }
        bad_lanes = 0;
        for (b, o) in c.outputs.iter().enumerate() {
            let have = o.map_or(0, |o| sim.get(o)[0]);
            let mut should = 0u64;
            for (lane, w) in want.iter().enumerate() {
                should |= ((w >> b & 1) as u64) << lane;
            }
            bad_lanes |= have ^ should;
        }
        if bad_lanes != 0 {
            right_from = ticks + 1;
        }
        if !changed && ticks > 1 {
            return Ok((ticks - 1, right_from, bad_lanes));
        }
        if ticks > LIMIT {
            return Err(format!("still changing after {LIMIT} ticks"));
        }
    }
}

pub fn run(c: &Circuit, m: &Machine, operations: u64, seed: u64) -> Result<Outcome, String> {
    if c.inputs.len() as u32 != m.bits || c.outputs.len() as u32 != m.out_bits {
        return Err(format!("{} inputs and {} outputs, the family has {} and {}", c.inputs.len(), c.outputs.len(), m.bits, m.out_bits));
    }
    let mut sim = Sim::new(c, LANES);
    let mut r = Random(seed | 1);
    let mut states: Vec<Vec<u128>> = vec![(m.start)(); LANES];
    let mut out = Outcome { wrong: 0, operations: 0, read: 0, write: 0, setup: 0, example: None };
    let levels: Vec<u32> = (0..m.bits).filter(|b| !m.pulses.contains(b)).collect();
    for e in &c.inputs {
        sim.force(*e, &[0]);
    }
    for op in 0..operations {
        // 1: new levels in every copy
        let mut inputs = [0u128; LANES];
        for &b in &levels {
            let word = if op % 7 == 3 { r.word() & r.word() } else { r.word() };
            sim.force(c.inputs[b as usize], &[word]);
            for (lane, x) in inputs.iter_mut().enumerate() {
                *x |= ((word >> lane & 1) as u128) << b;
            }
        }
        let want: Vec<u128> = (0..LANES).map(|l| (m.step)(&mut states[l].clone(), inputs[l])).collect();
        let (quiet, right, bad) = run_out(&mut sim, c, &want, |_| {})?;
        out.setup = out.setup.max(quiet);
        out.read = out.read.max(right);
        // 2: one pulse, or none, in every copy
        let mut masks = vec![0u64; m.pulses.len()];
        if !m.pulses.is_empty() {
            for lane in 0..LANES {
                let pick = (r.word() % (m.pulses.len() as u64 * 3 + 1)) as usize / 3;
                if pick < m.pulses.len() {
                    masks[pick] |= 1 << lane;
                    inputs[lane] |= 1u128 << m.pulses[pick];
                }
            }
        }
        let want: Vec<u128> = (0..LANES).map(|l| (m.step)(&mut states[l], inputs[l])).collect();
        for (k, &p) in m.pulses.iter().enumerate() {
            sim.force(c.inputs[p as usize], &[masks[k]]);
        }
        let (_, right2, bad2) = run_out(&mut sim, c, &want, |sim| {
            for &p in &m.pulses {
                sim.force(c.inputs[p as usize], &[0]);
            }
        })?;
        out.write = out.write.max(right2);
        out.operations += 2 * LANES as u64;
        let wrong = (bad | bad2).count_ones() as u64;
        if wrong > 0 && out.example.is_none() {
            let lane = (bad | bad2).trailing_zeros() as usize;
            out.example = Some(format!("operation {op}, inputs {:#x}: outputs should be {:#x}", inputs[lane], want[lane]));
        }
        out.wrong += bad.count_ones() as u64 + bad2.count_ones() as u64;
    }
    Ok(out)
}

/// How often operations can come (see the top of this file).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Pace {
    /// Ticks from the start of one operation to the start of the next.
    pub period: u64,
    /// Ticks from the level inputs changing to the pulse, within an operation.
    pub pulse_at: u64,
}

/// Operations in a row at a period: true if the outputs were right at the end of every one.
fn paced(c: &Circuit, m: &Machine, pace: Pace, operations: u64, seed: u64) -> Result<bool, String> {
    let mut sim = Sim::new(c, LANES);
    let mut r = Random(seed | 1);
    let mut states: Vec<Vec<u128>> = vec![(m.start)(); LANES];
    let levels: Vec<u32> = (0..m.bits).filter(|b| !m.pulses.contains(b)).collect();
    for e in &c.inputs {
        sim.force(*e, &[0]);
    }
    sim.settle(LIMIT).ok_or(format!("still changing after {LIMIT} ticks with all inputs off"))?;
    let mut pulsed = false;
    for op in 0..operations {
        let mut inputs = [0u128; LANES];
        let mut words = vec![];
        for &b in &levels {
            let word = if op % 7 == 3 { r.word() & r.word() } else { r.word() };
            words.push(word);
            for (lane, x) in inputs.iter_mut().enumerate() {
                *x |= ((word >> lane & 1) as u128) << b;
            }
        }
        // one pulse, or none, in every copy
        let mut masks = vec![0u64; m.pulses.len()];
        if !m.pulses.is_empty() {
            for (lane, x) in inputs.iter_mut().enumerate() {
                let pick = (r.word() % (m.pulses.len() as u64 * 3 + 1)) as usize / 3;
                if pick < m.pulses.len() {
                    masks[pick] |= 1 << lane;
                    *x |= 1u128 << m.pulses[pick];
                }
            }
        }
        let want: Vec<u128> = (0..LANES).map(|l| (m.step)(&mut states[l], inputs[l])).collect();
        for t in 0..pace.period {
            // a pulse is on for exactly one tick, also when that tick is the last of the operation
            if pulsed {
                m.pulses.iter().for_each(|&p| sim.force(c.inputs[p as usize], &[0]));
                pulsed = false;
            }
            if t == 0 {
                levels.iter().zip(&words).for_each(|(&b, &word)| sim.force(c.inputs[b as usize], &[word]));
            }
            if t == pace.pulse_at {
                m.pulses.iter().zip(&masks).for_each(|(&p, &mask)| sim.force(c.inputs[p as usize], &[mask]));
                pulsed = true;
            }
            sim.tick();
        }
        for (b, o) in c.outputs.iter().enumerate() {
            let have = o.map_or(0, |o| sim.get(o)[0]);
            let should = want.iter().enumerate().fold(0u64, |acc, (lane, w)| acc | ((w >> b & 1) as u64) << lane);
            if have != should {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Operations a period is tried on before it is looked at closer, and before it is believed.
const QUICK: u64 = 200;
const SURE: u64 = 3000;

/// The shortest period at which operations in a row work, up to `most` ticks, and where in it the pulse goes
/// (the earliest place that works). None if no period up to `most` works. This is what was seen to work on
/// random operations, not a proof; and a longer period is not checked to work as well.
pub fn pace(c: &Circuit, m: &Machine, most: u64, seed: u64) -> Result<Option<Pace>, String> {
    if c.inputs.len() as u32 != m.bits || c.outputs.len() as u32 != m.out_bits {
        return Err(format!("{} inputs and {} outputs, the family has {} and {}", c.inputs.len(), c.outputs.len(), m.bits, m.out_bits));
    }
    for period in 1..=most {
        for pulse_at in 0..period {
            let pace = Pace { period, pulse_at };
            if paced(c, m, pace, QUICK, seed)? && paced(c, m, pace, SURE, seed.wrapping_add(1))? {
                return Ok(Some(pace));
            }
        }
    }
    Ok(None)
}
