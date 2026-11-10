//! What each circuit family has to compute. Size is the folder name, for example "12bit".

pub struct Family {
    pub bits: u32,
    pub out_bits: u32,
    /// The correct output for an input.
    pub f: Box<dyn Fn(u64) -> u128 + Send + Sync>,
}

pub fn family(name: &str, size: &str) -> Result<Family, String> {
    match name {
        "double-dabble" => double_dabble(size),
        _ => Err(format!("unknown family \"{name}\"")),
    }
}

/// Binary to BCD. Output digit d (ones first) is bits 4d .. 4d+3, least significant first.
fn double_dabble(size: &str) -> Result<Family, String> {
    let bits: u32 = size.strip_suffix("bit").and_then(|s| s.parse().ok()).filter(|b| (1..=64).contains(b)).ok_or(format!("double-dabble: bad size \"{size}\""))?;
    let top = (1u128 << bits) - 1;
    let digits = top.to_string().len() as u32;
    Ok(Family {
        bits,
        out_bits: 4 * digits,
        f: Box::new(move |x| {
            let (mut x, mut out) = (x, 0u128);
            for d in 0..digits {
                out |= ((x % 10) as u128) << (4 * d);
                x /= 10;
            }
            out
        }),
    })
}
