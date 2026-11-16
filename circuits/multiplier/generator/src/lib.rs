//! Multiplier generator: the product of two n-bit numbers.
//!
//! Inputs: a is bits 0 .. n-1, b is bits n .. 2n-1. Outputs: the product, 2n bits.
//!
//! Every design starts from the n x n grid of products (bit i of a AND bit j of b, worth 2^(i+j)) and then
//! adds up each column. They differ in how the columns are added up.

use adder_generator::graft;
use logic_sim::build::{Builder, Design, Netlist, S};
use logic_sim::families::family;
use logic_sim::parts::{carries, full_adder, inputs, normal_form, offer, Carry};

pub const SIZES: [usize; 5] = [2, 4, 8, 16, 32];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    let wide_carry = [("lookahead", Carry::Blocks(2 * n)), ("blocks-8", Carry::Blocks(8)), ("blocks-16", Carry::Blocks(16)), ("two-level-4", Carry::TwoLevel(4)), ("two-level-8", Carry::TwoLevel(8))];
    let mut out = vec![Design::new("ripple", build(n, 3, false, None))?, Design::new("parts-ripple", build(n, 3, true, None))?];
    for (name, carry) in wide_carry {
        let fits = match carry {
            Carry::Blocks(k) => k <= 2 * n,
            Carry::TwoLevel(k) => 2 * k <= 2 * n,
        };
        if fits {
            out.push(Design::new(format!("tree-{name}"), build(n, 3, false, Some(carry)))?);
            out.push(Design::new(format!("parts-{name}"), build(n, 3, true, Some(carry)))?);
            if n >= 3 {
                offer(&mut out, format!("wide-{name}"), build(n, 7, false, Some(carry)));
            }
        }
    }
    if n <= 4 {
        let f = family("multiplier", &format!("{n}bit"))?;
        offer(&mut out, "normal-form", normal_form(2 * n, 2 * n, &*f.f));
    }
    out.extend(with_adders(&|most, parts, adder| build_with(n, most, parts, adder))?);
    Ok(out)
}

/// The same column trees, with the last two rows added by the adder family's searched designs. `make` builds
/// the circuit for a largest counter (3 or 7), with carries in three parts or not, and an adder, and says how
/// wide that adder has to be when it is given none. Only the results no other beats on both gates and ticks are kept. The square uses this too.
pub fn with_adders(make: &dyn Fn(usize, bool, Option<&Netlist>) -> Result<(usize, Option<Netlist>), String>) -> Result<Vec<Design>, String> {
    let mut all: Vec<(String, Netlist, (usize, usize))> = vec![];
    for (kind, most, parts) in [("tree", 3, false), ("parts", 3, true), ("wide", 7, false)] {
        let Ok((width, _)) = make(most, parts, None) else { continue };
        if width < 2 {
            continue;
        }
        for d in adder_generator::designs(width)? {
            let searched = ["pairs", "sum-carry", "clauses", "factors"].contains(&d.name.as_str()) || d.name.starts_with("within");
            if !searched || (most > 3 && d.name != "clauses" && d.name != "factors") {
                continue;
            }
            if let Ok((_, Some(net))) = make(most, parts, Some(&d.netlist)) {
                let size = (net.gates.len(), ticks(&net));
                all.push((format!("{kind}+{}", d.name), net, size));
            }
        }
    }
    let mut out = vec![];
    for (i, (name, net, (g, t))) in all.iter().enumerate() {
        if !all.iter().enumerate().any(|(k, (_, _, (g2, t2)))| g2 <= g && t2 <= t && ((g2, t2) != (g, t) || k < i)) {
            out.push(Design { name: name.clone(), netlist: net.clone() });
        }
    }
    Ok(out)
}

fn ticks(net: &Netlist) -> usize {
    let mut d = vec![0usize; net.inputs];
    for (_, ins) in &net.gates {
        d.push(1 + ins.iter().map(|&i| d[i as usize]).max().unwrap_or(0));
    }
    net.outputs.iter().flatten().map(|&o| d[o as usize]).max().unwrap_or(0)
}

/// A bit in a column: the signal, the tick it is ready at, and for a product straight off the grid the two
/// input bits it is the AND of.
///
/// A carry is not one signal but three, `s` and the two in `rest`, whose XOR is the carry (see `add`). An
/// adder that reads it takes all three into its XOR gate, which saves the gate that would join them.
#[derive(Clone)]
pub struct Bit {
    pub s: S,
    pub at: u32,
    pub of: Option<[S; 2]>,
    pub rest: Vec<S>,
}

impl Bit {
    pub fn new(s: S, at: u32, of: Option<[S; 2]>) -> Bit {
        Bit { s, at, of, rest: vec![] }
    }

    fn whole(&self) -> bool {
        self.rest.is_empty()
    }

    fn terms(&self) -> Vec<S> {
        [&[self.s], self.rest.as_slice()].concat()
    }

    /// The bit as one signal: for a carry in three parts, the XOR gate that joins them.
    fn joined(&self, b: &mut Builder) -> Bit {
        if self.whole() {
            return self.clone();
        }
        Bit::new(b.xor(&self.terms()), self.at + 1, None)
    }
}

/// Add 2 or 3 bits of a column: the sum stays, the carry goes one column up.
///
/// With sum = x xor y xor z, the carry is sum xor or(x, y, sum) xor and(x, y, sum). It is handed on as those
/// three parts and never joined, so a full adder is 3 gates where the textbook one is 4. Only x and y have to
/// be whole signals; z may itself be a carry in three parts.
fn add(b: &mut Builder, xs: &[Bit]) -> Vec<Bit> {
    let mut xs: Vec<Bit> = xs.to_vec();
    xs.sort_by_key(|x| !x.whole());
    let (x, y) = (xs[0].joined(b), xs[1].joined(b));
    if xs.len() == 2 {
        let at = x.at.max(y.at) + 1;
        return vec![Bit::new(b.xor(&[x.s, y.s]), at, None), Bit::new(b.and(&[x.s, y.s]), at, None)];
    }
    let at = x.at.max(y.at).max(xs[2].at) + 1;
    let sum = b.xor(&[&[x.s, y.s], xs[2].terms().as_slice()].concat());
    let rest = vec![b.or(&[x.s, y.s, sum]), b.and(&[x.s, y.s, sum])];
    vec![Bit::new(sum, at, None), Bit { s: sum, at: at + 1, of: None, rest }]
}

/// Take m bits to add from a column sorted by when they are ready: the two earliest whole ones and, for a
/// third, the earliest carry in parts if there is one. So that as few carries as possible have to be joined.
fn take(left: &mut Vec<Bit>, m: usize, parts: bool) -> Vec<Bit> {
    let whole: Vec<usize> = (0..left.len()).filter(|&i| left[i].whole()).take(2).collect();
    if !parts || m > 3 || whole.len() < 2 {
        return left.drain(..m).collect();
    }
    let mut picks = whole;
    if m == 3 {
        let third = (0..left.len()).find(|&i| !left[i].whole()).or_else(|| (0..left.len()).find(|i| !picks.contains(i)));
        picks.extend(third);
    }
    picks.sort_unstable();
    picks.iter().rev().map(|&i| left.remove(i)).collect()
}

/// The XOR of the AND of every group of `size` of these bits. For the bits of a column this is one bit of
/// how many of them are on: groups of 2 give the 2s bit of the count, groups of 4 the 4s bit. Products off
/// the grid go into the AND gates as their two input bits, which skips a gate on the way.
fn groups(b: &mut Builder, xs: &[Bit], size: usize) -> Bit {
    let mut terms = vec![];
    let xs: Vec<Bit> = xs.iter().map(|x| x.joined(b)).collect();
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
    Bit::new(b.xor(&terms), ready + if terms.len() > 1 { 2 } else { 1 }, None)
}

/// Count m bits of a column: the 1s bit stays in the column, the 2s bit goes one column up, the 4s bit two.
/// Up to 3 bits it is a full adder (4 gates, or 3 with the carry in `parts`) or a half adder. Above that it is a counter of up to 7 bits in
/// and 3 out, 2 gates deep whatever its size and far more gates.
fn count(b: &mut Builder, xs: &[Bit], wide: bool, parts: bool) -> Vec<Bit> {
    if !wide && parts {
        return add(b, xs);
    }
    let xs: Vec<Bit> = xs.iter().map(|x| x.joined(b)).collect();
    let xs = xs.as_slice();
    let ready = xs.iter().map(|x| x.at).max().unwrap_or(0);
    let signals: Vec<S> = xs.iter().map(|x| x.s).collect();
    if !wide {
        let (sum, carry) = full_adder(b, signals[0], signals[1], signals.get(2).copied().unwrap_or(S::Zero));
        return vec![Bit::new(sum, ready + 1, None), Bit::new(carry, ready + if xs.len() == 3 { 2 } else { 1 }, None)];
    }
    let mut out = vec![Bit::new(b.xor(&signals), ready + 1, None), groups(b, xs, 2)];
    if xs.len() >= 4 {
        out.push(groups(b, xs, 4));
    }
    out
}

/// Add columns up layer by layer until none has more than 2 bits. Each layer brings every column down to a
/// height the next layer can finish from, and counts no more bits than that takes. `most` is the largest
/// counter: 3 is full adders only.
fn layers(b: &mut Builder, mut cols: Vec<Vec<Bit>>, most: usize, parts: bool) -> Result<Vec<Vec<Bit>>, String> {
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
                let taken = take(&mut left, m, parts);
                for (up, bit) in count(b, &taken, most > 3, parts).into_iter().enumerate() {
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
fn walk(b: &mut Builder, cols: &mut [Vec<Bit>], parts: bool) {
    for c in 0..cols.len() {
        while cols[c].len() > 1 {
            cols[c].sort_by_key(|x| x.at);
            let m = cols[c].len().min(3);
            let taken = take(&mut cols[c], m, parts);
            for (up, bit) in count(b, &taken, false, parts).into_iter().enumerate() {
                if c + up < cols.len() && bit.s != S::Zero {
                    cols[c + up].push(bit);
                }
            }
        }
    }
}

/// The n x n grid of products, as columns.
fn grid(b: &mut Builder, n: usize) -> Vec<Vec<Bit>> {
    let (x, y) = (inputs(b, 0, n), inputs(b, n, n));
    let mut cols: Vec<Vec<Bit>> = vec![vec![]; 2 * n];
    for i in 0..n {
        for j in 0..n {
            cols[i + j].push(Bit::new(b.and(&[x[i], y[j]]), 2, Some([x[i], y[j]])));
        }
    }
    cols
}

/// `last` is how the final two rows are added; None is `walk`.
fn build(n: usize, most: usize, parts: bool, last: Option<Carry>) -> Result<Netlist, String> {
    let mut b = Builder::new(2 * n);
    let cols = grid(&mut b, n);
    sum_columns(b, cols, most, parts, last)
}

fn build_with(n: usize, most: usize, parts: bool, adder: Option<&Netlist>) -> Result<(usize, Option<Netlist>), String> {
    let mut b = Builder::new(2 * n);
    let cols = grid(&mut b, n);
    sum_columns_with(b, cols, most, parts, adder)
}

/// Like `sum_columns`, with the final two rows added by a given adder (see `with_adders`). Also gives how
/// many columns the adder has to cover: the ones from the lowest that is left with two bits.
pub fn sum_columns_with(mut b: Builder, cols: Vec<Vec<Bit>>, most: usize, parts: bool, adder: Option<&Netlist>) -> Result<(usize, Option<Netlist>), String> {
    let cols = layers(&mut b, cols, most, parts)?;
    let low = cols.iter().position(|c| c.len() > 1).unwrap_or(cols.len());
    let width = cols.len() - low;
    let Some(adder) = adder else { return Ok((width, None)) };
    let mut row = |k: usize| -> Vec<S> { cols[low..].iter().map(|c| c.get(k).map_or(S::Zero, |x| x.joined(&mut b).s)).collect() };
    let (x, y) = (row(0), row(1));
    let mut outs: Vec<S> = cols[..low].iter().map(|c| c.first().map_or(S::Zero, |x| x.joined(&mut b).s)).collect();
    outs.extend(graft(&mut b, adder, &x, &y).into_iter().take(width));
    Ok((width, Some(b.finish(&outs)?)))
}

/// Add up columns of bits, where column c is worth 2^c: one output bit per column. `most` is the largest
/// counter (3 or 7), `parts` hands carries on in three parts (see `add`), `last` is how the final two rows are added; None is `walk`. The square uses this too.
pub fn sum_columns(mut b: Builder, mut cols: Vec<Vec<Bit>>, most: usize, parts: bool, last: Option<Carry>) -> Result<Netlist, String> {
    let Some(carry) = last else {
        walk(&mut b, &mut cols, parts);
        let outs: Vec<S> = cols.iter().map(|c| c.first().map_or(S::Zero, |x| x.joined(&mut b).s)).collect();
        return b.finish(&outs);
    };
    let cols = layers(&mut b, cols, most, parts)?;
    let cols: Vec<Vec<Bit>> = cols.iter().map(|c| c.iter().map(|x| x.joined(&mut b)).collect()).collect();
    let bit = |c: &Vec<Bit>, k: usize| c.get(k).map_or(S::Zero, |x| x.s);
    let pass: Vec<S> = cols.iter().map(|c| b.xor(&[bit(c, 0), bit(c, 1)])).collect();
    let starts: Vec<Vec<S>> = cols.iter().map(|c| vec![bit(c, 0), bit(c, 1)]).collect();
    let terms = carries(&mut b, &starts, &pass, carry);
    let outs: Vec<S> = (0..cols.len())
        .map(|c| {
            let mut xs = terms[c].clone();
            xs.push(pass[c]);
            b.xor(&xs)
        })
        .collect();
    b.finish(&outs)
}
