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

/// Every design for this size. The tool that makes builds measures them and picks the winners.
pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    let mut out = vec![Design::new("ripple", ripple(n))?, Design::new("lookahead", blocks(n, n))?];
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
