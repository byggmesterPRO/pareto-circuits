//! Command line for logic-sim. Every command prints one line of JSON and exits 1 if something is wrong.
//!
//!   logic-sim verify <slot>                         a build slot, against its meta.json
//!   logic-sim verify <netlist.json> <family> <size>
//!   logic-sim count <slot | netlist.json>           gates and ticks
//!   logic-sim delay <slot | netlist.json> [jumps]   worst output delay seen over random input changes
//!   logic-sim validate <slot | blueprint.json>      can it be built in the game
//!   logic-sim run <slot | blueprint.json> <script>  a test script on the tick simulator

use logic_sim::circuit::{load, Circuit};
use logic_sim::families::family;
use logic_sim::{check, delay, script, validate};
use serde_json::{json, Value};
use std::path::Path;
use std::process::exit;
use std::{env, fs, thread};

fn threads() -> usize {
    thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

/// Correctness, gates and ticks of a circuit against a family.
fn verify(c: &Circuit, fam: &str, size: &str) -> Result<Value, String> {
    let f = family(fam, size)?;
    let mut problems = vec![];
    if c.inputs.len() as u32 != f.bits {
        problems.push(format!("{} inputs, {fam} {size} has {}", c.inputs.len(), f.bits));
    }
    if c.outputs.len() as u32 != f.out_bits {
        problems.push(format!("{} outputs, {fam} {size} has {}", c.outputs.len(), f.out_bits));
    }
    if !problems.is_empty() {
        return Ok(json!({ "problems": problems, "warnings": [] }));
    }
    let res = check::check(c, &*f.f, threads())?;
    let report = validate::validate(c);
    problems.extend(report.problems);
    if res.wrong > 0 {
        problems.push(format!("{} test inputs give a wrong output", res.wrong));
    }
    Ok(json!({
        "gates": c.gates(),
        "ticks": c.ticks(),
        "verified": if res.exhaustive { "exhaustive" } else { "sampled" },
        "tested": res.tested,
        "wrong": res.wrong,
        "problems": problems,
        "warnings": report.warnings,
    }))
}

fn run(args: &[String]) -> Result<Value, String> {
    let cmd = args.first().map(|s| s.as_str()).unwrap_or("");
    let path = Path::new(args.get(1).ok_or("missing path; see the top of src/main.rs for usage")?);
    let (c, meta) = load(path)?;
    match cmd {
        "verify" => {
            let Some(meta) = meta else {
                return verify(&c, args.get(2).ok_or("verify <netlist> needs <family> <size>")?, args.get(3).ok_or("verify <netlist> needs <family> <size>")?);
            };
            let (fam, size) = (meta["family"].as_str().unwrap_or(""), meta["size"].as_str().unwrap_or(""));
            let mut res = verify(&c, fam, size)?;
            let mut problems: Vec<Value> = res["problems"].as_array().cloned().unwrap_or_default();
            for key in ["gates", "ticks"] {
                if res.get(key).is_some() && meta[key] != res[key] {
                    problems.push(json!(format!("meta.json says {key} = {}, logic-sim gets {}", meta[key], res[key])));
                }
            }
            if meta["verified"] == "exhaustive" && res.get("verified").is_some_and(|v| v != "exhaustive") {
                problems.push(json!("meta.json says exhaustive, but this size can only be sampled"));
            }
            let full = fs::canonicalize(path).map_err(|e| e.to_string())?;
            let tail: Vec<String> = full.iter().rev().take(2).map(|p| p.to_string_lossy().into_owned()).collect();
            if tail.get(1).map(|s| s.as_str()) != Some(size) || tail.first().map(|s| s.as_str()) != meta["profile"].as_str() {
                problems.push(json!("meta.json size/profile do not match the folder"));
            }
            res["problems"] = Value::Array(problems);
            Ok(res)
        }
        "count" => Ok(json!({ "gates": c.gates(), "ticks": c.ticks(), "problems": [] })),
        "delay" => {
            if c.inputs.is_empty() {
                return Err("delay needs pins: give a build slot or a netlist".into());
            }
            let jumps = args.get(2).and_then(|k| k.parse().ok()).unwrap_or(200_000);
            Ok(json!({ "gates": c.gates(), "ticks": c.ticks(), "worst_delay_seen": delay::worst_seen(&c, jumps, 1)?, "jumps": jumps, "problems": [] }))
        }
        "validate" => {
            let r = validate::validate(&c);
            Ok(json!({ "gates": c.gates(), "problems": r.problems, "warnings": r.warnings }))
        }
        "run" => {
            let text = fs::read_to_string(args.get(2).ok_or("run needs a script file")?).map_err(|e| e.to_string())?;
            let o = script::run(&c, &text)?;
            Ok(json!({ "correct": o.correct, "total": o.total, "ticks": o.ticks, "problems": o.failed }))
        }
        _ => Err(format!("unknown command \"{cmd}\"; see the top of src/main.rs for usage")),
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(v) => {
            println!("{v}");
            exit(if v["problems"].as_array().is_some_and(|p| !p.is_empty()) { 1 } else { 0 });
        }
        Err(e) => {
            println!("{}", json!({ "problems": [e] }));
            exit(1);
        }
    }
}
