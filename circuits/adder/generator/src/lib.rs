//! Adder generator: several ways to add two numbers of n bits, at any size.
//!
//! Inputs: a is bits 0 .. n-1, b is bits n .. 2n-1. Outputs: the sum in bits 0 .. n-1, the carry out in bit n.
//!
//! All designs but the ripple adder rest on one trick. For every bit, p = a xor b ("passes a carry on") and
//! g = a and b ("starts a carry"). A carry reaches bit i from exactly one place: the nearest bit below that
//! starts one, with every bit in between passing it on. So the terms "g at j, p at everything between j and i"
//! are never on two at a time, and their OR is the same as their XOR. A sum bit is then one XOR gate over
//! p and all its carry terms, with no separate gate for the carry.

use logic_sim::build::{Builder, Design, Netlist, S};

mod factors;
mod minus;
mod net;
mod sum_carry;

pub const SIZES: [usize; 6] = [2, 4, 8, 16, 32, 64];

/// The tick limits the `within-T` designs are planned for.
const LIMITS: [usize; 10] = [5, 6, 8, 10, 12, 16, 24, 32, 48, 64];

/// Every design for this size. The tool that makes builds measures them and picks the winners.
pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    let mut out = vec![Design::new("ripple", ripple(n))?, Design::new("pairs", pairs(n))?, Design::new("lookahead", blocks(n, n))?];
    for k in [2, 3, 4, 6, 8, 12, 16, 24, 32] {
        if k < n {
            out.push(Design::new(format!("blocks-{k}"), blocks(n, k))?);
        }
    }
    for k in [2, 4, 8, 16] {
        if 2 * k <= n {
            out.push(Design::new(format!("two-level-{k}"), two_level(n, k))?);
        }
    }
    out.extend(new_designs(n)?);
    Ok(out)
}

/// The designs from the search (tools/evolve/tasks/adder/runs): `sum-carry`, `clauses`, `factors` and
/// `within-T`. A design that comes out the same in gates and ticks as one offered before is left out.
fn new_designs(n: usize) -> Result<Vec<Design>, String> {
    let size = |net: &Netlist| (net.gates.len(), net::ticks(net));
    let (sc, outs) = sum_carry::ripple(n);
    let mut out = vec![Design::new("sum-carry", sc.finish(&outs))?];
    // the fast ones, kept to offer them in the within-T designs too
    let mut quick: Vec<Netlist> = vec![];
    for (name, t) in [("clauses", 3), ("factors", 4)] {
        if let Some(net) = factors::fast(n, t) {
            if !quick.iter().any(|q| size(q) == size(&net)) {
                quick.push(net.clone());
                out.push(Design { name: name.into(), netlist: net });
            }
        }
    }
    let mut seen: Vec<(usize, usize)> = out.iter().map(|d| size(&d.netlist)).collect();
    let mut best: Option<Netlist> = None; // the smallest so far that fits the limit
    for t in LIMITS.into_iter().filter(|&t| t < 2 * n) {
        let mut cands: Vec<Netlist> = quick.iter().filter(|q| net::ticks(q) <= t).cloned().collect();
        if let Some((net, outs)) = sum_carry::within(n, t, false) {
            cands.push(net.finish(&outs).map_err(|e| format!("within-{t}: {e}"))?);
        }
        cands.extend(best.take());
        best = cands.into_iter().min_by_key(|c| size(c));
        if let Some(b) = &best {
            if !seen.contains(&size(b)) {
                seen.push(size(b));
                out.push(Design { name: format!("within-{t}"), netlist: b.clone() });
            }
        }
    }
    Ok(out)
}

fn start(n: usize) -> (Builder, Vec<S>, Vec<S>) {
    let b = Builder::new(2 * n);
    let (x, y) = ((0..n).map(|i| b.input(i)).collect(), (0..n).map(|i| b.input(n + i)).collect());
    (b, x, y)
}

/// One full adder per bit, the carry rippling up. Smallest, and slowest: 2 ticks per bit.
/// The full adder is 4 gates, which exact synthesis (SAT) showed is the fewest: sum = xor(a, b, c), and the
/// carry is xor(sum, and(a, b, c), or(a, b, c)).
fn ripple(n: usize) -> Result<Netlist, String> {
    let (mut b, x, y) = start(n);
    let (mut carry, mut outs) = (S::Zero, vec![]);
    for i in 0..n {
        let ins = [x[i], y[i], carry];
        let sum = b.xor(&ins);
        carry = if i == 0 {
            b.and(&ins[..2])
        } else {
            let (all, any) = (b.and(&ins), b.or(&ins));
            b.xor(&[sum, all, any])
        };
        outs.push(sum);
    }
    outs.push(carry);
    b.finish(&outs)
}

/// Two bits at a time, the carry walking from pair to pair. A pair with carry in and carry out is 7 gates,
/// which exact search (SAT) showed is the fewest: 6 is impossible. That is 3.5 gates per bit where the full
/// adder takes 4. The seven gates have no neat names; they are the search's answer as it came out.
fn pairs(n: usize) -> Result<Netlist, String> {
    let (mut b, x, y) = start(n);
    let (mut carry, mut outs) = (S::Zero, vec![]);
    let mut i = 0;
    while i + 1 < n {
        let (a0, a1, b0, b1, c) = (x[i], x[i + 1], y[i], y[i + 1], carry);
        let low = [a0, b0, c];
        let (any, all) = (b.or(&low), b.and(&low));
        let high = b.xor(&[a0, a1, b0, b1, c, any, all]);
        let g8 = b.or(&[a1, b1, high]);
        let g9 = b.nand(&[a1, b1, any, high]);
        let out = b.xnor(&[a0, a1, b0, b1, c, any, all, g8, g9]);
        let sum0 = b.xnor(&[a0, b0, c, high, g8, g9, out]);
        outs.extend([sum0, high]);
        carry = out;
        i += 2;
    }
    if i < n {
        // an odd bit at the top gets the 4-gate full adder
        let ins = [x[i], y[i], carry];
        let sum = b.xor(&ins);
        let (all, any) = (b.and(&ins), b.or(&ins));
        carry = b.xor(&[sum, all, any]);
        outs.push(sum);
    }
    outs.push(carry);
    b.finish(&outs)
}

fn and_with(b: &mut Builder, first: S, rest: &[S]) -> S {
    let mut xs = vec![first];
    xs.extend_from_slice(rest);
    b.and(&xs)
}

/// Blocks of k bits. Inside a block every sum bit sees its carry in one step (see the top of the file); the
/// carry out of a block ripples into the next. k = n is one block: 4 ticks at any size, and about n*n/2 gates.
fn blocks(n: usize, k: usize) -> Result<Netlist, String> {
    let (mut b, x, y) = start(n);
    let p: Vec<S> = (0..n).map(|i| b.xor(&[x[i], y[i]])).collect();
    let g: Vec<S> = (0..n).map(|i| b.and(&[x[i], y[i]])).collect();
    let (mut carry_in, mut outs) = (S::Zero, vec![]);
    for s in (0..n).step_by(k) {
        let e = (s + k).min(n);
        // the carry into bit i (or out of the block, for i = e), as terms of which at most one is on
        let carry_terms = |b: &mut Builder, i: usize| {
            let mut terms = vec![and_with(b, carry_in, &p[s..i])];
            for j in s..i {
                terms.push(and_with(b, g[j], &p[j + 1..i]));
            }
            terms
        };
        for i in s..e {
            let mut xs = carry_terms(&mut b, i);
            xs.push(p[i]);
            outs.push(b.xor(&xs));
        }
        let terms = carry_terms(&mut b, e);
        carry_in = b.xor(&terms);
    }
    outs.push(carry_in);
    b.finish(&outs)
}

/// Blocks of k bits with a second level on top: each block says whether it starts a carry (G) and whether it
/// passes one on (P), and a sum bit takes its carry from the blocks below in one step. 6 ticks at any size,
/// with far fewer gates than one big block.
fn two_level(n: usize, k: usize) -> Result<Netlist, String> {
    let (mut b, x, y) = start(n);
    let p: Vec<S> = (0..n).map(|i| b.xor(&[x[i], y[i]])).collect();
    let g: Vec<S> = (0..n).map(|i| b.and(&[x[i], y[i]])).collect();
    let ranges: Vec<(usize, usize)> = (0..n).step_by(k).map(|s| (s, (s + k).min(n))).collect();
    let mut big_p = vec![];
    let mut big_g = vec![];
    for &(s, e) in &ranges {
        big_p.push(b.and(&p[s..e]));
        let terms: Vec<S> = (s..e).map(|j| and_with(&mut b, g[j], &p[j + 1..e])).collect();
        big_g.push(b.xor(&terms));
    }
    // a carry that block `from` starts, passed on by every block up to block m and by the bits s..i of block m
    let from_below = |b: &mut Builder, m: usize, bits: &[S]| -> Vec<S> {
        (0..m)
            .map(|from| {
                let mut xs = vec![big_g[from]];
                xs.extend_from_slice(&big_p[from + 1..m]);
                xs.extend_from_slice(bits);
                b.and(&xs)
            })
            .collect()
    };
    let mut outs = vec![];
    for (m, &(s, e)) in ranges.iter().enumerate() {
        for i in s..e {
            let mut xs = from_below(&mut b, m, &p[s..i]);
            for j in s..i {
                xs.push(and_with(&mut b, g[j], &p[j + 1..i]));
            }
            xs.push(p[i]);
            outs.push(b.xor(&xs));
        }
    }
    let terms = from_below(&mut b, ranges.len(), &[]);
    outs.push(b.xor(&terms));
    b.finish(&outs)
}

/// The search's designs turned into subtractors, a - b, for the subtractor family: `clauses`, `factors` and
/// `within-T`, where T is the ticks the subtractor really takes. Only the ones no other beats are given.
pub fn minus_designs(n: usize) -> Result<Vec<Design>, String> {
    let mut all: Vec<(String, Netlist)> = vec![];
    for (name, t) in [("clauses", 3), ("factors", 4)] {
        if let Some(net) = factors::fast(n, t) {
            let outs: Vec<u32> = net.outputs.iter().flatten().copied().collect();
            all.push((name.into(), minus::minus(n, net.inputs, &net.gates, &outs)?));
        }
    }
    for t in [3, 4].into_iter().chain(LIMITS).filter(|&t| t <= 2 * n) {
        if let Some((net, outs)) = sum_carry::within(n, t, true) {
            all.push(("within".into(), minus::minus(n, net.inputs(), &net.gates, &outs)?));
        }
    }
    Ok(unbeaten(all))
}

/// The designs that no other in the list beats on both gates and ticks. A `within` design is named by the
/// ticks it really takes.
fn unbeaten(all: Vec<(String, Netlist)>) -> Vec<Design> {
    let size = |net: &Netlist| (net.gates.len(), net::ticks(net));
    let mut out: Vec<Design> = vec![];
    for (i, (name, net)) in all.iter().enumerate() {
        let (g, t) = size(net);
        let beaten = all.iter().enumerate().any(|(k, (_, o))| {
            let (g2, t2) = size(o);
            (g2 <= g && t2 <= t) && ((g2, t2) != (g, t) || k < i)
        });
        if !beaten {
            let name = if name.starts_with("within") { format!("within-{t}") } else { name.clone() };
            out.push(Design { name, netlist: net.clone() });
        }
    }
    out
}

/// The search's designs with a mode pin, for the adder-subtractor family: a + b, or a - b when the mode is on.
///
/// a - b is a + "not b" + 1. A carry in comes for free from an adder one bit longer: with the mode as both
/// inputs of its lowest bit, that bit adds nothing and carries exactly when the mode is on. b goes through
/// one XOR with the mode per bit, and an XOR gate that reads it takes b and the mode themselves instead.
/// The last output is the carry of an addition or the borrow of a subtraction: the carry xor the mode.
pub fn mode_designs(n: usize) -> Result<Vec<Design>, String> {
    let mut all = vec![];
    for d in designs(n + 1)? {
        if ["pairs", "sum-carry", "clauses", "factors"].contains(&d.name.as_str()) || d.name.starts_with("within") {
            all.push((d.name, with_mode(n, &d.netlist)?));
        }
    }
    Ok(unbeaten(all))
}

fn with_mode(n: usize, adder: &Netlist) -> Result<Netlist, String> {
    let wide = n + 1;
    let mut b = Builder::new(2 * n + 1);
    let mode = b.input(2 * n);
    let mut made: Vec<(u8, Vec<S>)> = vec![];
    let mut sig: Vec<S> = vec![];
    for (m, ins) in &adder.gates {
        let mut xs: Vec<S> = vec![];
        for &i in ins {
            let i = i as usize;
            if i == 0 || i == wide {
                xs.push(mode);
            } else if i < wide {
                xs.push(b.input(i - 1));
            } else if i < 2 * wide {
                let bit = b.input(n + i - wide - 1);
                if m % 3 == logic_sim::build::XOR {
                    xs.extend([bit, mode]);
                } else {
                    xs.push(b.xor(&[bit, mode]));
                }
            } else {
                xs.push(sig[i - 2 * wide]);
            }
        }
        sig.push(b.gate(*m, &xs));
        made.push((*m, xs));
    }
    let gate = |o: Option<u32>| (o.ok_or("an output that is always off")? as usize).checked_sub(2 * wide).ok_or("an output that is an input".to_string());
    let mut outs = vec![];
    for k in 1..wide {
        outs.push(sig[gate(adder.outputs[k])?]);
    }
    let top = gate(adder.outputs[wide])?;
    let (m, xs) = &made[top];
    outs.push(if m % 3 == logic_sim::build::XOR { b.gate(*m, &[xs.as_slice(), &[mode]].concat()) } else { b.xor(&[sig[top], mode]) });
    b.finish(&outs)
}
