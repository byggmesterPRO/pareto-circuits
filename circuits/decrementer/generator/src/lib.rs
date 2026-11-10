//! Decrementer generator: x - 1.
//!
//! Inputs: x, n bits. Outputs: x - 1 in bits 0 .. n-1 (0 wraps around to all ones) and the borrow in bit n
//! (on when x was 0).
//!
//! Bit i flips when every bit below it is 0. That is one NOR gate over the lower bits and one XOR gate, for
//! every bit at once: 3 ticks at any size.

use logic_sim::build::{Builder, Design, Netlist};
use logic_sim::parts::inputs;

pub const SIZES: [usize; 6] = [2, 4, 8, 16, 32, 64];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    Ok(vec![Design::new("flat", flat(n))?])
}

fn flat(n: usize) -> Result<Netlist, String> {
    let mut b = Builder::new(n);
    let x = inputs(&b, 0, n);
    let mut outs = vec![b.not(x[0])];
    for i in 1..n {
        let below = b.nor(&x[..i]);
        outs.push(b.xor(&[x[i], below]));
    }
    outs.push(b.nor(&x));
    b.finish(&outs)
}
