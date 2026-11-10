//! Demultiplexer generator: one data input sent to one of n output lines.
//!
//! Inputs: bit 0 is the data, the bits above are the line number. Outputs: n lines.
//!
//! The output gates are laid out as a matrix: the low half of the line number picks the column, the high half
//! the row.

use logic_sim::build::{Builder, Design, Netlist};
use logic_sim::parts::{decode, inputs};

/// The size is the number of output lines.
pub const SIZES: [usize; 6] = [2, 4, 8, 16, 32, 64];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    if n < 2 || !n.is_power_of_two() {
        return Err(format!("demultiplexer: {n} lines is not a power of two"));
    }
    let mut out = vec![Design::new("flat", flat(n))?];
    if n >= 4 {
        out.push(Design::new("matrix", matrix(n))?);
    }
    Ok(out)
}

fn finish(n: usize, mut net: Netlist) -> Netlist {
    let s = n.trailing_zeros() as usize;
    net.grid = if s >= 2 { 1 << (s / 2) } else { 0 };
    net
}

/// One AND gate per line over the data and the line number bits or their inverses. 3 ticks.
fn flat(n: usize) -> Result<Netlist, String> {
    let s = n.trailing_zeros() as usize;
    let mut b = Builder::new(1 + s);
    let (data, select) = (b.input(0), inputs(&b, 1, s));
    let lines = decode(&mut b, &select, &[data]);
    Ok(finish(n, b.finish(&lines)?))
}

/// Row and column: the high half of the line number is decoded into row lines (which also carry the data), the
/// low half into column lines, and each output is the AND of its row and its column. Every output gate then
/// has 2 wires instead of one per bit, at the cost of the row and column gates and one tick.
fn matrix(n: usize) -> Result<Netlist, String> {
    let s = n.trailing_zeros() as usize;
    let mut b = Builder::new(1 + s);
    let (data, select) = (b.input(0), inputs(&b, 1, s));
    let low = s / 2;
    let cols = decode(&mut b, &select[..low], &[]);
    let rows = decode(&mut b, &select[low..], &[data]);
    let lines: Vec<_> = (0..n).map(|k| b.and(&[rows[k >> low], cols[k & ((1 << low) - 1)]])).collect();
    Ok(finish(n, b.finish(&lines)?))
}
