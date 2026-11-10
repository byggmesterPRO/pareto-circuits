//! Decoder generator: n input bits, one output line per value, and only the line of the current value is on.
//!
//! One design: an inverter per input, and one AND gate per line over the inputs or their inverses. Gates take
//! any number of inputs, so splitting the decoding into stages would only add gates and ticks.

use logic_sim::build::{Builder, Design, Netlist};
use logic_sim::parts::{decode, inputs};

/// The size is the number of input bits: 8bit has 256 lines.
pub const SIZES: [usize; 3] = [2, 4, 8];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    Ok(vec![Design::new("flat", flat(n))?])
}

fn flat(n: usize) -> Result<Netlist, String> {
    let mut b = Builder::new(n);
    let select = inputs(&b, 0, n);
    let lines = decode(&mut b, &select, &[]);
    let mut net = b.finish(&lines)?;
    // the lines as a block: the low half of the bits picks the column, the high half the row
    net.grid = if n >= 4 { 1 << (n / 2) } else { 0 };
    Ok(net)
}
