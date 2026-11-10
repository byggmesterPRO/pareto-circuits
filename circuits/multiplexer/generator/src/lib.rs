//! Multiplexer generator: one of n data inputs sent to the output.
//!
//! Inputs: the data lines are bits 0 .. n-1, the line number is the bits above. Output: the chosen line.

use logic_sim::build::{Builder, Design, Netlist, S};
use logic_sim::parts::{inputs, offer};

/// The size is the number of data lines.
pub const SIZES: [usize; 6] = [2, 4, 8, 16, 32, 64];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    if n < 2 || !n.is_power_of_two() {
        return Err(format!("multiplexer: {n} lines is not a power of two"));
    }
    let mut out = vec![Design::new("flat", flat(n))?];
    if n <= 32 {
        offer(&mut out, "no-inverters", no_inverters(n));
    }
    Ok(out)
}

/// One AND gate per data line over the line and the line number bits or their inverses, and an OR over them.
/// 4 ticks.
fn flat(n: usize) -> Result<Netlist, String> {
    let s = n.trailing_zeros() as usize;
    let mut b = Builder::new(n + s);
    let (data, select) = (inputs(&b, 0, n), inputs(&b, n, s));
    let not: Vec<S> = select.iter().map(|&x| b.not(x)).collect();
    let terms: Vec<S> = (0..n)
        .map(|k| {
            let mut xs: Vec<S> = (0..s).map(|j| if k >> j & 1 == 1 { select[j] } else { not[j] }).collect();
            xs.push(data[k]);
            b.and(&xs)
        })
        .collect();
    let out = b.or(&terms);
    b.finish(&[out])
}

/// 3 ticks: "d and not s" is d xor (d and s), so a term with inverses can be written as the XOR of terms
/// without, and the whole multiplexer is one XOR gate over AND gates of plain inputs. A line whose number has
/// z zero bits takes 2^z AND gates, 3^(number of select bits) in all, so this stops at 32 lines (243 terms).
fn no_inverters(n: usize) -> Result<Netlist, String> {
    let s = n.trailing_zeros() as usize;
    let mut b = Builder::new(n + s);
    let (data, select) = (inputs(&b, 0, n), inputs(&b, n, s));
    let mut terms = vec![];
    for k in 0..n {
        // every way of adding select bits where the line number has a zero
        for extra in 0..n {
            if extra & k == 0 {
                let mut xs: Vec<S> = (0..s).filter(|j| (k | extra) >> j & 1 == 1).map(|j| select[j]).collect();
                xs.push(data[k]);
                terms.push(b.and(&xs));
            }
        }
    }
    let out = b.xor(&terms);
    b.finish(&[out])
}
