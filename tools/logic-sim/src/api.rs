//! Every command as one JSON request and one JSON reply. The command line and `serve` both go through here.
//!
//! A request is {"cmd": ..., "circuit": ..., ...}. "circuit" is a path (build slot, blueprint.json or
//! netlist.json) or the JSON itself. A bare blueprint needs "pins": {"inputs": [controller id, ...],
//! "outputs": [controller id or null, ...]}, or a path to a file with that.
//!
//! | cmd      | extra fields                                             |
//! |----------|----------------------------------------------------------|
//! | verify   | slot: nothing. Otherwise "family" and "size".            |
//! | check    | one of "table", "cases", "reference" (see below)         |
//! | count    |                                                          |
//! | delay    | "jumps" (default 200000)                                 |
//! | validate |                                                          |
//! | run      | "script" (the text) or "script_file"                     |
//!
//! check compares a circuit without memory against what it should give:
//! - "table": one output value per input, for every input, in order
//! - "cases": [[input, output], ...] for chosen inputs
//! - "reference": another circuit (path or JSON) with the same pins
//! Values above 2^53 are given as strings, decimal or 0x hex.
//!
//! Every reply has "problems". An empty list means all is well.

use crate::check::{check, Checked, Expect};
use crate::circuit::{from_value, load, read_pins, Circuit};
use crate::families::family;
use crate::{delay, script, validate};
use serde_json::{json, Value};
use std::path::Path;
use std::{fs, thread};

fn threads() -> usize {
    thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

fn number(v: &Value) -> Result<u128, String> {
    if let Some(n) = v.as_u64() {
        return Ok(n as u128);
    }
    let s = v.as_str().ok_or(format!("{v} is not a number"))?;
    match s.strip_prefix("0x") {
        Some(hex) => u128::from_str_radix(hex, 16),
        None => s.parse(),
    }
    .map_err(|_| format!("\"{s}\" is not a number"))
}

/// Numbers above 2^53 do not survive JSON, so they go out as strings.
fn big(v: u128) -> Value {
    if v < 1 << 53 { json!(v as u64) } else { json!(v.to_string()) }
}

fn circuit(v: &Value, pins: Option<&Value>) -> Result<(Circuit, Option<Value>), String> {
    match v.as_str() {
        Some(path) => load(Path::new(path), pins),
        None if v.is_object() => Ok((from_value(v, pins)?, None)),
        None => Err("\"circuit\" must be a path or a blueprint or netlist".into()),
    }
}

/// Gates, ticks and the outcome of a check, plus the buildability report.
fn report(c: &Circuit, res: Checked) -> Value {
    let v = validate::validate(c);
    let mut problems = v.problems;
    if res.wrong > 0 {
        problems.push(format!("{} of {} test inputs give a wrong output, for example input {}", res.wrong, res.tested, res.example.unwrap_or(0)));
    }
    json!({
        "gates": c.gates(),
        "ticks": c.ticks(),
        "verified": if res.exhaustive { "exhaustive" } else { "sampled" },
        "tested": res.tested,
        "wrong": res.wrong,
        "example": res.example.map(big),
        "problems": problems,
        "warnings": v.warnings,
    })
}

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
    Ok(report(c, check(c, &Expect::Run(&*f.f, &*f.run), threads(), &f.hard)?))
}

fn run(req: &Value) -> Result<Value, String> {
    let cmd = req["cmd"].as_str().ok_or("the request has no \"cmd\"")?;
    let pins = match req.get("pins") {
        Some(p) => Some(read_pins(p)?),
        None => None,
    };
    let (c, meta) = circuit(req.get("circuit").ok_or("the request has no \"circuit\"")?, pins.as_ref())?;
    let need_pins = || if c.inputs.is_empty() { Err("this needs to know the inputs and outputs: give a build slot, a netlist, or \"pins\"".to_string()) } else { Ok(()) };
    match cmd {
        "verify" => {
            need_pins()?;
            let Some(meta) = meta.filter(|_| req.get("family").is_none()) else {
                let fam = req["family"].as_str().ok_or("verify needs \"family\" and \"size\" unless the circuit is a build slot")?;
                return verify(&c, fam, req["size"].as_str().ok_or("verify needs \"size\"")?);
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
            let full = fs::canonicalize(req["circuit"].as_str().unwrap_or("")).map_err(|e| e.to_string())?;
            let tail: Vec<String> = full.iter().rev().take(2).map(|p| p.to_string_lossy().into_owned()).collect();
            if tail.get(1).map(|s| s.as_str()) != Some(size) || tail.first().map(|s| s.as_str()) != meta["profile"].as_str() {
                problems.push(json!("meta.json size/profile do not match the folder"));
            }
            res["problems"] = Value::Array(problems);
            Ok(res)
        }
        "check" => {
            need_pins()?;
            if let Some(t) = req.get("table") {
                let table = t.as_array().ok_or("\"table\" must be a list")?.iter().map(number).collect::<Result<Vec<_>, _>>()?;
                Ok(report(&c, check(&c, &Expect::Table(&table), threads(), &[])?))
            } else if let Some(cs) = req.get("cases") {
                let mut cases = vec![];
                for case in cs.as_array().ok_or("\"cases\" must be a list of [input, output]")? {
                    cases.push((number(&case[0])?, number(&case[1])?));
                }
                Ok(report(&c, check(&c, &Expect::Cases(&cases), threads(), &[])?))
            } else if let Some(r) = req.get("reference") {
                let ref_pins = match req.get("reference_pins") {
                    Some(p) => Some(read_pins(p)?),
                    None => None,
                };
                let (rc, _) = circuit(r, ref_pins.as_ref())?;
                Ok(report(&c, check(&c, &Expect::Circuit(&rc), threads(), &[])?))
            } else {
                Err("check needs \"table\", \"cases\" or \"reference\"".into())
            }
        }
        "count" => Ok(json!({ "gates": c.gates(), "ticks": c.ticks(), "parts": c.len(), "problems": [] })),
        "delay" => {
            need_pins()?;
            let jumps = req["jumps"].as_u64().unwrap_or(200_000) as usize;
            Ok(json!({ "gates": c.gates(), "ticks": c.ticks(), "worst_delay_seen": delay::worst_seen(&c, jumps, 1)?, "jumps": jumps, "problems": [] }))
        }
        "validate" => {
            let r = validate::validate(&c);
            Ok(json!({ "gates": c.gates(), "parts": c.len(), "problems": r.problems, "warnings": r.warnings }))
        }
        "run" => {
            let text = match req["script"].as_str() {
                Some(t) => t.to_string(),
                None => fs::read_to_string(req["script_file"].as_str().ok_or("run needs \"script\" or \"script_file\"")?).map_err(|e| e.to_string())?,
            };
            let o = script::run(&c, &text)?;
            Ok(json!({ "correct": o.correct, "total": o.total, "ticks": o.ticks, "problems": o.failed }))
        }
        _ => Err(format!("unknown command \"{cmd}\"")),
    }
}

/// Answer one request. Never fails: an error comes back as {"problems": [text]}.
pub fn handle(req: &Value) -> Value {
    run(req).unwrap_or_else(|e| json!({ "problems": [e] }))
}
