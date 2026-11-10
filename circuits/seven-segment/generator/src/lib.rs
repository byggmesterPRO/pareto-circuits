//! Seven-segment generator: one hexadecimal digit to the segments of a display.
//!
//! Inputs: the digit, 4 bits. Outputs: segments a to g as bits 0 to 6 (a top, b top right, c bottom right,
//! d bottom, e bottom left, f top left, g middle).
//!
//! There is one size, so the designs here are fixed circuits rather than recipes.

use logic_sim::build::{Builder, Design, Netlist, S};
use logic_sim::families::family;
use logic_sim::parts::{decode, inputs, normal_form, offer};

pub const SIZES: [usize; 1] = [4];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    if n != 4 {
        return Err("seven-segment: the only size is 4bit".into());
    }
    let f = family("seven-segment", "4bit")?;
    let mut out = vec![Design::new("table", table(&*f.f))?];
    offer(&mut out, "normal-form", normal_form(4, 7, &*f.f));
    for (name, gates, outputs) in SEARCHED {
        let net = Netlist { inputs: 4, gates: gates.iter().map(|(m, ins)| (*m, ins.to_vec())).collect(), outputs: outputs.iter().map(|&o| Some(o)).collect(), grid: 0 };
        out.push(Design { name: name.to_string(), netlist: net });
    }
    Ok(out)
}

/// One line per digit, and every segment is an OR over the digits that light it (or a NOR over the digits
/// that do not, when those are fewer). 4 ticks.
fn table(f: &dyn Fn(u128) -> u128) -> Result<Netlist, String> {
    let mut b = Builder::new(4);
    let digit = inputs(&b, 0, 4);
    let lines = decode(&mut b, &digit, &[]);
    let outs: Vec<S> = (0..7)
        .map(|seg| {
            let (lit, dark): (Vec<usize>, Vec<usize>) = (0..16).partition(|&d| f(d as u128) >> seg & 1 == 1);
            if dark.len() < lit.len() {
                let xs: Vec<S> = dark.iter().map(|&d| lines[d]).collect();
                b.nor(&xs)
            } else {
                let xs: Vec<S> = lit.iter().map(|&d| lines[d]).collect();
                b.or(&xs)
            }
        })
        .collect();
    b.finish(&outs)
}

/// Circuits found by search (optimizers/fable): name, gates as (mode, inputs), and the signal of each segment.
/// Signals 0 to 3 are the digit's bits, signal 4 + g is gate g.
const SEARCHED: &[(&str, &[(u8, &[u32])], &[u32])] = &[(
    // resub from `normal-form`, two rounds on 12 cores: 18 gates down to 14. Exact search could not settle
    // whether 13 is possible.
    "searched-small",
    &[
        (1, &[0, 1]),
        (5, &[1, 2, 3, 4]),
        (0, &[1, 2]),
        (4, &[0, 2]),
        (1, &[5, 6, 7]),
        (3, &[0, 3, 8]),
        (2, &[0, 1, 4, 7, 8, 9]),
        (1, &[0, 1, 2]),
        (2, &[0, 3, 4, 5, 6, 8, 9, 11]),
        (3, &[5, 11, 12]),
        (3, &[3, 8, 10]),
        (2, &[0, 3, 4, 6, 8, 13, 14]),
        (3, &[4, 10, 14]),
        (2, &[1, 2, 3, 11, 12, 13]),
    ],
    &[8, 10, 13, 15, 12, 16, 17],
)];
