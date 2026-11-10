//! Is a circuit without memory correct: try every input (or a chosen set) against what it should give.
//!
//! All test inputs of a block run at once, one bit per input in every signal, on every core. What it should
//! give can be a function, a table with one value per input, a list of cases, or another circuit.

use crate::circuit::{Circuit, Kind};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread;

/// Up to this many input bits every input is tried.
pub const EXHAUSTIVE_BITS: u32 = 32;
pub const SAMPLES: usize = 1_000_000;
/// Inputs are numbers of up to this many bits. So are outputs, unless the expectation is `Wide` or a circuit.
pub const MAX_BITS: u32 = 128;

pub enum Expect<'a> {
    /// The correct output for an input.
    Function(&'a (dyn Fn(u128) -> u128 + Sync)),
    /// The same, plus a quicker way to get the outputs of up to 64 inputs in a row: fill(first input, low 64
    /// bits of each output, the bits above those). Used when every input is tried.
    Run(&'a (dyn Fn(u128) -> u128 + Sync), &'a (dyn Fn(u64, &mut [u64], &mut [u64]) + Sync)),
    /// One value per input, for every input.
    Table(&'a [u128]),
    /// Chosen inputs and the output each must give.
    Cases(&'a [(u128, u128)]),
    /// For more than 128 outputs: sets the output bits that are on for an input, in words of 64 that start
    /// out as zero.
    Wide(&'a (dyn Fn(u128, &mut [u64]) + Sync)),
    /// Another circuit with the same pins: the two must agree.
    Circuit(&'a Circuit),
}

pub struct Checked {
    /// Test inputs with at least one wrong output bit.
    pub wrong: u64,
    pub tested: u64,
    pub exhaustive: bool,
    /// The smallest input that gave a wrong output.
    pub example: Option<u128>,
}

/// Inputs to try when there are too many to try them all: edge cases, the family's own hard cases, and
/// random ones.
pub fn sample(bits: u32, hard: &[u128]) -> Vec<u128> {
    let top = if bits >= 128 { u128::MAX } else { (1u128 << bits) - 1 };
    let mut xs = vec![0, 1, top, top - 1];
    xs.extend(hard.iter().map(|&x| x & top));
    for i in 0..bits {
        xs.extend([1u128 << i, (1u128 << i) - 1, top ^ (1u128 << i)]);
    }
    let mut p: u128 = 1;
    while let Some(next) = p.checked_mul(10) {
        xs.extend([next - 1, next, next / 2].into_iter().filter(|&v| v <= top));
        p = next;
    }
    let mut s: u64 = 0x9E3779B97F4A7C15;
    let mut word = || {
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        s.wrapping_mul(0x2545F4914F6CDD1D)
    };
    for _ in 0..SAMPLES {
        xs.push((word() as u128 | (word() as u128) << 64) & top);
    }
    xs.sort_unstable();
    xs.dedup();
    xs
}

const PATTERN: [u64; 6] = [
    0xAAAAAAAAAAAAAAAA,
    0xCCCCCCCCCCCCCCCC,
    0xF0F0F0F0F0F0F0F0,
    0xFF00FF00FF00FF00,
    0xFFFF0000FFFF0000,
    0xFFFFFFFF00000000,
];

/// Turn 64 values into 64 columns: afterwards bit r of a[b] is bit b of the old a[r].
pub fn transpose64(a: &mut [u64; 64]) {
    let mut j = 32;
    let mut m: u64 = 0x0000_0000_FFFF_FFFF;
    while j != 0 {
        let mut k = 0;
        while k < 64 {
            let t = ((a[k] >> j) ^ a[k + j]) & m;
            a[k] ^= t << j;
            a[k + j] ^= t;
            k = (k + j + 1) & !j;
        }
        j >>= 1;
        m ^= m << j;
    }
}

/// A circuit without memory laid out for evaluation: slot i < inputs is input bit i, then one slot per gate
/// in an order where every gate comes after what it reads. A gate can then write straight into its slot.
struct Flat {
    inputs: usize,
    gates: Vec<(u8, Vec<u32>)>,
    outputs: Vec<Option<u32>>,
}

impl Flat {
    fn new(c: &Circuit) -> Result<Flat, String> {
        if !c.combinational() {
            return Err("the circuit has memory, timers or a feedback loop; use a test script on the tick simulator".into());
        }
        let mut slot = vec![u32::MAX; c.len()];
        for (i, &e) in c.inputs.iter().enumerate() {
            if slot[e as usize] != u32::MAX {
                return Err(format!("input {i} is the same part as an earlier input"));
            }
            slot[e as usize] = i as u32;
        }
        let mut gates = vec![];
        for e in c.order().unwrap() {
            let e = e as usize;
            if slot[e] != u32::MAX {
                continue;
            }
            let Kind::Gate(mode) = c.kind[e] else { unreachable!() };
            slot[e] = (c.inputs.len() + gates.len()) as u32;
            gates.push((mode, c.ins[e].iter().map(|&i| slot[i as usize]).collect()));
        }
        Ok(Flat { inputs: c.inputs.len(), gates, outputs: c.outputs.iter().map(|o| o.map(|o| slot[o as usize])).collect() })
    }

    fn slots(&self) -> usize {
        self.inputs + self.gates.len()
    }

    fn eval(&self, vals: &mut [u64], w: usize) {
        for (g, (mode, ins)) in self.gates.iter().enumerate() {
            let (before, rest) = vals.split_at_mut((self.inputs + g) * w);
            let out = &mut rest[..w];
            match ins.split_first() {
                None => out.fill(0),
                Some((&first, more)) => {
                    out.copy_from_slice(&before[first as usize * w..(first as usize + 1) * w]);
                    for &i in more {
                        let src = &before[i as usize * w..(i as usize + 1) * w];
                        match mode % 3 {
                            0 => out.iter_mut().zip(src).for_each(|(o, s)| *o &= s),
                            1 => out.iter_mut().zip(src).for_each(|(o, s)| *o |= s),
                            _ => out.iter_mut().zip(src).for_each(|(o, s)| *o ^= s),
                        }
                    }
                    if *mode >= 3 {
                        out.iter_mut().for_each(|o| *o = !*o);
                    }
                }
            }
        }
    }
}

/// Check a circuit without memory. Its input pins are the input bits, least significant first.
/// `hard` is extra inputs to include when the inputs are sampled.
pub fn check(c: &Circuit, expect: &Expect, threads: usize, hard: &[u128]) -> Result<Checked, String> {
    let flat = Flat::new(c)?;
    let bits = c.inputs.len() as u32;
    if bits > MAX_BITS {
        return Err(format!("{bits} input bits; this checker handles up to {MAX_BITS}"));
    }
    if c.outputs.len() as u32 > MAX_BITS && !matches!(expect, Expect::Wide(_) | Expect::Circuit(_)) {
        return Err(format!("{} output bits; above {MAX_BITS} the expected outputs have to be given bit by bit", c.outputs.len()));
    }
    let reference = match expect {
        Expect::Circuit(r) => {
            if r.inputs.len() != c.inputs.len() || r.outputs.len() != c.outputs.len() {
                return Err(format!("the two circuits have different pins: {} in, {} out against {} in, {} out", c.inputs.len(), c.outputs.len(), r.inputs.len(), r.outputs.len()));
            }
            Some(Flat::new(r)?)
        }
        _ => None,
    };
    // which inputs to try
    let listed: Option<Vec<u128>> = match expect {
        Expect::Cases(cases) => Some(cases.iter().map(|c| c.0).collect()),
        Expect::Table(t) => {
            if bits > 40 || t.len() as u64 != 1u64 << bits {
                return Err(format!("the table has {} values, {bits} input bits need {}", t.len(), 1u128 << bits.min(127)));
            }
            None
        }
        _ if bits > EXHAUSTIVE_BITS => Some(sample(bits, hard)),
        _ => None,
    };
    let exhaustive = listed.is_none();
    let tested: u64 = listed.as_ref().map_or_else(|| 1u64 << bits, |l| l.len() as u64);
    // rows per block: small enough that all signals of a block stay in the CPU cache
    let biggest = flat.slots().max(reference.as_ref().map_or(0, |r| r.slots()));
    let budget = std::env::var("LOGIC_SIM_BLOCK_KB").ok().and_then(|v| v.parse().ok()).unwrap_or(512usize) * 1024;
    let mut block_bits = 16u32;
    while block_bits > 6 && biggest * (1usize << (block_bits - 6)) * 8 > budget {
        block_bits -= 1;
    }
    let block_bits = if exhaustive { block_bits.min(bits) } else { block_bits };
    let block_rows: u64 = 1 << block_bits;
    let blocks = tested.div_ceil(block_rows);
    let w = (block_rows as usize).div_ceil(64);
    let next_block = AtomicU64::new(0);
    let wrong = AtomicU64::new(0);
    let example: Mutex<Option<u128>> = Mutex::new(None);
    let (listed, reference, flat) = (&listed, &reference, &flat);
    thread::scope(|scope| {
        for _ in 0..threads.max(1).min(blocks.max(1) as usize) {
            scope.spawn(|| {
                let mut vals = vec![0u64; flat.slots() * w];
                let mut ref_vals = vec![0u64; reference.as_ref().map_or(0, |r| r.slots()) * w];
                let (mut lo, mut hi) = ([0u64; 64], [0u64; 64]);
                let wide = flat.outputs.len() > 64;
                let chunks = flat.outputs.len().div_ceil(64);
                let mut want = vec![[0u64; 64]; if matches!(expect, Expect::Wide(_)) { chunks } else { 0 }];
                let mut buf = vec![0u64; chunks];
                loop {
                    let k = next_block.fetch_add(1, Ordering::Relaxed);
                    if k >= blocks {
                        break;
                    }
                    let rows = (tested - k * block_rows).min(block_rows) as usize;
                    let x_at = |r: usize| match listed {
                        Some(l) => l[(k * block_rows) as usize + r],
                        None => (k * block_rows + r as u64) as u128,
                    };
                    for i in 0..flat.inputs {
                        let row = &mut vals[i * w..(i + 1) * w];
                        if exhaustive {
                            let i = i as u32;
                            for (j, word) in row.iter_mut().enumerate() {
                                let on = if i < 6 { PATTERN[i as usize] } else if i < block_bits { if j >> (i - 6) & 1 == 1 { !0 } else { 0 } } else if k >> (i - block_bits) & 1 == 1 { !0 } else { 0 };
                                *word = on;
                            }
                        } else {
                            row.fill(0);
                            for r in 0..rows {
                                if x_at(r) >> i & 1 == 1 {
                                    row[r / 64] |= 1 << (r % 64);
                                }
                            }
                        }
                    }
                    flat.eval(&mut vals, w);
                    if let Some(rf) = reference {
                        ref_vals[..flat.inputs * w].copy_from_slice(&vals[..flat.inputs * w]);
                        rf.eval(&mut ref_vals, w);
                    }
                    let mut bad = 0u64;
                    for j in 0..rows.div_ceil(64) {
                        let n = (rows - j * 64).min(64);
                        let mut diff = 0u64;
                        if let Some(rf) = reference {
                            for (o, ro) in flat.outputs.iter().zip(&rf.outputs) {
                                diff |= o.map_or(0, |o| vals[o as usize * w + j]) ^ ro.map_or(0, |o| ref_vals[o as usize * w + j]);
                            }
                        } else if let Expect::Wide(f) = expect {
                            want.iter_mut().for_each(|ch| ch.fill(0));
                            for l in 0..n {
                                buf.fill(0);
                                f(x_at(j * 64 + l), &mut buf);
                                want.iter_mut().zip(&buf).for_each(|(ch, &v)| ch[l] = v);
                            }
                            want.iter_mut().for_each(transpose64);
                            for (b, o) in flat.outputs.iter().enumerate() {
                                diff |= o.map_or(0, |o| vals[o as usize * w + j]) ^ want[b / 64][b % 64];
                            }
                        } else {
                            // the wanted values of 64 test inputs, turned into one word per output bit
                            hi.fill(0);
                            lo.fill(0);
                            match expect {
                                Expect::Run(_, fill) if exhaustive => fill(x_at(j * 64) as u64, &mut lo[..n], &mut hi[..n]),
                                _ => {
                                    for l in 0..n {
                                        let x = x_at(j * 64 + l);
                                        let want = match expect {
                                            Expect::Function(f) | Expect::Run(f, _) => f(x),
                                            Expect::Table(t) => t[x as usize],
                                            Expect::Cases(cases) => cases[(k * block_rows) as usize + j * 64 + l].1,
                                            Expect::Circuit(_) | Expect::Wide(_) => unreachable!(),
                                        };
                                        lo[l] = want as u64;
                                        hi[l] = (want >> 64) as u64;
                                    }
                                }
                            }
                            transpose64(&mut lo);
                            if wide {
                                transpose64(&mut hi);
                            }
                            for (b, o) in flat.outputs.iter().enumerate() {
                                let expect = if b < 64 { lo[b] } else { hi[b - 64] };
                                diff |= o.map_or(0, |o| vals[o as usize * w + j]) ^ expect;
                            }
                        }
                        if n < 64 {
                            diff &= (1u64 << n) - 1;
                        }
                        if diff != 0 {
                            bad += diff.count_ones() as u64;
                            let x = x_at(j * 64 + diff.trailing_zeros() as usize);
                            let mut e = example.lock().unwrap();
                            if e.is_none_or(|old| x < old) {
                                *e = Some(x);
                            }
                        }
                    }
                    wrong.fetch_add(bad, Ordering::Relaxed);
                }
            });
        }
    });
    Ok(Checked { wrong: wrong.into_inner(), tested, exhaustive, example: example.into_inner().unwrap() })
}
