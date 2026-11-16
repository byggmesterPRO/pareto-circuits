//! Square generator: x times x.
//!
//! Inputs: x, n bits. Outputs: the square, 2n bits.
//!
//! A multiplier with half the work. In the grid of products, bit i with bit j shows up twice (i with j and j
//! with i), and twice a bit is the same bit one column up. Bit i with itself is just bit i. So there is one
//! AND gate per pair instead of two, and the columns are added up the way the multiplier does it.

use logic_sim::build::{Builder, Design, Netlist};
use logic_sim::families::family;
use logic_sim::parts::{inputs, normal_form, offer, Carry};
use multiplier_generator::{sum_columns, sum_columns_with, with_adders, Bit};

pub const SIZES: [usize; 5] = [2, 4, 8, 16, 32];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    let adders = [("lookahead", Carry::Blocks(2 * n)), ("blocks-8", Carry::Blocks(8)), ("blocks-16", Carry::Blocks(16)), ("two-level-4", Carry::TwoLevel(4)), ("two-level-8", Carry::TwoLevel(8))];
    let mut out = vec![Design::new("ripple", build(n, 3, false, None))?, Design::new("parts-ripple", build(n, 3, true, None))?];
    for (name, carry) in adders {
        let fits = match carry {
            Carry::Blocks(k) => k <= 2 * n,
            Carry::TwoLevel(k) => 2 * k <= 2 * n,
        };
        if fits {
            out.push(Design::new(format!("tree-{name}"), build(n, 3, false, Some(carry)))?);
            out.push(Design::new(format!("parts-{name}"), build(n, 3, true, Some(carry)))?);
            if n >= 4 {
                offer(&mut out, format!("wide-{name}"), build(n, 7, false, Some(carry)));
            }
        }
    }
    if n <= 8 {
        let f = family("square", &format!("{n}bit"))?;
        offer(&mut out, "normal-form", normal_form(n, 2 * n, &*f.f));
    }
    // the adder search's designs for the last two rows
    out.extend(with_adders(&|most, parts, adder| {
        let mut b = Builder::new(n);
        let cols = grid(&mut b, n);
        sum_columns_with(b, cols, most, parts, adder)
    })?);
    Ok(out)
}

fn build(n: usize, most: usize, parts: bool, last: Option<Carry>) -> Result<Netlist, String> {
    let mut b = Builder::new(n);
    let cols = grid(&mut b, n);
    sum_columns(b, cols, most, parts, last)
}

/// The products of every pair of bits, as columns. A bit times itself is the bit, and a pair counts twice,
/// so it goes one column up.
fn grid(b: &mut Builder, n: usize) -> Vec<Vec<Bit>> {
    let x = inputs(b, 0, n);
    let mut cols: Vec<Vec<Bit>> = vec![vec![]; 2 * n];
    for i in 0..n {
        cols[2 * i].push(Bit::new(x[i], 1, None));
        for j in i + 1..n {
            cols[i + j + 1].push(Bit::new(b.and(&[x[i], x[j]]), 2, Some([x[i], x[j]])));
        }
    }
    cols
}
