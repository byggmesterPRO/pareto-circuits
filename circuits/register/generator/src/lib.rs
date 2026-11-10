//! Register generator: n bits that are kept until the next write.
//!
//! Inputs: the data is bits 0 .. n-1, write is bit n. Outputs: the stored value.
//!
//! A bit is stored in an XOR gate wired to itself: it keeps its value, and flips when its other input is on
//! for one tick. A write flips exactly the bits that differ from the data: flip = write AND (data XOR stored).
//! That is 3 gates per bit. Write has to be on for exactly one tick; two ticks would flip the bits back.

use logic_sim::build::{Builder, Design, Netlist, XOR};
use logic_sim::parts::inputs;

pub const SIZES: [usize; 6] = [2, 4, 8, 16, 32, 64];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    Ok(vec![Design::new("toggle", toggle(n))?])
}

fn toggle(n: usize) -> Result<Netlist, String> {
    let mut b = Builder::new(n + 1);
    let (data, write) = (inputs(&b, 0, n), b.input(n));
    let mut outs = vec![];
    for d in data {
        let stored = b.later(XOR);
        let differs = b.xor(&[d, stored]);
        let flip = b.and(&[write, differs]);
        b.wire(stored, &[stored, flip]);
        outs.push(stored);
    }
    b.finish(&outs)
}
