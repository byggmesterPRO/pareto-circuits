//! Subtractor generator: several ways to take one n-bit number from another, at any size.
//!
//! Inputs: a is bits 0 .. n-1, b is bits n .. 2n-1. Outputs: a - b in bits 0 .. n-1, the borrow in bit n.
//!
//! A borrow works like an adder's carry. Bit j starts one when a is 0 and b is 1, and passes one on when a
//! and b are equal. A borrow reaches a bit from exactly one place, so the terms "started at j, passed on by
//! everything in between" are never on two at a time and a difference bit is one XNOR gate over them.

use logic_sim::build::{Builder, Design, Netlist, S};
use logic_sim::families::family;
use logic_sim::parts::{carries, inputs, normal_form, offer, Carry};

pub const SIZES: [usize; 6] = [2, 4, 8, 16, 32, 64];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    let mut out = vec![Design::new("ripple", ripple(n))?, Design::new("lookahead", with(n, Carry::Blocks(n)))?];
    for k in [2, 3, 4, 6, 8, 12, 16, 24, 32] {
        if k < n {
            out.push(Design::new(format!("blocks-{k}"), with(n, Carry::Blocks(k)))?);
        }
    }
    for k in [2, 4, 8, 16] {
        if 2 * k <= n {
            out.push(Design::new(format!("two-level-{k}"), with(n, Carry::TwoLevel(k)))?);
        }
    }
    if n <= 4 {
        let f = family("subtractor", &format!("{n}bit"))?;
        offer(&mut out, "normal-form", normal_form(2 * n, n + 1, &*f.f));
    }
    Ok(out)
}

/// One full subtractor per bit, the borrow walking up. The cell is 4 gates, the fewest there is (exact
/// search): difference = xor(a, b, borrow), borrow out = xor(a, nor(a, b, borrow), nand(a, b, borrow)).
fn ripple(n: usize) -> Result<Netlist, String> {
    let mut b = Builder::new(2 * n);
    let (x, y) = (inputs(&b, 0, n), inputs(&b, n, n));
    let (mut borrow, mut outs) = (S::Zero, vec![]);
    for i in 0..n {
        let ins = [x[i], y[i], borrow];
        let diff = b.xor(&ins);
        borrow = if i == 0 {
            b.and(&[y[i], diff])
        } else {
            let (none, not_all) = (b.nor(&ins), b.nand(&ins));
            b.xor(&[x[i], none, not_all])
        };
        outs.push(diff);
    }
    outs.push(borrow);
    b.finish(&outs)
}

fn with(n: usize, carry: Carry) -> Result<Netlist, String> {
    let mut b = Builder::new(2 * n);
    let (x, y) = (inputs(&b, 0, n), inputs(&b, n, n));
    let same: Vec<S> = (0..n).map(|i| b.xnor(&[x[i], y[i]])).collect();
    let starts: Vec<Vec<S>> = (0..n).map(|i| vec![b.not(x[i]), y[i]]).collect();
    let borrows = carries(&mut b, &starts, &same, carry);
    let mut outs = vec![];
    for i in 0..n {
        // a xor b xor borrow, written with "a equals b"
        let mut xs = borrows[i].clone();
        xs.push(same[i]);
        outs.push(b.xnor(&xs));
    }
    outs.push(b.xor(&borrows[n]));
    b.finish(&outs)
}
