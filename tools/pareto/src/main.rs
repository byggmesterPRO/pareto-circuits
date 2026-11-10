//! pareto: run the generators, measure what they make, and write build slots.
//!
//! A family's generator offers several designs for a size. Each one is checked and counted by logic-sim, and
//! the three profiles go to whichever design wins by the README's rule: fast is fewest ticks, small is fewest
//! gates, balanced is lowest gates x ticks. A slot is only written when the winner beats what is in it.

use logic_sim::build::{Design, Netlist};
use logic_sim::check::{check, Expect};
use logic_sim::circuit::load;
use logic_sim::families::{family, FAMILIES};
use logic_sim::validate::validate;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs, process, thread};

const USAGE: &str = "pareto: generators and builds for the circuit families

  pareto designs <family> <size>                  every design the generator has for this size, measured
  pareto netlist <family> <size> <design> <out>   write one design as a netlist.json
  pareto build <family> <size> [--write]          pick the fast, small and balanced winners for one size
  pareto build-all <family> [--write]             the same for 2, 4, 8, 16, 32 and 64 bit

  --write              write the winners into circuits/<family>/builds/ (without it, only show what would happen)
  --authors a,b        who to credit in meta.json for a new slot

Run it from inside the repo. Families: ";
const STANDARD: [usize; 6] = [2, 4, 8, 16, 32, 64];
const PROFILES: [&str; 3] = ["fast", "small", "balanced"];
const LINK: &str = "https://github.com/byggmesterPRO/pareto-circuits";

fn designs(fam: &str, bits: usize) -> Result<Vec<Design>, String> {
    match fam {
        "adder" => adder_generator::designs(bits),
        "double-dabble" => double_dabble_generator::designs(bits),
        _ => Err(format!("no generator for \"{fam}\"; there are generators for: {}", FAMILIES.join(", "))),
    }
}

struct Measured {
    name: String,
    netlist: Netlist,
    gates: u64,
    ticks: u64,
    verified: &'static str,
}

fn threads() -> usize {
    thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

/// Check a design against its family and count it. A wrong or unbuildable design is an error.
fn measure(fam: &str, size: &str, d: Design) -> Result<Measured, String> {
    let f = family(fam, size)?;
    let c = d.netlist.circuit();
    if c.inputs.len() as u32 != f.bits || c.outputs.len() as u32 != f.out_bits {
        return Err(format!("{}: {} inputs and {} outputs, {fam} {size} has {} and {}", d.name, c.inputs.len(), c.outputs.len(), f.bits, f.out_bits));
    }
    let res = check(&c, &Expect::Run(&*f.f, &*f.run), threads(), &f.hard)?;
    if res.wrong > 0 {
        return Err(format!("{}: {} of {} test inputs give a wrong output, for example input {}", d.name, res.wrong, res.tested, res.example.unwrap_or(0)));
    }
    let problems = validate(&c).problems;
    if !problems.is_empty() {
        return Err(format!("{}: {}", d.name, problems.join("; ")));
    }
    Ok(Measured { name: d.name, netlist: d.netlist, gates: c.gates() as u64, ticks: c.ticks().unwrap_or(0) as u64, verified: if res.exhaustive { "exhaustive" } else { "sampled" } })
}

/// Lower wins.
fn score(profile: &str, gates: u64, ticks: u64) -> (u64, u64) {
    match profile {
        "fast" => (ticks, gates),
        "small" => (gates, ticks),
        _ => (gates * ticks, ticks),
    }
}

fn bits_of(size: &str) -> Result<usize, String> {
    size.strip_suffix("bit").and_then(|s| s.parse().ok()).ok_or(format!("\"{size}\" is not a size like 8bit"))
}

fn measured(fam: &str, size: &str) -> Result<Vec<Measured>, String> {
    designs(fam, bits_of(size)?)?.into_iter().map(|d| measure(fam, size, d)).collect()
}

fn repo_root() -> Result<PathBuf, String> {
    let mut dir = env::current_dir().map_err(|e| e.to_string())?;
    loop {
        if dir.join("circuits").is_dir() && dir.join("tools").is_dir() {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err("run pareto from inside the pareto-circuits repo".into());
        }
    }
}

/// Today's date as YYYY-MM-DD (UTC).
fn today() -> String {
    let days = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() / 86400).unwrap_or(0) as i64;
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let (d, m) = (doy - (153 * mp + 2) / 5 + 1, if mp < 10 { mp + 3 } else { mp - 9 });
    format!("{:04}-{:02}-{:02}", yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
}

/// A random id in the shape the game uses for localId.
fn new_id() -> String {
    let mut s = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1) ^ (process::id() as u64) << 32 | 1;
    let mut word = || {
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        s.wrapping_mul(0x2545F4914F6CDD1D)
    };
    let (a, b) = (word(), word());
    format!("{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}", a >> 32, a >> 16 & 0xFFFF, a & 0xFFF, 0x8000 | b >> 48 & 0x3FFF, b & 0xFFFF_FFFF_FFFF)
}

fn title(fam: &str, size: &str, profile: &str) -> String {
    let cap = |w: &str| w[..1].to_uppercase() + &w[1..];
    format!("Pareto {} {size} {}", fam.split('-').map(cap).collect::<Vec<_>>().join(" "), cap(profile))
}

/// What a slot holds now: (gates, ticks, version), if it holds anything.
fn current(slot: &Path) -> Option<(u64, u64, String, Value)> {
    let meta: Value = serde_json::from_str(&fs::read_to_string(slot.join("meta.json")).ok()?).ok()?;
    Some((meta["gates"].as_u64()?, meta["ticks"].as_u64()?, meta["version"].as_str()?.to_string(), meta))
}

fn write_slot(slot: &Path, fam: &str, size: &str, profile: &str, m: &Measured, authors: &[String]) -> Result<String, String> {
    let old = current(slot);
    let mut version = "1.0".to_string();
    let mut local_id = new_id();
    let mut workshop = json!("");
    let mut authors: Vec<String> = authors.to_vec();
    if let Some((_, old_ticks, old_version, meta)) = &old {
        let (major, minor) = old_version.split_once('.').and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?))).ok_or("the slot's version is not like 1.0")?;
        version = if *old_ticks != m.ticks { format!("{}.0", major + 1) } else { format!("{major}.{}", minor + 1) };
        workshop = meta["workshop"].clone();
        let desc: Value = serde_json::from_str(&fs::read_to_string(slot.join("blueprint").join("description.json")).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        local_id = desc["localId"].as_str().ok_or("the slot's description.json has no localId")?.to_string();
        if authors.is_empty() {
            authors = meta["authors"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
        }
    }
    if authors.is_empty() {
        return Err("a new slot needs --authors".into());
    }
    let (bp, pins) = m.netlist.blueprint();
    fs::create_dir_all(slot.join("blueprint")).map_err(|e| e.to_string())?;
    fs::write(slot.join("blueprint").join("blueprint.json"), bp.to_string()).map_err(|e| e.to_string())?;
    let desc = json!({ "description": format!("v{version} | {} gates | {} ticks\n{LINK}", m.gates, m.ticks), "localId": local_id, "name": title(fam, size, profile), "type": "Blueprint", "version": 0 });
    fs::write(slot.join("blueprint").join("description.json"), serde_json::to_string_pretty(&desc).unwrap() + "\n").map_err(|e| e.to_string())?;
    let meta = json!({
        "family": fam, "size": size, "profile": profile, "version": version, "gates": m.gates, "ticks": m.ticks,
        "source": "generator", "verified": m.verified, "optimal": "best-known", "authors": authors, "date": today(),
        "workshop": workshop, "notes": format!("Generator design: {}.", m.name), "pins": pins,
    });
    fs::write(slot.join("meta.json"), serde_json::to_string_pretty(&meta).unwrap() + "\n").map_err(|e| e.to_string())?;
    // read the slot back the way the verifier does and check it once more
    let (c, _) = load(slot, None)?;
    let f = family(fam, size)?;
    let res = check(&c, &Expect::Run(&*f.f, &*f.run), threads(), &f.hard)?;
    if res.wrong > 0 || c.gates() as u64 != m.gates || c.ticks().map(|t| t as u64) != Some(m.ticks) {
        return Err(format!("the slot written to {} does not check out", slot.display()));
    }
    Ok(version)
}

fn build(fam: &str, size: &str, write: bool, authors: &[String]) -> Result<(), String> {
    let all = measured(fam, size)?;
    let root = repo_root()?;
    for profile in PROFILES {
        let best = all.iter().min_by_key(|m| score(profile, m.gates, m.ticks)).ok_or("the generator has no designs for this size")?;
        let slot = root.join("circuits").join(fam).join("builds").join(size).join(profile);
        let now = current(&slot);
        let wins = now.as_ref().is_none_or(|(g, t, _, _)| score(profile, best.gates, best.ticks) < score(profile, *g, *t));
        let what = match (&now, wins) {
            (None, _) => "new slot".to_string(),
            (Some((g, t, v, _)), true) => format!("beats v{v} ({g} gates, {t} ticks)"),
            (Some((g, t, v, _)), false) => format!("kept v{v} ({g} gates, {t} ticks)"),
        };
        let done = if write && wins { format!(", wrote v{}", write_slot(&slot, fam, size, profile, best, authors)?) } else { String::new() };
        println!("{fam:<14}{size:>6} {profile:<9} {:<14}{:>6} gates{:>5} ticks  {:<10} {what}{done}", best.name, best.gates, best.ticks, best.verified);
    }
    Ok(())
}

fn run(mut args: Vec<String>) -> Result<(), String> {
    let write = args.iter().position(|a| a == "--write").map(|i| args.remove(i)).is_some();
    let authors: Vec<String> = match args.iter().position(|a| a == "--authors") {
        Some(i) => {
            let list = args.get(i + 1).ok_or("--authors needs names")?.split(',').map(String::from).collect();
            args.drain(i..i + 2);
            list
        }
        None => vec![],
    };
    let arg = |i: usize| args.get(i).map(|s| s.as_str()).ok_or(format!("{USAGE}{}", FAMILIES.join(", ")));
    match arg(0)? {
        "designs" => {
            let (fam, size) = (arg(1)?, arg(2)?);
            let all = measured(fam, size)?;
            println!("{:<14}{:>7}{:>7}{:>14}  {:<11}wins", "design", "gates", "ticks", "gates x ticks", "verified");
            for m in &all {
                let wins: Vec<&str> = PROFILES.into_iter().filter(|p| all.iter().min_by_key(|o| score(p, o.gates, o.ticks)).is_some_and(|b| b.name == m.name)).collect();
                println!("{:<14}{:>7}{:>7}{:>14}  {:<11}{}", m.name, m.gates, m.ticks, m.gates * m.ticks, m.verified, wins.join(", "));
            }
            Ok(())
        }
        "netlist" => {
            let (fam, size, name) = (arg(1)?, arg(2)?, arg(3)?);
            let d = designs(fam, bits_of(size)?)?.into_iter().find(|d| d.name == name).ok_or(format!("{fam} has no design \"{name}\" at {size}"))?;
            let m = measure(fam, size, d)?;
            fs::write(arg(4)?, m.netlist.to_json().to_string() + "\n").map_err(|e| e.to_string())?;
            println!("{}: {} gates, {} ticks, {}", m.name, m.gates, m.ticks, m.verified);
            Ok(())
        }
        "build" => build(arg(1)?, arg(2)?, write, &authors),
        "build-all" => STANDARD.iter().try_for_each(|b| build(arg(1)?, &format!("{b}bit"), write, &authors)),
        other => Err(format!("unknown command \"{other}\"\n\n{USAGE}{}", FAMILIES.join(", "))),
    }
}

fn main() {
    if let Err(e) = run(env::args().skip(1).collect()) {
        eprintln!("{e}");
        process::exit(1);
    }
}
