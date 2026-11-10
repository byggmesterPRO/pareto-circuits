//! RAM generator: words of memory, one of them picked by an address.
//!
//! Size is `<words>x<bits>bit`: `16x8bit` is 16 words of 8 bits.
//! Inputs: the address (log2 of the words), then the data to write, then write. Outputs: the word at the
//! address, all the time.
//!
//! A bit is stored in an XOR gate wired to itself, which flips when its other input is on for one tick. The
//! word at the address is always on the outputs, so "which bits differ from the data" is one XOR gate per
//! bit for the whole RAM, not one per cell. A cell is then 3 gates: the stored bit, its flip gate, and the
//! gate that puts it on the output when its word is picked.
//! Write has to be on for exactly one tick.

use logic_sim::build::{Builder, Design, Netlist, S, XOR};
use logic_sim::parts::inputs;

pub const SIZES: [&str; 4] = ["16x4bit", "16x8bit", "64x8bit", "256x8bit"];

pub fn designs(size: &str) -> Result<Vec<Design>, String> {
    let (words, width) = logic_sim::families::ram_size(size)?;
    let (words, width) = (words as usize, width as usize);
    Ok(vec![Design::new("direct", build(words, width, false))?, Design::new("lines", build(words, width, true))?])
}

/// `lines`: each word gets a "picked" line and a "write here" line that its cells read. Without, every cell
/// reads the address bits itself: 2 gates per word fewer and a tick less on a write, with more wires.
fn build(words: usize, width: usize, lines: bool) -> Result<Netlist, String> {
    let a = words.trailing_zeros() as usize;
    let mut b = Builder::new(a + width + 1);
    let (address, data, write) = (inputs(&b, 0, a), inputs(&b, a, width), b.input(a + width));
    let not: Vec<S> = address.iter().map(|&x| b.not(x)).collect();
    let picked: Vec<Vec<S>> = (0..words)
        .map(|k| {
            let bits: Vec<S> = (0..a).map(|j| if k >> j & 1 == 1 { address[j] } else { not[j] }).collect();
            if lines { vec![b.and(&bits)] } else { bits }
        })
        .collect();
    let write_here: Vec<Vec<S>> = picked
        .iter()
        .map(|p| {
            let mut xs = p.clone();
            xs.push(write);
            if lines { vec![b.and(&xs)] } else { xs }
        })
        .collect();
    let mut outs = vec![];
    for d in data {
        let cells: Vec<S> = (0..words).map(|_| b.later(XOR)).collect();
        let shown: Vec<S> = (0..words)
            .map(|k| {
                let mut xs = picked[k].clone();
                xs.push(cells[k]);
                b.and(&xs)
            })
            .collect();
        // an OR gate takes 255 inputs
        let parts: Vec<S> = shown.chunks(255).map(|c| b.or(c)).collect();
        let out = b.or(&parts);
        let differs = b.xor(&[d, out]);
        for k in 0..words {
            let mut xs = write_here[k].clone();
            xs.push(differs);
            let flip = b.and(&xs);
            b.wire(cells[k], &[cells[k], flip]);
        }
        outs.push(out);
    }
    b.finish(&outs)
}
