//! What each circuit family has to compute. Size is the folder name, for example "12bit".
//!
//! Inputs and outputs are numbers: input pin i is bit i of the input number, output pin i is bit i of the
//! output number. A family with several operands packs them into the input number and says how.

use crate::check::Expect;

pub struct Family {
    pub bits: u32,
    pub out_bits: u32,
    /// The correct output for an input. For a family with more than 128 outputs this is not used.
    pub f: Box<dyn Fn(u128) -> u128 + Send + Sync>,
    /// The outputs of inputs in a row, starting at the first argument: low 64 bits and the bits above. Has to
    /// give exactly what `f` gives, only quicker. Used when every input is tried (32 input bits or fewer).
    pub run: Box<dyn Fn(u64, &mut [u64], &mut [u64]) + Send + Sync>,
    /// For a family with more than 128 outputs: sets the output bits that are on, in words of 64.
    pub wide: Option<Box<dyn Fn(u128, &mut [u64]) + Send + Sync>>,
    /// Inputs that are likely to catch a mistake, for sizes where the inputs are sampled.
    pub hard: Vec<u128>,
}

impl Family {
    /// What a circuit of this family is checked against.
    pub fn expect(&self) -> Expect<'_> {
        match &self.wide {
            Some(w) => Expect::Wide(&**w),
            None => Expect::Run(&*self.f, &*self.run),
        }
    }

    fn new(bits: u32, out_bits: u32, f: impl Fn(u128) -> u128 + Copy + Send + Sync + 'static, hard: Vec<u128>) -> Family {
        Family {
            bits,
            out_bits,
            f: Box::new(f),
            run: Box::new(move |first, lo, hi| {
                for (k, (l, h)) in lo.iter_mut().zip(hi.iter_mut()).enumerate() {
                    let v = f(first as u128 + k as u128);
                    *l = v as u64;
                    *h = (v >> 64) as u64;
                }
            }),
            wide: None,
            hard,
        }
    }
}

pub const FAMILIES: [&str; 10] =
    ["adder", "barrel-shifter", "comparator", "decoder", "demultiplexer", "double-dabble", "multiplexer", "multiplier", "seven-segment", "subtractor"];

pub fn family(name: &str, size: &str) -> Result<Family, String> {
    match name {
        "adder" => adder(size),
        "barrel-shifter" => barrel_shifter(size),
        "comparator" => comparator(size),
        "decoder" => decoder(size),
        "demultiplexer" => demultiplexer(size),
        "double-dabble" => double_dabble(size),
        "multiplexer" => multiplexer(size),
        "multiplier" => multiplier(size),
        "seven-segment" => seven_segment(size),
        "subtractor" => subtractor(size),
        _ => Err(format!("unknown family \"{name}\"; known: {}, {}", FAMILIES.join(", "), MACHINES.join(", "))),
    }
}

/// A family with memory, as a model: what the outputs should be after an operation (see `memory`).
pub struct Machine {
    pub bits: u32,
    pub out_bits: u32,
    /// The inputs that are pulses of exactly one tick (write, count, reset). The others are levels.
    pub pulses: Vec<u32>,
    /// The state of a circuit that was just built.
    pub start: Box<dyn Fn() -> Vec<u128> + Send + Sync>,
    /// Apply the inputs (a pulse input is 1 if it was pulsed) to the state and give the outputs.
    pub step: Box<dyn Fn(&mut Vec<u128>, u128) -> u128 + Send + Sync>,
    /// True if `ticks` in a build's meta.json is the read delay (inputs to outputs); otherwise it is the
    /// delay from a pulse to the outputs.
    pub ticks_is_read: bool,
}

pub const MACHINES: [&str; 3] = ["counter", "ram", "register"];

pub fn machine(name: &str, size: &str) -> Result<Machine, String> {
    match name {
        "counter" => {
            let n = bits_of("counter", size, 64)?;
            Ok(Machine {
                bits: 2,
                out_bits: n,
                pulses: vec![0, 1],
                start: Box::new(|| vec![0]),
                step: Box::new(move |s, x| {
                    if x & 1 == 1 {
                        s[0] = (s[0] + 1) & mask(n);
                    }
                    if x & 2 == 2 {
                        s[0] = 0;
                    }
                    s[0]
                }),
                ticks_is_read: false,
            })
        }
        "register" => {
            let n = bits_of("register", size, 64)?;
            Ok(Machine {
                bits: n + 1,
                out_bits: n,
                pulses: vec![n],
                start: Box::new(|| vec![0]),
                step: Box::new(move |s, x| {
                    if x >> n & 1 == 1 {
                        s[0] = x & mask(n);
                    }
                    s[0]
                }),
                ticks_is_read: false,
            })
        }
        "ram" => {
            let (words, width) = ram_size(size)?;
            let a = words.trailing_zeros();
            Ok(Machine {
                bits: a + width + 1,
                out_bits: width,
                pulses: vec![a + width],
                start: Box::new(move || vec![0; words as usize]),
                step: Box::new(move |s, x| {
                    let at = (x & mask(a)) as usize;
                    if x >> (a + width) & 1 == 1 {
                        s[at] = x >> a & mask(width);
                    }
                    s[at]
                }),
                ticks_is_read: true,
            })
        }
        _ => Err(format!("unknown family \"{name}\"; known: {}, {}", FAMILIES.join(", "), MACHINES.join(", "))),
    }
}

/// Words and bits per word in a RAM size like "16x8bit".
pub fn ram_size(size: &str) -> Result<(u32, u32), String> {
    let bad = || format!("ram: bad size \"{size}\" (like 16x8bit: 16 words of 8 bits; the words a power of two up to 1024, the bits up to 64)");
    let (w, b) = size.strip_suffix("bit").and_then(|s| s.split_once('x')).ok_or_else(bad)?;
    let (w, b): (u32, u32) = (w.parse().map_err(|_| bad())?, b.parse().map_err(|_| bad())?);
    if w < 2 || w > 1024 || !w.is_power_of_two() || b < 1 || b > 64 {
        return Err(bad());
    }
    Ok((w, b))
}

/// The number in a size like "16bit".
pub fn bits_of(family: &str, size: &str, max: u32) -> Result<u32, String> {
    size.strip_suffix("bit").and_then(|s| s.parse().ok()).filter(|b| (1..=max).contains(b)).ok_or(format!("{family}: bad size \"{size}\" (1bit to {max}bit)"))
}

/// The same for a size that has to be a power of two; also gives its logarithm.
pub fn power_of_two(family: &str, size: &str, max: u32) -> Result<(u32, u32), String> {
    let n = bits_of(family, size, max)?;
    if n < 2 || !n.is_power_of_two() {
        return Err(format!("{family}: size \"{size}\" has to be a power of two, 2bit or more"));
    }
    Ok((n, n.trailing_zeros()))
}

fn mask(n: u32) -> u128 {
    if n >= 128 { u128::MAX } else { (1u128 << n) - 1 }
}

/// A source of test inputs that is the same every time.
struct Random(u64);
impl Random {
    fn word(&mut self) -> u128 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545F4914F6CDD1D) as u128
    }
    /// Mostly zeros.
    fn sparse(&mut self) -> u128 {
        self.word() & self.word() & self.word()
    }
}

/// Binary to BCD. Output digit d (ones first) is bits 4d .. 4d+3, least significant first.
fn double_dabble(size: &str) -> Result<Family, String> {
    let bits = bits_of("double-dabble", size, 64)?;
    let digits = mask(bits).to_string().len() as u32;
    let f = move |x: u128| {
        let (mut x, mut out) = (x as u64, 0u128);
        for d in 0..digits {
            out |= ((x % 10) as u128) << (4 * d);
            x /= 10;
        }
        out
    };
    let mut fam = Family::new(bits, 4 * digits, f, vec![]);
    // count in decimal: add 1 and carry, instead of dividing for every input
    fam.run = Box::new(move |first, lo, hi| {
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
    });
    Ok(fam)
}

/// Adds two numbers of n bits. Inputs: a is bits 0 .. n-1, b is bits n .. 2n-1. Outputs: the sum in bits
/// 0 .. n-1 and the carry out in bit n.
fn adder(size: &str) -> Result<Family, String> {
    let n = bits_of("adder", size, 64)?;
    let m = mask(n);
    // long carry chains: a run of ones in a, and a single one in b at its low end
    let mut hard = vec![];
    let run = |i: u32, j: u32| (m >> (n - 1 - j)) & !((1u128 << i) - 1);
    for i in 0..n {
        for j in i..n {
            hard.push(run(i, j) | (1u128 << i) << n);
            hard.push(1u128 << i | run(i, j) << n);
        }
    }
    // b mostly the opposite of a: every bit passes a carry on, with a few bits that start or stop one
    let mut r = Random(0xD1B54A32D192ED03);
    for _ in 0..20_000 {
        let a = r.word() & m;
        hard.push(a | ((!a ^ r.sparse()) & m) << n);
    }
    Ok(Family::new(2 * n, n + 1, move |x| (x & m) + (x >> n & m), hard))
}

/// a - b for two numbers of n bits. Inputs: a is bits 0 .. n-1, b is bits n .. 2n-1. Outputs: the difference
/// in bits 0 .. n-1 (wrapping around below zero) and the borrow in bit n, which is on when a < b.
fn subtractor(size: &str) -> Result<Family, String> {
    let n = bits_of("subtractor", size, 64)?;
    let m = mask(n);
    // long borrow chains: b has a single one, and a is zero (the borrow runs to the top) or has a single one
    // further up (the borrow stops there)
    let mut hard = vec![];
    for i in 0..n {
        hard.push((1u128 << i) << n);
        for j in i + 1..n {
            hard.push(1u128 << j | (1u128 << i) << n);
        }
    }
    // a mostly equal to b: every bit passes a borrow on
    let mut r = Random(0x9E3779B97F4A7C15);
    for _ in 0..20_000 {
        let b = r.word() & m;
        hard.push(((b ^ r.sparse()) & m) | b << n);
    }
    Ok(Family::new(2 * n, n + 1, move |x| {
        let (a, b) = (x & m, x >> n & m);
        (a.wrapping_sub(b) & m) | ((a < b) as u128) << n
    }, hard))
}

/// Compares two numbers of n bits. Inputs: a is bits 0 .. n-1, b is bits n .. 2n-1. Outputs: bit 0 is a < b,
/// bit 1 is a = b, bit 2 is a > b.
fn comparator(size: &str) -> Result<Family, String> {
    let n = bits_of("comparator", size, 64)?;
    let m = mask(n);
    // equal numbers, and numbers that differ in one bit only: random inputs almost never hit these
    let mut hard = vec![];
    let mut r = Random(0xA0761D6478BD642F);
    for _ in 0..2_000 {
        let a = r.word() & m;
        hard.push(a | a << n);
    }
    for i in 0..n {
        for _ in 0..300 {
            let a = r.word() & m;
            hard.push(a | (a ^ 1u128 << i) << n);
        }
    }
    Ok(Family::new(2 * n, 3, move |x| {
        let (a, b) = (x & m, x >> n & m);
        (a < b) as u128 | ((a == b) as u128) << 1 | ((a > b) as u128) << 2
    }, hard))
}

/// Multiplies two numbers of n bits. Inputs: a is bits 0 .. n-1, b is bits n .. 2n-1. Outputs: the product,
/// 2n bits.
fn multiplier(size: &str) -> Result<Family, String> {
    let n = bits_of("multiplier", size, 64)?;
    let m = mask(n);
    let mut hard = vec![m | m << n];
    for i in 0..n {
        for j in 0..n {
            hard.push(1u128 << i | (1u128 << j) << n);
            hard.push((m >> i) | (m >> j) << n);
        }
    }
    Ok(Family::new(2 * n, 2 * n, move |x| (x & m) * (x >> n & m), hard))
}

/// One output line per value of the input: line k is on when the n input bits spell k, the others are off.
fn decoder(size: &str) -> Result<Family, String> {
    let n = bits_of("decoder", size, 10)?;
    let mut fam = Family::new(n, 1 << n, move |x| if x < 128 { 1u128 << x } else { 0 }, vec![]);
    if n > 7 {
        fam.wide = Some(Box::new(|x, out| out[(x / 64) as usize] |= 1 << (x % 64)));
    }
    Ok(fam)
}

/// One data input sent to one of n output lines. Inputs: bit 0 is the data, bits 1 and up are the line number.
/// Outputs: line k carries the data when the line number is k, and is off otherwise.
fn demultiplexer(size: &str) -> Result<Family, String> {
    let (n, _) = power_of_two("demultiplexer", size, 256)?;
    let mut fam = Family::new(1 + n.trailing_zeros(), n, move |x| if x >> 1 < 128 { (x & 1) << (x >> 1) } else { 0 }, vec![]);
    if n > 128 {
        fam.wide = Some(Box::new(|x, out| out[(x >> 1) as usize / 64] |= ((x & 1) as u64) << ((x >> 1) % 64)));
    }
    Ok(fam)
}

/// One of n data inputs sent to the output. Inputs: the data lines are bits 0 .. n-1, the line number is the
/// bits above. Output: the chosen line.
fn multiplexer(size: &str) -> Result<Family, String> {
    let (n, s) = power_of_two("multiplexer", size, 64)?;
    let m = mask(n);
    let mut hard = vec![];
    for k in 0..n as u128 {
        for d in [0, m, 1u128 << k, m ^ 1u128 << k] {
            hard.push(d | k << n);
        }
    }
    Ok(Family::new(n + s, 1, move |x| (x & m) >> (x >> n) & 1, hard))
}

/// Shifts an n-bit number left. Inputs: the number is bits 0 .. n-1, the shift amount is the bits above
/// (log2 n of them). Outputs: the number moved up by that many places; bits that fall off the top are lost and
/// zeros come in at the bottom.
fn barrel_shifter(size: &str) -> Result<Family, String> {
    let (n, s) = power_of_two("barrel-shifter", size, 64)?;
    let m = mask(n);
    let mut hard = vec![];
    for amount in 0..n as u128 {
        hard.push(m | amount << n);
        for i in 0..n {
            hard.push(1u128 << i | amount << n);
            hard.push((m ^ 1u128 << i) | amount << n);
        }
    }
    Ok(Family::new(n + s, n, move |x| ((x & m) << (x >> n)) & m, hard))
}

/// Segments of a seven-segment display for one hexadecimal digit. Inputs: the digit, 4 bits. Outputs: bit 0 is
/// segment a (top), then b (top right), c (bottom right), d (bottom), e (bottom left), f (top left), g (middle).
/// A to F are shown as A, b, C, d, E, F.
fn seven_segment(size: &str) -> Result<Family, String> {
    if size != "4bit" {
        return Err("seven-segment: the only size is 4bit, one hexadecimal digit".into());
    }
    const GLYPHS: [u8; 16] = [0x3F, 0x06, 0x5B, 0x4F, 0x66, 0x6D, 0x7D, 0x07, 0x7F, 0x6F, 0x77, 0x7C, 0x39, 0x5E, 0x79, 0x71];
    Ok(Family::new(4, 7, |x| GLYPHS[(x & 15) as usize] as u128, vec![]))
}
