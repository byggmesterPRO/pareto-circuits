//! Command line for logic-sim. Every command prints one line of JSON and exits 1 if there is a problem.

use logic_sim::api::handle;
use serde_json::{json, Map, Value};
use std::io::{self, BufRead, Write};
use std::process::exit;
use std::{env, fs};

const USAGE: &str = "logic-sim: simulator and verifier for Scrap Mechanic logic circuits

A <circuit> is a build slot folder, a blueprint.json or a netlist.json.

  logic-sim verify <slot>                         check a build slot against its meta.json
  logic-sim verify <circuit> <family> <size>      check against a known family, for example double-dabble 12bit
  logic-sim equal <circuit> <reference circuit>   do two circuits give the same outputs for every input
  logic-sim check <circuit> --table <file.json>   check against a list with one output value per input
  logic-sim check <circuit> --cases <file.json>   check chosen inputs: [[input, output], ...]
  logic-sim count <circuit>                       gates, timers, ticks, and is it balanced
  logic-sim delay <circuit> [jumps]               worst output delay seen over random input changes
  logic-sim stream <circuit> [steps]              new inputs every tick: are the outputs right, ticks later
  logic-sim balance <circuit> <out.json> [ticks]  pad with timers until every path is the same; writes a netlist
  logic-sim validate <circuit>                    can it be built in the game
  logic-sim run <circuit> <script>                run a test script (set, tick, check) on the tick simulator
  logic-sim serve                                 stay running: one JSON request per line in, one reply out

  --pins <file.json>     inputs and outputs of a bare blueprint.json:
                         {\"inputs\": [controller id, ...], \"outputs\": [controller id or null, ...]}

Balanced means every path from an input to an output is the same number of ticks, so the inputs can change
every tick. A timer set to d counts as d + 1 gates in a row.

verify, equal, check, delay, stream and balance are for circuits without memory. Circuits with memory or
loops are tested with run. See README.md for the request format of serve.";

fn read(path: &str) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))
}

fn request(mut args: Vec<String>) -> Result<Value, String> {
    let mut req = Map::new();
    for flag in ["--pins", "--table", "--cases"] {
        if let Some(i) = args.iter().position(|a| a == flag) {
            let file = args.get(i + 1).ok_or(format!("{flag} needs a file"))?.clone();
            req.insert(flag[2..].to_string(), if flag == "--pins" { json!(file) } else { read(&file)? });
            args.drain(i..i + 2);
        }
    }
    let cmd = args.first().ok_or(USAGE)?.as_str();
    req.insert("circuit".into(), json!(args.get(1).ok_or(format!("{cmd} needs a circuit\n\n{USAGE}"))?));
    match cmd {
        "verify" => {
            if let (Some(f), Some(s)) = (args.get(2), args.get(3)) {
                req.insert("family".into(), json!(f));
                req.insert("size".into(), json!(s));
            }
        }
        "equal" => {
            req.insert("reference".into(), json!(args.get(2).ok_or("equal needs two circuits")?));
            if let Some(p) = req.get("pins").cloned() {
                req.insert("reference_pins".into(), p);
            }
        }
        "delay" => {
            if let Some(j) = args.get(2).and_then(|j| j.parse::<u64>().ok()) {
                req.insert("jumps".into(), json!(j));
            }
        }
        "stream" => {
            if let Some(n) = args.get(2).and_then(|n| n.parse::<u64>().ok()) {
                req.insert("steps".into(), json!(n));
            }
        }
        "balance" => {
            req.insert("out".into(), json!(args.get(2).ok_or("balance needs a file to write the netlist to")?));
            if let Some(t) = args.get(3) {
                req.insert("ticks".into(), json!(t.parse::<u64>().map_err(|_| format!("\"{t}\" is not a number of ticks"))?));
            }
        }
        "run" => {
            req.insert("script_file".into(), json!(args.get(2).ok_or("run needs a script file")?));
        }
        "check" | "count" | "validate" => {}
        _ => return Err(format!("unknown command \"{cmd}\"\n\n{USAGE}")),
    }
    req.insert("cmd".into(), json!(if cmd == "equal" { "check" } else { cmd }));
    Ok(Value::Object(req))
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args[0] == "help" || args[0] == "--help" || args[0] == "-h" {
        println!("{USAGE}");
        return;
    }
    if args[0] == "serve" {
        let stdout = io::stdout();
        for line in io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            if line.trim().is_empty() {
                continue;
            }
            let reply = match serde_json::from_str::<Value>(&line) {
                Ok(req) => handle(&req),
                Err(e) => json!({ "problems": [format!("not JSON: {e}")] }),
            };
            let mut out = stdout.lock();
            let _ = writeln!(out, "{reply}");
            let _ = out.flush();
        }
        return;
    }
    let reply = match request(args) {
        Ok(req) => handle(&req),
        Err(e) => {
            eprintln!("{e}");
            exit(2);
        }
    };
    println!("{reply}");
    exit(if reply["problems"].as_array().is_some_and(|p| !p.is_empty()) { 1 } else { 0 });
}
