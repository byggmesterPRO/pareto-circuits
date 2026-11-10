//! Barrel shifter generator: an n-bit number moved up by 0 to n-1 places.
//!
//! Inputs: the number is bits 0 .. n-1, the shift amount is the bits above. Outputs: the shifted number,
//! n bits; what moves past the top is lost and zeros come in at the bottom.

use logic_sim::build::{Builder, Design, Netlist, S};
use logic_sim::families::family;
use logic_sim::parts::{inputs, normal_form, offer};

/// The size is the width of the number. It has to be a power of two.
pub const SIZES: [usize; 6] = [2, 4, 8, 16, 32, 64];

pub fn designs(n: usize) -> Result<Vec<Design>, String> {
    if n < 2 || !n.is_power_of_two() {
        return Err(format!("barrel-shifter: {n} bits is not a power of two"));
    }
    let s = n.trailing_zeros() as usize;
    let mut out = vec![Design::new("flat", stages(n, s))?];
    for r in 1..s {
        out.push(Design::new(format!("stages-{r}"), stages(n, r))?);
    }
    if n + s <= 12 {
        let f = family("barrel-shifter", &format!("{n}bit"))?;
        offer(&mut out, "normal-form", normal_form(n + s, n, &*f.f));
    }
    Ok(out)
}

/// The amount bits are taken r at a time, lowest first. A stage that takes the bits worth 2^g and up moves
/// the number by 0, 1, .. 2^r - 1 times 2^g places: each output bit is an OR over one AND gate per choice.
/// Two ticks per stage. r = all the bits is a single stage: 4 ticks at any size and about n x n / 2 gates.
/// r = 1 is the classic chain of two-way choices: the most ticks, and at large sizes the fewest gates.
fn stages(n: usize, r: usize) -> Result<Netlist, String> {
    let s = n.trailing_zeros() as usize;
    let mut b = Builder::new(n + s);
    let (mut cur, amount) = (inputs(&b, 0, n), inputs(&b, n, s));
    let not: Vec<S> = amount.iter().map(|&x| b.not(x)).collect();
    for g in (0..s).step_by(r) {
        let bits = r.min(s - g);
        cur = (0..n)
            .map(|i| {
                let terms: Vec<S> = (0..1usize << bits)
                    .filter(|k| k << g <= i)
                    .map(|k| {
                        let mut xs: Vec<S> = (0..bits).map(|j| if k >> j & 1 == 1 { amount[g + j] } else { not[g + j] }).collect();
                        xs.push(cur[i - (k << g)]);
                        b.and(&xs)
                    })
                    .collect();
                b.or(&terms)
            })
            .collect();
    }
    b.finish(&cur)
}
