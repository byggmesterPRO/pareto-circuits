//! Adder-subtractor generator: a + b, or a - b when the mode pin is on.
//!
//! Inputs: a is bits 0 .. n-1, b is bits n .. 2n-1, the mode is bit 2n. Outputs: the result in bits 0 .. n-1,
//! and in bit n the carry out of an addition or the borrow of a subtraction.
//!
//! Subtracting is adding the inverse of b plus one. So b goes through one XOR gate per bit with the mode, the
//! mode is the carry into the lowest bit, and the rest is an adder with any of the adder family's carry
//! networks. The code is `add_sub` in `tools/logic-sim/src/parts.rs`, which the ALU uses too.

use logic_sim::build::{Builder, Design, Netlist};
use logic_sim::parts::{add_sub, inputs, Carry};

pub const SIZES: [usize; 5] = [2, 4, 8, 16, 32];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    let mut out = vec![Design::new("ripple", build(n, None))?, Design::new("lookahead", build(n, Some(Carry::Blocks(n + 1))))?];
    for k in [2, 3, 4, 6, 8, 12, 16, 24, 32] {
        if k < n {
            out.push(Design::new(format!("blocks-{k}"), build(n, Some(Carry::Blocks(k))))?);
        }
    }
    for k in [2, 4, 8, 16] {
        if 2 * k <= n {
            out.push(Design::new(format!("two-level-{k}"), build(n, Some(Carry::TwoLevel(k))))?);
        }
    }
    Ok(out)
}

fn build(n: usize, carry: Option<Carry>) -> Result<Netlist, String> {
    let mut b = Builder::new(2 * n + 1);
    let (x, y, mode) = (inputs(&b, 0, n), inputs(&b, n, n), b.input(2 * n));
    let outs = add_sub(&mut b, &x, &y, mode, carry);
    b.finish(&outs)
}
