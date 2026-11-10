//! Comparator generator: is a below, equal to, or above b.
//!
//! Inputs: a is bits 0 .. n-1, b is bits n .. 2n-1. Outputs: bit 0 is a < b, bit 1 is a = b, bit 2 is a > b.
//!
//! The highest bit where the two differ decides. So there is one term per bit: "the two differ here, b has
//! the 1, and every bit above is equal". A gate takes any number of inputs, so each term is a single AND gate
//! and the whole comparison is 4 ticks at any size.
//!
//! Walking the answer up bit by bit was tried too (3 gates per bit is the fewest, by exact search), and so
//! were the subtractor's borrow networks. Both take more gates and more ticks than `flat-nor`, so they are
//! not offered.

use logic_sim::build::{Builder, Design, Netlist, S};
use logic_sim::families::family;
use logic_sim::parts::{inputs, normal_form, offer};

pub const SIZES: [usize; 6] = [2, 4, 8, 16, 32, 64];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    let mut out = vec![Design::new("flat", flat(n, true))?, Design::new("flat-nor", flat(n, false))?];
    if n <= 4 {
        let f = family("comparator", &format!("{n}bit"))?;
        offer(&mut out, "normal-form", normal_form(2 * n, 3, &*f.f));
    }
    Ok(out)
}

/// With `both`, below and above each get their own terms: 4 ticks. Without, above is "neither below nor
/// equal", which is n gates fewer and one tick more.
fn flat(n: usize, both: bool) -> Result<Netlist, String> {
    let mut b = Builder::new(2 * n);
    let (x, y) = (inputs(&b, 0, n), inputs(&b, n, n));
    let differ: Vec<S> = (0..n).map(|i| b.xor(&[x[i], y[i]])).collect();
    let same: Vec<S> = (0..n).map(|i| b.xnor(&[x[i], y[i]])).collect();
    let decided = |b: &mut Builder, bits: &[S]| -> S {
        let terms: Vec<S> = (0..n)
            .map(|i| {
                let mut xs = vec![bits[i], differ[i]];
                xs.extend_from_slice(&same[i + 1..]);
                b.and(&xs)
            })
            .collect();
        b.or(&terms)
    };
    let below = decided(&mut b, &y);
    let equal = b.and(&same);
    let above = if both { decided(&mut b, &x) } else { b.nor(&[below, equal]) };
    b.finish(&[below, equal, above])
}
