//! Pieces that several generators share.

use crate::build::{Builder, Design, Netlist, S};

/// Input bits from..from+n as signals.
pub fn inputs(b: &Builder, from: usize, n: usize) -> Vec<S> {
    (from..from + n).map(|i| b.input(i)).collect()
}

/// Add a design to the list if it can be built; one that cannot (a gate with more than 255 inputs, say) is
/// left out.
pub fn offer(out: &mut Vec<Design>, name: impl Into<String>, netlist: Result<Netlist, String>) {
    if let Ok(netlist) = netlist {
        out.push(Design { name: name.into(), netlist });
    }
}

fn and_of(b: &mut Builder, first: &[S], rest: &[S]) -> S {
    let mut xs = first.to_vec();
    xs.extend_from_slice(rest);
    b.and(&xs)
}

/// How a carry gets from bit to bit.
#[derive(Clone, Copy)]
pub enum Carry {
    /// Blocks of k bits: inside a block every bit sees its carry in one step, and the carry out of a block
    /// walks into the next. One block as wide as the number is 3 gates deep at any size.
    Blocks(usize),
    /// Blocks of k bits with a second level across the blocks: 5 gates deep at any size.
    TwoLevel(usize),
}

/// The carry into every bit, and out of the top one (n + 1 entries for n bits), each as a list of terms of
/// which at most one is on at a time. So the carry is the OR of its terms, and also their XOR: a sum bit can
/// be one XOR gate over its own terms.
///
/// Bit j starts a carry when all of `generate[j]` are on, and passes one on when `pass[j]` is on. The two
/// must never be on together. For an adder that is generate = [a, b], pass = a xor b.
pub fn carries(b: &mut Builder, generate: &[Vec<S>], pass: &[S], carry: Carry) -> Vec<Vec<S>> {
    let n = pass.len();
    let mut out = vec![];
    match carry {
        Carry::Blocks(k) => {
            let mut carry_in = S::Zero;
            for s in (0..n).step_by(k.max(1)) {
                let e = (s + k.max(1)).min(n);
                for i in s..=e {
                    let mut terms = vec![and_of(b, &[carry_in], &pass[s..i])];
                    for j in s..i {
                        terms.push(and_of(b, &generate[j], &pass[j + 1..i]));
                    }
                    if i < e {
                        out.push(terms);
                    } else {
                        carry_in = b.xor(&terms);
                    }
                }
            }
            out.push(vec![carry_in]);
        }
        Carry::TwoLevel(k) => {
            let ranges: Vec<(usize, usize)> = (0..n).step_by(k.max(1)).map(|s| (s, (s + k.max(1)).min(n))).collect();
            let (mut big_p, mut big_g) = (vec![], vec![]);
            for &(s, e) in &ranges {
                big_p.push(b.and(&pass[s..e]));
                let terms: Vec<S> = (s..e).map(|j| and_of(b, &generate[j], &pass[j + 1..e])).collect();
                big_g.push(b.xor(&terms));
            }
            // a carry that block `from` starts, passed on by every block up to block m and by `bits` of block m
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
            for (m, &(s, e)) in ranges.iter().enumerate() {
                for i in s..e {
                    let mut terms = from_below(b, m, &pass[s..i]);
                    for j in s..i {
                        terms.push(and_of(b, &generate[j], &pass[j + 1..i]));
                    }
                    out.push(terms);
                }
            }
            out.push(from_below(b, ranges.len(), &[]));
        }
    }
    out
}

/// Sum and carry of three bits in 4 gates, the fewest there is (found by exact search): sum = xor(a, b, c),
/// carry = xor(sum, and(a, b, c), or(a, b, c)). With a bit that is always 0 it is a half adder, 2 gates.
pub fn full_adder(b: &mut Builder, x: S, y: S, c: S) -> (S, S) {
    let ins = [x, y, c];
    let sum = b.xor(&ins);
    if ins.contains(&S::Zero) {
        let rest: Vec<S> = ins.iter().copied().filter(|&s| s != S::Zero).collect();
        return (sum, if rest.len() == 2 { b.and(&rest) } else { S::Zero });
    }
    let (all, any) = (b.and(&ins), b.or(&ins));
    (sum, b.xor(&[sum, all, any]))
}

/// One line per value of the select bits: line k is on when they spell k and every signal in `also` is on.
pub fn decode(b: &mut Builder, select: &[S], also: &[S]) -> Vec<S> {
    let not: Vec<S> = select.iter().map(|&s| b.not(s)).collect();
    (0..1usize << select.len())
        .map(|k| {
            let mut xs: Vec<S> = (0..select.len()).map(|j| if k >> j & 1 == 1 { select[j] } else { not[j] }).collect();
            xs.extend_from_slice(also);
            b.and(&xs)
        })
        .collect()
}

/// Any function of a few input bits in 3 ticks: every output is one XOR gate over AND gates of plain inputs
/// (its algebraic normal form), so no inverters are needed. Only for small inputs: the number of AND gates
/// can double with every input bit.
pub fn normal_form(bits: usize, out_bits: usize, f: &dyn Fn(u128) -> u128) -> Result<Netlist, String> {
    if bits > 16 {
        return Err("too many input bits for a normal form".into());
    }
    let mut b = Builder::new(bits);
    let table: Vec<u128> = (0..1u128 << bits).map(f).collect();
    let mut outs = vec![];
    for o in 0..out_bits {
        let mut on: Vec<bool> = table.iter().map(|v| v >> o & 1 == 1).collect();
        for i in 0..bits {
            for x in 0..on.len() {
                if x >> i & 1 == 1 {
                    on[x] ^= on[x ^ (1 << i)];
                }
            }
        }
        let mut terms = vec![];
        for (mask, _) in on.iter().enumerate().filter(|(_, on)| **on) {
            let vars: Vec<S> = (0..bits).filter(|i| mask >> i & 1 == 1).map(|i| b.input(i)).collect();
            terms.push(b.and(&vars));
        }
        outs.push(b.xor(&terms));
    }
    b.finish(&outs)
}
