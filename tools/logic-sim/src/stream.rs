//! Does a circuit keep up with inputs that change every tick.
//!
//! A balanced circuit does: every path from an input to an output is the same number of ticks, so an output
//! never mixes the inputs of two different ticks. This runs the circuit on the tick simulator with new random
//! inputs on every tick, and checks on every tick that the outputs are what the inputs of `ticks` ticks ago
//! settle on. `Circuit::balance` says the same from the wiring alone; this is the test of it.
//!
//! A circuit that is not balanced keeps up with slower inputs: new ones every `Balance::period` ticks. With
//! `every` above 1 the inputs are held that long, and the outputs are checked once per input, `ticks` ticks
//! after it was driven.

use crate::check::Flat;
use crate::circuit::Circuit;
use crate::sim::Sim;
use std::collections::VecDeque;

/// Streams that run side by side, each with its own random inputs.
pub const LANES: usize = 256;

pub struct Streamed {
    /// The delay the outputs were checked at: the circuit's ticks.
    pub ticks: u32,
    /// Input values tried, over all streams.
    pub tested: u64,
    /// How many of them gave a wrong output `ticks` later.
    pub wrong: u64,
    /// The first one that did, in words.
    pub example: Option<String>,
}

pub fn stream(c: &Circuit, steps: usize, seed: u64) -> Result<Streamed, String> {
    stream_every(c, steps, 1, seed)
}

/// The same with each input held for `every` ticks.
pub fn stream_every(c: &Circuit, steps: usize, every: u32, seed: u64) -> Result<Streamed, String> {
    if every == 0 || steps == 0 {
        return Err("stream needs at least one input, and at least a tick for each".into());
    }
    let flat = Flat::new(c)?;
    let ticks = c.ticks().ok_or("no input reaches an output")?;
    let (bits, w) = (c.inputs.len(), LANES / 64);
    let mut s = 0x9E3779B97F4A7C15u64 ^ seed.wrapping_mul(0xD1B54A32D192ED03);
    let mut word = || {
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        s.wrapping_mul(0x2545F4914F6CDD1D)
    };
    let mut sim = Sim::new(c, LANES);
    let mut sent: VecDeque<(u64, Vec<u64>)> = VecDeque::new();
    let mut vals = vec![0u64; flat.slots() * w];
    let (mut wrong, mut example) = (0u64, None);
    for t in 1..=(steps as u64 - 1) * every as u64 + ticks as u64 {
        if (t - 1) % every as u64 == 0 && (t - 1) / (every as u64) < steps as u64 {
            let x: Vec<u64> = (0..bits * w).map(|_| word()).collect();
            for (i, &e) in c.inputs.iter().enumerate() {
                sim.force(e, &x[i * w..(i + 1) * w]);
            }
            // the input gates show these inputs after this tick, tick 1 of their way through, and the
            // outputs show them `ticks` - 1 ticks after that
            sent.push_back((t + ticks as u64 - 1, x));
        }
        sim.tick();
        if sent.front().is_none_or(|s| s.0 != t) {
            continue;
        }
        let x = sent.pop_front().unwrap().1;
        vals[..bits * w].copy_from_slice(&x);
        flat.eval(&mut vals, w);
        let mut bad = vec![0u64; w];
        let mut first = None;
        for (b, o) in flat.outputs.iter().enumerate() {
            let (Some(o), Some(e)) = (o, c.outputs[b]) else { continue };
            let got = sim.get(e);
            for q in 0..w {
                let diff = got[q] ^ vals[*o as usize * w + q];
                if diff != 0 && first.is_none() {
                    first = Some(b);
                }
                bad[q] |= diff;
            }
        }
        wrong += bad.iter().map(|d| d.count_ones() as u64).sum::<u64>();
        if let (None, Some(b)) = (&example, first) {
            example = Some(format!("output {b} at tick {t} is not what the inputs of tick {} give", t + 1 - ticks as u64));
        }
    }
    Ok(Streamed { ticks, tested: steps as u64 * LANES as u64, wrong, example })
}
