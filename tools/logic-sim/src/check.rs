//! Is a circuit without memory correct: try every input (or a large sample) against the family's function.
//!
//! All test inputs of a block run at once, one bit per input in every signal, on every core.

use crate::circuit::{Circuit, Kind};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

/// Up to this many input bits every input is tried.
pub const EXHAUSTIVE_BITS: u32 = 32;
pub const SAMPLES: usize = 1_000_000;
const BLOCK_BITS: u32 = 16;

pub struct Checked {
    /// Test inputs with at least one wrong output bit.
    pub wrong: u64,
    pub tested: u64,
    pub exhaustive: bool,
}

/// Edge cases plus random inputs, for sizes too big to try everything.
pub fn sample(bits: u32) -> Vec<u64> {
    let top = if bits >= 64 { u64::MAX } else { (1u64 << bits) - 1 };
    let mut xs = vec![0, 1, top, top - 1];
    for i in 0..bits {
        xs.extend([1u64 << i, (1u64 << i) - 1, top ^ (1u64 << i)]);
    }
    let mut p: u64 = 1;
    while let Some(next) = p.checked_mul(10) {
        xs.extend([next - 1, next, next / 2].into_iter().filter(|&v| v <= top));
        p = next;
    }
    let mut s: u64 = 0x9E3779B97F4A7C15;
    for _ in 0..SAMPLES {
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        xs.push(s.wrapping_mul(0x2545F4914F6CDD1D) & top);
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

/// Check a combinational circuit against f. `bits` is the number of input pins.
pub fn check(c: &Circuit, f: &(dyn Fn(u64) -> u128 + Sync), threads: usize) -> Result<Checked, String> {
    if !c.combinational() {
        return Err("the circuit has memory, timers or a feedback loop; use the tick simulator".into());
    }
    let bits = c.inputs.len() as u32;
    let order: Vec<u32> = c.order().unwrap().into_iter().filter(|e| !c.inputs.contains(e)).collect();
    let exhaustive = bits <= EXHAUSTIVE_BITS;
    let xs = if exhaustive { vec![] } else { sample(bits) };
    let block_rows: u64 = 1 << bits.min(BLOCK_BITS);
    let tested: u64 = if exhaustive { 1u64 << bits } else { xs.len() as u64 };
    let blocks = tested.div_ceil(block_rows);
    let w = (block_rows as usize).div_ceil(64);
    let next_block = AtomicU64::new(0);
    let wrong = AtomicU64::new(0);
    thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            scope.spawn(|| {
                let mut vals = vec![0u64; c.len() * w];
                let mut acc = vec![0u64; w];
                loop {
                    let k = next_block.fetch_add(1, Ordering::Relaxed);
                    if k >= blocks {
                        break;
                    }
                    let rows = (tested - k * block_rows).min(block_rows) as usize;
                    // inputs
                    for (i, &e) in c.inputs.iter().enumerate() {
                        let row = &mut vals[e as usize * w..(e as usize + 1) * w];
                        if exhaustive {
                            for (j, word) in row.iter_mut().enumerate() {
                                let on = if i < 6 { PATTERN[i] } else if (i as u32) < BLOCK_BITS { if j >> (i - 6) & 1 == 1 { !0 } else { 0 } } else if k >> (i as u32 - BLOCK_BITS) & 1 == 1 { !0 } else { 0 };
                                *word = on;
                            }
                        } else {
                            row.fill(0);
                            for r in 0..rows {
                                if xs[(k * block_rows) as usize + r] >> i & 1 == 1 {
                                    row[r / 64] |= 1 << (r % 64);
                                }
                            }
                        }
                    }
                    // gates, in feed-forward order
                    for &e in &order {
                        let e = e as usize;
                        let Kind::Gate(mode) = c.kind[e] else { unreachable!() };
                        // elements are not stored in feed-forward order, so work in a buffer
                        match c.ins[e].split_first() {
                            None => acc.fill(0),
                            Some((&first, more)) => {
                                acc.copy_from_slice(&vals[first as usize * w..(first as usize + 1) * w]);
                                for &i in more {
                                    let src = &vals[i as usize * w..(i as usize + 1) * w];
                                    match mode % 3 {
                                        0 => acc.iter_mut().zip(src).for_each(|(o, s)| *o &= s),
                                        1 => acc.iter_mut().zip(src).for_each(|(o, s)| *o |= s),
                                        _ => acc.iter_mut().zip(src).for_each(|(o, s)| *o ^= s),
                                    }
                                }
                                if mode >= 3 {
                                    acc.iter_mut().for_each(|o| *o = !*o);
                                }
                            }
                        }
                        vals[e * w..(e + 1) * w].copy_from_slice(&acc);
                    }
                    // compare with the family's function, 64 test inputs at a time
                    let mut bad = 0u64;
                    let mut want = [0u128; 64];
                    for j in 0..rows.div_ceil(64) {
                        let n = (rows - j * 64).min(64);
                        for l in 0..n {
                            let r = j * 64 + l;
                            want[l] = f(if exhaustive { k * block_rows + r as u64 } else { xs[(k * block_rows) as usize + r] });
                        }
                        let mut diff = 0u64;
                        for (b, o) in c.outputs.iter().enumerate() {
                            let mut expect = 0u64;
                            for l in 0..n {
                                expect |= ((want[l] >> b) as u64 & 1) << l;
                            }
                            let got = o.map_or(0, |o| vals[o as usize * w + j]);
                            diff |= got ^ expect;
                        }
                        if n < 64 {
                            diff &= (1u64 << n) - 1;
                        }
                        bad += diff.count_ones() as u64;
                    }
                    wrong.fetch_add(bad, Ordering::Relaxed);
                }
            });
        }
    });
    Ok(Checked { wrong: wrong.into_inner(), tested, exhaustive })
}
