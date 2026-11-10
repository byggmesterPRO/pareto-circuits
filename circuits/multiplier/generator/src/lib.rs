//! Multiplier generator: the product of two n-bit numbers.
//!
//! Inputs: a is bits 0 .. n-1, b is bits n .. 2n-1. Outputs: the product, 2n bits.
//!
//! Every design starts from the n x n grid of products (bit i of a AND bit j of b, worth 2^(i+j)) and then
//! adds up each column. They differ in how the columns are added up.

use logic_sim::build::{Builder, Design, Netlist, S};
use logic_sim::families::family;
use logic_sim::parts::{carries, full_adder, inputs, normal_form, offer, Carry};

pub const SIZES: [usize; 5] = [2, 4, 8, 16, 32];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    let wide_carry = [("lookahead", Carry::Blocks(2 * n)), ("blocks-8", Carry::Blocks(8)), ("blocks-16", Carry::Blocks(16)), ("two-level-4", Carry::TwoLevel(4)), ("two-level-8", Carry::TwoLevel(8))];
    let mut out = vec![Design::new("ripple", build(n, 3, None))?];
    for (name, carry) in wide_carry {
        let fits = match carry {
            Carry::Blocks(k) => k <= 2 * n,
            Carry::TwoLevel(k) => 2 * k <= 2 * n,
        };
        if fits {
            out.push(Design::new(format!("tree-{name}"), build(n, 3, Some(carry)))?);
            if n >= 3 {
                offer(&mut out, format!("wide-{name}"), build(n, 7, Some(carry)));
            }
        }
    }
    if n <= 4 {
        let f = family("multiplier", &format!("{n}bit"))?;
        offer(&mut out, "normal-form", normal_form(2 * n, 2 * n, &*f.f));
    }
    Ok(out)
}

/// A bit in a column: the signal, the tick it is ready at, and for a product straight off the grid the two
/// input bits it is the AND of.
#[derive(Clone)]
struct Bit {
    s: S,
    at: u32,
    of: Option<[S; 2]>,
}

/// The XOR of the AND of every group of `size` of these bits. For the bits of a column this is one bit of
/// how many of them are on: groups of 2 give the 2s bit of the count, groups of 4 the 4s bit. Products off
/// the grid go into the AND gates as their two input bits, which skips a gate on the way.
fn groups(b: &mut Builder, xs: &[Bit], size: usize) -> Bit {
    let mut terms = vec![];
    let mut pick: Vec<usize> = (0..size).collect();
    loop {
        let mut chosen = vec![];
        for &i in &pick {
            match xs[i].of {
                Some(pair) => chosen.extend(pair),
                None => chosen.push(xs[i].s),
            }
        }
        terms.push(b.and(&chosen));
        // the next group of `size` out of xs.len()
        let Some(i) = (0..size).rev().find(|&i| pick[i] < xs.len() - size + i) else { break };
        pick[i] += 1;
        for j in i + 1..size {
            pick[j] = pick[j - 1] + 1;
        }
    }
    let ready = xs.iter().map(|x| if x.of.is_some() { 1 } else { x.at }).max().unwrap_or(0);
    Bit { s: b.xor(&terms), at: ready + if terms.len() > 1 { 2 } else { 1 }, of: None }
}

/// Count m bits of a column: the 1s bit stays in the column, the 2s bit goes one column up, the 4s bit two.
/// Up to 3 bits it is a full adder (4 gates) or a half adder. Above that it is a counter of up to 7 bits in
/// and 3 out, 2 gates deep whatever its size and far more gates.
fn count(b: &mut Builder, xs: &[Bit], wide: bool) -> Vec<Bit> {
    let ready = xs.iter().map(|x| x.at).max().unwrap_or(0);
    let signals: Vec<S> = xs.iter().map(|x| x.s).collect();
    if !wide {
        let (sum, carry) = full_adder(b, signals[0], signals[1], signals.get(2).copied().unwrap_or(S::Zero));
        return vec![Bit { s: sum, at: ready + 1, of: None }, Bit { s: carry, at: ready + if xs.len() == 3 { 2 } else { 1 }, of: None }];
    }
    let mut out = vec![Bit { s: b.xor(&signals), at: ready + 1, of: None }, groups(b, xs, 2)];
    if xs.len() >= 4 {
        out.push(groups(b, xs, 4));
    }
    out
}

/// Add columns up layer by layer until none has more than 2 bits. Each layer brings every column down to a
/// height the next layer can finish from, and counts no more bits than that takes. `most` is the largest
/// counter: 3 is full adders only.
fn layers(b: &mut Builder, mut cols: Vec<Vec<Bit>>, most: usize) -> Result<Vec<Vec<Bit>>, String> {
    for _ in 0..64 {
        let height = cols.iter().map(|c| c.len()).max().unwrap_or(0);
        if height <= 2 {
            return Ok(cols);
        }
        // The heights that 1, 2, 3 .. layers can bring down to 2. A column that runs q full counters ends up
        // with one bit from each, one from each counter in the column below (two columns below as well, for
        // the wide ones), and the bits it left alone.
        let outs = if most > 3 { 3 } else { 2 };
        let (mut target, mut reach) = (2, 3);
        while reach < height {
            target = reach;
            reach = most * (target / outs) + target % outs;
        }
        let mut next: Vec<Vec<Bit>> = vec![vec![]; cols.len()];
        for c in 0..cols.len() {
            let mut left = std::mem::take(&mut cols[c]);
            left.sort_by_key(|x| x.at);
            while next[c].len() + left.len() > target {
                let m = (next[c].len() + left.len() - target + 1).min(most).min(left.len());
                if m < 2 {
                    break;
                }
                let taken: Vec<Bit> = left.drain(..m).collect();
                for (up, bit) in count(b, &taken, most > 3).into_iter().enumerate() {
                    if c + up < next.len() && bit.s != S::Zero {
                        next[c + up].push(bit);
                    }
                }
            }
            next[c].extend(left);
        }
        cols = next;
    }
    Err("the columns do not come down to two rows".into())
}

/// Add every column all the way down to one bit, lowest column first, with full adders. The carries walk,
/// and it takes the fewest gates.
fn walk(b: &mut Builder, cols: &mut [Vec<Bit>]) {
    for c in 0..cols.len() {
        while cols[c].len() > 1 {
            cols[c].sort_by_key(|x| x.at);
            let m = cols[c].len().min(3);
            let taken: Vec<Bit> = cols[c].drain(..m).collect();
            for (up, bit) in count(b, &taken, false).into_iter().enumerate() {
                if c + up < cols.len() && bit.s != S::Zero {
                    cols[c + up].push(bit);
                }
            }
        }
    }
}

/// `last` is how the final two rows are added; None is `walk`.
fn build(n: usize, most: usize, last: Option<Carry>) -> Result<Netlist, String> {
    let mut b = Builder::new(2 * n);
    let (x, y) = (inputs(&b, 0, n), inputs(&b, n, n));
    let mut cols: Vec<Vec<Bit>> = vec![vec![]; 2 * n];
    for i in 0..n {
        for j in 0..n {
            cols[i + j].push(Bit { s: b.and(&[x[i], y[j]]), at: 2, of: Some([x[i], y[j]]) });
        }
    }
    let bit = |c: &Vec<Bit>, k: usize| c.get(k).map_or(S::Zero, |x| x.s);
    let Some(carry) = last else {
        walk(&mut b, &mut cols);
        let outs: Vec<S> = cols.iter().map(|c| bit(c, 0)).collect();
        return b.finish(&outs);
    };
    let cols = layers(&mut b, cols, most)?;
    let pass: Vec<S> = cols.iter().map(|c| b.xor(&[bit(c, 0), bit(c, 1)])).collect();
    let starts: Vec<Vec<S>> = cols.iter().map(|c| vec![bit(c, 0), bit(c, 1)]).collect();
    let terms = carries(&mut b, &starts, &pass, carry);
    let outs: Vec<S> = (0..2 * n)
        .map(|c| {
            let mut xs = terms[c].clone();
            xs.push(pass[c]);
            b.xor(&xs)
        })
        .collect();
    b.finish(&outs)
}
