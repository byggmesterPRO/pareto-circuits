//! Absolute value generator: |x| of a two's complement number.
//!
//! Inputs: x, n bits, the top bit is the sign. Outputs: |x|, n bits, read as an unsigned number (so the most
//! negative number, -2^(n-1), comes out right as 2^(n-1)).
//!
//! A negative number is negated: bit i flips when any bit below it is 1. So a bit flips when the sign is on
//! and any lower bit is 1.

use logic_sim::build::{Builder, Design, Netlist};
use logic_sim::families::family;
use logic_sim::parts::{inputs, normal_form, offer};

pub const SIZES: [usize; 6] = [2, 4, 8, 16, 32, 64];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    let mut out = vec![Design::new("flat", flat(n))?];
    if n <= 8 {
        let f = family("absolute", &format!("{n}bit"))?;
        offer(&mut out, "normal-form", normal_form(n, n, &*f.f));
    }
    Ok(out)
}

/// An OR gate over the lower bits, an AND gate with the sign, and an XOR gate, per bit. 4 ticks.
fn flat(n: usize) -> Result<Netlist, String> {
    let mut b = Builder::new(n);
    let x = inputs(&b, 0, n);
    let sign = x[n - 1];
    let mut outs = vec![b.buffer(x[0])];
    for i in 1..n {
        let below = b.or(&x[..i]);
        let flip = b.and(&[sign, below]);
        outs.push(b.xor(&[x[i], flip]));
    }
    b.finish(&outs)
}
