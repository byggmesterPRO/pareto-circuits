//! Negate generator: -x in two's complement.
//!
//! Inputs: x, n bits. Outputs: -x, n bits. (0 stays 0, and the most negative number stays itself.)
//!
//! Negating keeps every bit up to and including the lowest 1, and flips the ones above it. So bit i flips
//! when any bit below it is 1: one OR gate over the lower bits and one XOR gate. 3 ticks at any size.
//! Bit 0 never changes; it goes through a gate that only copies it, so that the output has a gate of its own.

use logic_sim::build::{Builder, Design, Netlist};
use logic_sim::parts::inputs;

pub const SIZES: [usize; 6] = [2, 4, 8, 16, 32, 64];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    Ok(vec![Design::new("flat", flat(n))?])
}

fn flat(n: usize) -> Result<Netlist, String> {
    let mut b = Builder::new(n);
    let x = inputs(&b, 0, n);
    let mut outs = vec![b.buffer(x[0])];
    for i in 1..n {
        let below = b.or(&x[..i]);
        outs.push(b.xor(&[x[i], below]));
    }
    b.finish(&outs)
}
