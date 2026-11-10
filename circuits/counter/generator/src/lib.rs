//! Counter generator: counts pulses, n bits, wrapping around to 0 after the largest value.
//!
//! Inputs: bit 0 is count, bit 1 is reset. Outputs: the count.
//!
//! A bit is stored in an XOR gate wired to itself, which flips when another input is on for one tick. Bit i
//! flips on a count when every lower bit is 1, which is one AND gate, and on a reset when it is 1, which is
//! another. All bits see the count pulse at the same time, so nothing walks from bit to bit.
//! Count and reset have to be on for exactly one tick, and not both at once.

use logic_sim::build::{Builder, Design, Netlist, S, XOR};

pub const SIZES: [usize; 6] = [2, 4, 8, 16, 32, 64];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    Ok(vec![Design::new("toggle", toggle(n))?])
}

fn toggle(n: usize) -> Result<Netlist, String> {
    let mut b = Builder::new(2);
    let (count, reset) = (b.input(0), b.input(1));
    let bits: Vec<S> = (0..n).map(|_| b.later(XOR)).collect();
    for i in 0..n {
        let mut below = vec![count];
        below.extend_from_slice(&bits[..i]);
        let carry = b.and(&below);
        let clear = b.and(&[reset, bits[i]]);
        b.wire(bits[i], &[bits[i], carry, clear]);
    }
    b.finish(&bits)
}
