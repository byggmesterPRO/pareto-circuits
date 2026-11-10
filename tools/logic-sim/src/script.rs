//! Small test scripts for the tick simulator, after the test cases in niknal's circuitlib.
//!
//! One step per line:
//!
//! ```text
//! set in0 1        drive a pin or part (on the next tick it shows the value)
//! release in0      stop driving it
//! tick 3           run 3 game ticks
//! settle           tick until nothing changes
//! check out2 0     count one check, and whether it held
//! // a comment
//! ```
//!
//! A name is `in<k>` or `out<k>` (the pins, least significant bit 0) or `#<controller id>` for any part.

use crate::circuit::Circuit;
use crate::sim::Sim;

pub struct Outcome {
    pub correct: u64,
    pub total: u64,
    pub ticks: u64,
    /// The checks that did not hold, as "line N: text".
    pub failed: Vec<String>,
}

fn resolve(c: &Circuit, name: &str) -> Result<u32, String> {
    if let Some(k) = name.strip_prefix("in").and_then(|k| k.parse::<usize>().ok()) {
        return c.inputs.get(k).copied().ok_or(format!("there is no input {k}"));
    }
    if let Some(k) = name.strip_prefix("out").and_then(|k| k.parse::<usize>().ok()) {
        return c.outputs.get(k).copied().flatten().ok_or(format!("output {k} has no gate"));
    }
    if let Some(id) = name.strip_prefix('#').and_then(|k| k.parse::<i64>().ok()) {
        return c.id.iter().position(|&x| x == id).map(|e| e as u32).ok_or(format!("there is no part {id}"));
    }
    Err(format!("\"{name}\" is not in<k>, out<k> or #<id>"))
}

fn bit(v: Option<&&str>) -> Result<bool, String> {
    match v.copied() {
        Some("1") | Some("on") | Some("true") => Ok(true),
        Some("0") | Some("off") | Some("false") => Ok(false),
        _ => Err("expected 0 or 1".into()),
    }
}

pub fn run(c: &Circuit, text: &str) -> Result<Outcome, String> {
    let mut sim = Sim::new(c, 1);
    let mut out = Outcome { correct: 0, total: 0, ticks: 0, failed: vec![] };
    for (n, line) in text.lines().enumerate() {
        let line = line.split("//").next().unwrap_or("").trim();
        let words: Vec<&str> = line.split_whitespace().collect();
        let at = |e: String| format!("line {}: {e}", n + 1);
        match words.first().copied() {
            None => {}
            Some("set") => sim.force_all(resolve(c, words.get(1).unwrap_or(&"")).map_err(at)?, bit(words.get(2)).map_err(at)?),
            Some("release") => sim.release(resolve(c, words.get(1).unwrap_or(&"")).map_err(at)?),
            Some("tick") => {
                let k: u64 = words.get(1).and_then(|k| k.parse().ok()).unwrap_or(1);
                for _ in 0..k {
                    sim.tick();
                }
            }
            Some("settle") => {
                sim.settle(100_000).ok_or(at("the circuit never settles".into()))?;
            }
            Some("check") => {
                let e = resolve(c, words.get(1).unwrap_or(&"")).map_err(at)?;
                let want = bit(words.get(2)).map_err(at)?;
                out.total += 1;
                if (sim.get(e)[0] & 1 == 1) == want {
                    out.correct += 1;
                } else {
                    out.failed.push(format!("line {}: {}", n + 1, line));
                }
            }
            Some(other) => return Err(at(format!("unknown step \"{other}\""))),
        }
    }
    out.ticks = sim.tick_count;
    Ok(out)
}
