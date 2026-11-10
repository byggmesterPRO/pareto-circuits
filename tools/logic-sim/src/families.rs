//! What each circuit family has to compute. Size is the folder name, for example "12bit".
//!
//! Inputs and outputs are numbers: input pin i is bit i of the input number, output pin i is bit i of the
//! output number. A family with several operands packs them into the input number and says how.

pub struct Family {
    pub bits: u32,
    pub out_bits: u32,
    /// The correct output for an input.
    pub f: Box<dyn Fn(u128) -> u128 + Send + Sync>,
    /// The outputs of inputs in a row, starting at the first argument: low 64 bits and the bits above. Has to
    /// give exactly what `f` gives, only quicker. Used when every input is tried (32 input bits or fewer).
    pub run: Box<dyn Fn(u64, &mut [u64], &mut [u64]) + Send + Sync>,
    /// Inputs that are likely to catch a mistake, for sizes where the inputs are sampled.
    pub hard: Vec<u128>,
}

pub const FAMILIES: [&str; 2] = ["adder", "double-dabble"];

pub fn family(name: &str, size: &str) -> Result<Family, String> {
    match name {
        "adder" => adder(size),
        "double-dabble" => double_dabble(size),
        _ => Err(format!("unknown family \"{name}\"; known: {}", FAMILIES.join(", "))),
    }
}

fn bits_of(family: &str, size: &str, max: u32) -> Result<u32, String> {
    size.strip_suffix("bit").and_then(|s| s.parse().ok()).filter(|b| (1..=max).contains(b)).ok_or(format!("{family}: bad size \"{size}\" (1bit to {max}bit)"))
}

/// Binary to BCD. Output digit d (ones first) is bits 4d .. 4d+3, least significant first.
fn double_dabble(size: &str) -> Result<Family, String> {
    let bits = bits_of("double-dabble", size, 64)?;
    let top = (1u128 << bits) - 1;
    let digits = top.to_string().len() as u32;
    let f = move |x: u128| {
        let (mut x, mut out) = (x as u64, 0u128);
        for d in 0..digits {
            out |= ((x % 10) as u128) << (4 * d);
            x /= 10;
        }
        out
    };
    Ok(Family {
        bits,
        out_bits: 4 * digits,
        f: Box::new(f),
        // count in decimal: add 1 and carry, instead of dividing for every input
        run: Box::new(move |first, lo, hi| {
            let mut v = f(first as u128);
            for (l, h) in lo.iter_mut().zip(hi.iter_mut()) {
                *l = v as u64;
                *h = (v >> 64) as u64;
                let mut d = 0;
                loop {
                    if d >= 4 * digits {
                        v = 0; // past the largest value; never read
                        break;
                    }
                    if (v >> d) & 15 == 9 {
                        v &= !(15u128 << d);
                        d += 4;
                    } else {
                        v += 1u128 << d;
                        break;
                    }
                }
            }
        }),
        hard: vec![],
    })
}

/// Adds two numbers of n bits. Inputs: a is bits 0 .. n-1, b is bits n .. 2n-1. Outputs: the sum in bits
/// 0 .. n-1 and the carry out in bit n.
fn adder(size: &str) -> Result<Family, String> {
    let n = bits_of("adder", size, 64)?;
    let mask = if n == 64 { u64::MAX as u128 } else { (1u128 << n) - 1 };
    let f = move |x: u128| (x & mask) + (x >> n & mask);
    // long carry chains: a run of ones in a, and a single one in b at its low end
    let mut hard = vec![];
    let run = |i: u32, j: u32| (mask >> (n - 1 - j)) & !((1u128 << i) - 1);
    for i in 0..n {
        for j in i..n {
            hard.push(run(i, j) | (1u128 << i) << n);
            hard.push(1u128 << i | run(i, j) << n);
        }
    }
    // b mostly the opposite of a: every bit passes a carry on, with a few bits that start or stop one
    let mut s: u64 = 0xD1B54A32D192ED03;
    let mut word = || {
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        s.wrapping_mul(0x2545F4914F6CDD1D) as u128
    };
    for _ in 0..20_000 {
        let a = word() & mask;
        let b = (!a ^ (word() & word() & word())) & mask;
        hard.push(a | b << n);
    }
    Ok(Family {
        bits: 2 * n,
        out_bits: n + 1,
        f: Box::new(f),
        run: Box::new(move |first, lo, hi| {
            for (k, (l, h)) in lo.iter_mut().zip(hi.iter_mut()).enumerate() {
                let v = f(first as u128 + k as u128);
                *l = v as u64;
                *h = (v >> 64) as u64;
            }
        }),
        hard,
    })
}
