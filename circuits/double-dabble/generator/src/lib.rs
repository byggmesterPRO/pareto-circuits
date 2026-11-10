//! Double dabble generator: binary to BCD for any number of bits.
//!
//! Output digit d (ones first) is bits 4d .. 4d+3, least significant first.
//!
//! Both designs are the classic shift-and-add-3 chain: feed the bits in from the most significant end, and
//! before each new bit is shifted in, add 3 to every BCD digit that is 5 or more. A digit only gets an add-3
//! cell at the steps where it can reach 5 at all. The designs differ in the cell.

use logic_sim::build::{Builder, Design, Netlist, S};

pub const SIZES: [usize; 6] = [2, 4, 8, 16, 32, 64];

/// Every design for this size. The tool that makes builds measures them and picks the winners.
pub fn designs(bits: usize) -> Result<Vec<Design>, String> {
    Ok(vec![Design::new("chain-7", chain(bits, cell_7))?, Design::new("chain-8", chain(bits, cell_8))?])
}

type Cell = fn(&mut Builder, [S; 4]) -> [S; 4];

/// The add-3 cell with the fewest gates: 7, which exact synthesis (SAT) showed is the minimum for a cell
/// taken alone. Up to 4 gates deep.
fn cell_7(b: &mut Builder, d: [S; 4]) -> [S; 4] {
    let g4 = b.nor(&[d[0], d[2], d[3]]);
    let g5 = b.or(&[d[0], d[1], g4]);
    let g6 = b.xnor(&[d[3], g5]);
    let g7 = b.and(&[d[3], g5, g6]);
    let g8 = b.xor(&[d[2], d[3], g6, g7]);
    let g9 = b.xor(&[d[0], g8]);
    let g10 = b.xnor(&[d[0], d[1], d[2], d[3], g4, g8]);
    [g9, g10, g6, g8]
}

/// The add-3 cell in two layers: four AND terms, then one XOR per output bit. 8 gates, 2 gates deep.
fn cell_8(b: &mut Builder, d: [S; 4]) -> [S; 4] {
    let t02 = b.and(&[d[0], d[2]]);
    let t12 = b.and(&[d[1], d[2]]);
    let t012 = b.and(&[d[0], d[1], d[2]]);
    let t03 = b.and(&[d[0], d[3]]);
    [b.xor(&[d[0], d[3], t02, t12, t012]), b.xor(&[d[1], d[3], t12, t012, t03]), b.xor(&[d[2], t02, t12, t012, t03]), b.xor(&[d[3], t02, t12, t012])]
}

fn chain(bits: usize, cell: Cell) -> Result<Netlist, String> {
    if !(1..=64).contains(&bits) {
        return Err(format!("{bits} bits; this generator makes 1 to 64"));
    }
    let digits = ((1u128 << bits) - 1).to_string().len();
    let mut b = Builder::new(bits);
    let mut reg = vec![[S::Zero; 4]; digits];
    for k in 0..bits {
        // k bits are in the register; the next one in is input bit bits - 1 - k
        for d in 0..digits {
            if (1u128 << k) - 1 >= 5 * 10u128.pow(d as u32) {
                reg[d] = cell(&mut b, reg[d]);
            }
        }
        let mut carry = b.input(bits - 1 - k);
        for digit in reg.iter_mut() {
            let top = digit[3];
            *digit = [carry, digit[0], digit[1], digit[2]];
            carry = top;
        }
    }
    let outs: Vec<S> = reg.iter().flatten().copied().collect();
    b.finish(&outs)
}
