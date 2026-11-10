//! ALU generator: one of eight operations on two n-bit numbers, picked by a 3-bit operation code.
//!
//! Inputs: a is bits 0 .. n-1, b is bits n .. 2n-1, the operation is the 3 bits above.
//! Outputs: the result in bits 0 .. n-1 and a flag in bit n.
//!
//! 0 a + b (flag: carry), 1 a - b (flag: borrow), 2 a and b, 3 a or b, 4 a xor b, 5 a shifted up one place
//! (flag: the bit that fell off), 6 a shifted down one place (flag: the bit that fell off), 7 not a.
//!
//! Add and subtract share one adder-subtractor, with the lowest operation bit as its mode. Every operation
//! has a line that is on when it is picked, and each result bit is an OR over one AND gate per operation.

use logic_sim::build::{Builder, Design, Netlist, S};
use logic_sim::parts::{add_sub, decode, inputs, Carry};

pub const SIZES: [usize; 5] = [2, 4, 8, 16, 32];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    let mut out = vec![Design::new("ripple", build(n, None))?, Design::new("lookahead", build(n, Some(Carry::Blocks(n + 1))))?];
    for k in [2, 4, 8, 16] {
        if k < n {
            out.push(Design::new(format!("blocks-{k}"), build(n, Some(Carry::Blocks(k))))?);
        }
    }
    for k in [4, 8] {
        if 2 * k <= n {
            out.push(Design::new(format!("two-level-{k}"), build(n, Some(Carry::TwoLevel(k))))?);
        }
    }
    Ok(out)
}

fn build(n: usize, carry: Option<Carry>) -> Result<Netlist, String> {
    let mut b = Builder::new(2 * n + 3);
    let (x, y, op) = (inputs(&b, 0, n), inputs(&b, n, n), inputs(&b, 2 * n, 3));
    let line = decode(&mut b, &op, &[]);
    let arithmetic = b.nor(&[op[1], op[2]]);
    let sum = add_sub(&mut b, &x, &y, op[0], carry);
    let mut outs = vec![];
    for i in 0..n {
        let (either, differ, not) = (b.or(&[x[i], y[i]]), b.xor(&[x[i], y[i]]), b.not(x[i]));
        let mut terms = vec![b.and(&[arithmetic, sum[i]]), b.and(&[line[2], x[i], y[i]]), b.and(&[line[3], either]), b.and(&[line[4], differ]), b.and(&[line[7], not])];
        if i > 0 {
            terms.push(b.and(&[line[5], x[i - 1]]));
        }
        if i + 1 < n {
            terms.push(b.and(&[line[6], x[i + 1]]));
        }
        outs.push(b.or(&terms));
    }
    let flags: Vec<S> = vec![b.and(&[arithmetic, sum[n]]), b.and(&[line[5], x[n - 1]]), b.and(&[line[6], x[0]])];
    outs.push(b.or(&flags));
    b.finish(&outs)
}
