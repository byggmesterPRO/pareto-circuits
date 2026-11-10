//! Square root generator: the square root of an n-bit number, rounded down.
//!
//! Inputs: x, n bits (n is even). Outputs: the root, n / 2 bits.
//!
//! The pen and paper method in binary, one row per root bit from the top: bring down the next two bits of x,
//! and if what is left is at least (root so far, then 01), take that away and the root bit is 1. A row is the
//! same "subtract if you can" row as in the divider.

use logic_sim::build::{Builder, Design, Netlist, S};
use logic_sim::families::family;
use logic_sim::parts::{inputs, normal_form, offer, subtract_if, Carry};

pub const SIZES: [usize; 6] = [2, 4, 8, 16, 32, 64];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    if n % 2 == 1 {
        return Err(format!("square-root: {n} bits is not an even number"));
    }
    let mut out = vec![Design::new("ripple", build(n, None))?, Design::new("lookahead", build(n, Some(Carry::Blocks(n))))?];
    for k in [4, 8] {
        if k < n / 2 {
            out.push(Design::new(format!("blocks-{k}"), build(n, Some(Carry::Blocks(k))))?);
        }
        if 2 * k <= n / 2 {
            out.push(Design::new(format!("two-level-{k}"), build(n, Some(Carry::TwoLevel(k))))?);
        }
    }
    if n <= 8 {
        let f = family("square-root", &format!("{n}bit"))?;
        offer(&mut out, "normal-form", normal_form(n, n / 2, &*f.f));
    }
    Ok(out)
}

fn build(n: usize, carry: Option<Carry>) -> Result<Netlist, String> {
    let mut b = Builder::new(n);
    let x = inputs(&b, 0, n);
    let mut left: Vec<S> = vec![];
    // the root so far, lowest bit first
    let mut root: Vec<S> = vec![];
    for k in 0..n / 2 {
        let mut row = vec![x[n - 2 - 2 * k], x[n - 1 - 2 * k]];
        row.extend_from_slice(&left);
        let mut take = vec![S::One, S::Zero];
        take.extend_from_slice(&root);
        let (did, mut next) = subtract_if(&mut b, &row, &take, &[], carry);
        // what is left is at most twice the root, which now has k + 1 bits
        next.truncate(k + 2);
        left = next;
        root.insert(0, did);
    }
    b.finish(&root)
}
