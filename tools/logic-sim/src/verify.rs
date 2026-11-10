//! Check a circuit against what its family should do, and measure its timing.
//!
//! This is the one place that asks whether a family has memory. A family without is a function of its
//! inputs: the circuit is tried on every input (or a sample), and its timing is its longest path. A family
//! with memory is a model with a state: the circuit is run through random operations on the tick simulator,
//! and its timing is measured there. Everything else takes a `Spec`, hands it over here, and gets the same
//! `Verified` back for both.

use crate::check::check;
use crate::circuit::Circuit;
use crate::families::Spec;
use crate::memory;

/// Random operations a circuit with memory is checked on, over 64 copies at once.
pub const OPERATIONS: u64 = 3000;

/// A circuit's timing, in ticks. A tick is counted from the moment an input is driven, so the input gate
/// itself is the first tick.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Timing {
    /// From an input changing to the outputs being right. For a family with memory: from a level input
    /// (data, address), with no pulse.
    pub read: u64,
    /// From a pulse to the outputs being right. None for a family without pulse inputs.
    pub write: Option<u64>,
    /// How long the level inputs have to be steady before a pulse, if they are to be read off first.
    pub setup: Option<u64>,
    /// How often new inputs can come: every this many ticks. Without memory that is the longest path minus
    /// the shortest, plus 1, so 1 for a balanced circuit. With memory it is the shortest gap between
    /// operations that was seen to work (see `memory::pace`), or None if none was found.
    pub period: Option<u64>,
    /// With memory: how many ticks after the level inputs the pulse comes, at that period.
    pub pulse_at: Option<u64>,
}

pub struct Verified {
    /// Test inputs, or operations, with a wrong output.
    pub wrong: u64,
    pub tested: u64,
    /// True if every input there is was tried. Never for a family with memory.
    pub exhaustive: bool,
    /// Without memory: the smallest input that gave a wrong output.
    pub example: Option<u128>,
    /// What is wrong, in words, if anything is.
    pub problem: Option<String>,
    /// The one number a build's meta.json calls `ticks`: the longest path, or for a family with memory the
    /// one of `read` and `write` its README names.
    pub ticks: u64,
    pub timing: Timing,
}

impl Verified {
    pub fn verified(&self) -> &'static str {
        if self.exhaustive { "exhaustive" } else { "sampled" }
    }
}

/// Fails only if the circuit cannot be checked at all: the wrong number of pins, or a loop where there
/// should be none. A circuit that gives wrong outputs comes back with `wrong` counted and `problem` set.
pub fn verify(c: &Circuit, spec: &Spec, threads: usize) -> Result<Verified, String> {
    let (bits, out_bits) = spec.pins();
    if (c.inputs.len() as u32, c.outputs.len() as u32) != (bits, out_bits) {
        return Err(format!("{} inputs and {} outputs, the family has {bits} and {out_bits}", c.inputs.len(), c.outputs.len()));
    }
    match spec {
        Spec::Function(f) => {
            let res = check(c, &f.expect(), threads, &f.hard)?;
            let ticks = c.ticks().unwrap_or(0) as u64;
            let problem = (res.wrong > 0).then(|| format!("{} of {} test inputs give a wrong output, for example input {}", res.wrong, res.tested, res.example.unwrap_or(0)));
            let timing = Timing { read: ticks, write: None, setup: None, period: c.balance().map(|b| b.period() as u64), pulse_at: None };
            Ok(Verified { wrong: res.wrong, tested: res.tested, exhaustive: res.exhaustive, example: res.example, problem, ticks, timing })
        }
        Spec::Machine(m) => {
            let res = memory::run(c, m, OPERATIONS, 0x5EED)?;
            let problem = (res.wrong > 0).then(|| format!("{} of {} operations end with a wrong output: {}", res.wrong, res.operations, res.example.clone().unwrap_or_default()));
            // how often operations can come is only worth asking of a circuit that works when given time
            let pace = if res.wrong == 0 { memory::pace(c, m, res.setup + res.write + 16, 0x5EED)? } else { None };
            let timing = Timing { read: res.read, write: Some(res.write), setup: Some(res.setup), period: pace.map(|p| p.period), pulse_at: pace.map(|p| p.pulse_at) };
            let ticks = if m.ticks_is_read { res.read } else { res.write };
            Ok(Verified { wrong: res.wrong, tested: res.operations, exhaustive: false, example: None, problem, ticks, timing })
        }
    }
}
