//! Worst output delay actually seen when the inputs change.
//!
//! `ticks` (the longest path) is a guarantee that no input change can exceed. Many circuits never use their
//! longest path, so what really happens can be lower. This jumps from one random input value to another, with
//! all jumps running at once in the tick simulator, and reports the last tick at which any output changed.
//! It counts like `ticks`: the input gate is tick 1. The result is a measurement, not a guarantee.

use crate::circuit::Circuit;
use crate::sim::Sim;

pub fn worst_seen(c: &Circuit, transitions: usize, seed: u64) -> Result<u64, String> {
    let limit = match c.depths() {
        Some(d) if c.combinational() => *d.iter().max().unwrap_or(&1) as u64 + 1,
        _ => return Err("delay needs a circuit without memory, timers or feedback".into()),
    };
    let bits = c.inputs.len();
    let mut sim = Sim::new(c, transitions);
    let w = sim.w;
    let mut s = 0x9E3779B97F4A7C15u64 ^ seed.wrapping_mul(0xD1B54A32D192ED03);
    let mut random = |sim: &mut Sim| {
        // one random word per input bit and word: every lane gets an independent random input value
        for i in 0..bits {
            let mut row = vec![0u64; w];
            for word in row.iter_mut() {
                s ^= s >> 12;
                s ^= s << 25;
                s ^= s >> 27;
                *word = s.wrapping_mul(0x2545F4914F6CDD1D);
            }
            if transitions % 64 != 0 {
                row[w - 1] &= (1u64 << (transitions % 64)) - 1;
            }
            sim.force(c.inputs[i], &row);
        }
    };
    random(&mut sim);
    for _ in 0..limit {
        sim.tick();
    }
    random(&mut sim);
    let outs: Vec<u32> = c.outputs.iter().flatten().copied().collect();
    let mut worst = 1;
    for t in 1..=limit {
        let before: Vec<u64> = outs.iter().flat_map(|&o| sim.get(o).to_vec()).collect();
        sim.tick();
        if outs.iter().enumerate().any(|(k, &o)| sim.get(o) != &before[k * w..(k + 1) * w]) {
            worst = t;
        }
    }
    Ok(worst)
}
