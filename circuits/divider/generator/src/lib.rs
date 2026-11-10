//! Divider generator: a divided by b, with quotient and remainder.
//!
//! Inputs: a is bits 0 .. n-1, b is bits n .. 2n-1. Outputs: the quotient in bits 0 .. n-1, the remainder in
//! bits n .. 2n-1. Dividing by 0 gives a quotient of all ones and a as the remainder.
//!
//! Long division, one row per quotient bit from the top: bring down the next bit of a, and if what is left
//! is at least b, take b away and the quotient bit is 1. What is left after row k fits in k + 1 bits, so the
//! rows start narrow: the first compares 1 bit and the last n. The bits of b above a row's width only have
//! to be 0, which is extra inputs on one gate.

use logic_sim::build::{Builder, Design, Netlist, S};
use logic_sim::families::family;
use logic_sim::parts::{inputs, normal_form, offer, subtract_if, Carry};

pub const SIZES: [usize; 5] = [2, 4, 8, 16, 32];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    let mut out = vec![Design::new("ripple", build(n, None))?, Design::new("lookahead", build(n, Some(Carry::Blocks(n))))?];
    for k in [4, 8] {
        if k < n {
            out.push(Design::new(format!("blocks-{k}"), build(n, Some(Carry::Blocks(k))))?);
        }
        if 2 * k <= n {
            out.push(Design::new(format!("two-level-{k}"), build(n, Some(Carry::TwoLevel(k))))?);
        }
    }
    if n <= 4 {
        let f = family("divider", &format!("{n}bit"))?;
        offer(&mut out, "normal-form", normal_form(2 * n, 2 * n, &*f.f));
    }
    Ok(out)
}

/// `carry` is how a row finds out whether it can subtract; None lets the borrow walk along the row.
fn build(n: usize, carry: Option<Carry>) -> Result<Netlist, String> {
    let mut b = Builder::new(2 * n);
    let (x, y) = (inputs(&b, 0, n), inputs(&b, n, n));
    let mut left: Vec<S> = vec![];
    let mut quotient = vec![S::Zero; n];
    for k in 0..n {
        let i = n - 1 - k;
        let mut row = vec![x[i]];
        row.extend_from_slice(&left);
        let (did, next) = subtract_if(&mut b, &row, &y[..=k], &y[k + 1..], carry);
        quotient[i] = did;
        left = next;
    }
    quotient.extend(left);
    b.finish(&quotient)
}
