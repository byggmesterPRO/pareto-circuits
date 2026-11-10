//! Can this be built in the game. The rules come from niknal's circuitlib.

use crate::circuit::{Circuit, Kind, MAX_CONNECTIONS};
use std::collections::{HashMap, HashSet};

pub struct Report {
    /// Things that make the circuit invalid.
    pub problems: Vec<String>,
    /// Things that are allowed but worth knowing about.
    pub warnings: Vec<String>,
}

pub fn validate(c: &Circuit) -> Report {
    let mut r = Report { problems: vec![], warnings: vec![] };
    let readers = c.readers();
    let name = |e: usize| format!("part {}", c.id[e]);
    for e in 0..c.len() {
        if c.ins[e].len() > MAX_CONNECTIONS {
            r.problems.push(format!("{} has {} wires going in, the game allows {MAX_CONNECTIONS}", name(e), c.ins[e].len()));
        }
        if readers[e].len() > MAX_CONNECTIONS {
            r.problems.push(format!("{} has {} wires going out, the game allows {MAX_CONNECTIONS}", name(e), readers[e].len()));
        }
        if matches!(c.kind[e], Kind::Timer(_)) && c.ins[e].len() > 1 {
            r.problems.push(format!("{} is a timer with {} inputs, a timer takes one", name(e), c.ins[e].len()));
        }
        let mut seen = HashSet::new();
        if c.ins[e].iter().any(|i| !seen.insert(i)) {
            r.problems.push(format!("{} is wired to the same part twice", name(e)));
        }
    }
    for (i, &e) in c.inputs.iter().enumerate() {
        if !c.ins[e as usize].is_empty() {
            r.problems.push(format!("input {i} ({}) has wires going into it", name(e as usize)));
        }
    }
    let selfwired = (0..c.len()).filter(|&e| c.ins[e].contains(&(e as u32))).count();
    if selfwired > 0 {
        r.warnings.push(format!("{selfwired} parts are wired to themselves"));
    }
    let twoloops = (0..c.len()).map(|e| c.ins[e].iter().filter(|&&i| i as usize > e && c.ins[i as usize].contains(&(e as u32))).count()).sum::<usize>();
    if twoloops > 0 {
        r.warnings.push(format!("{twoloops} pairs of parts are wired to each other"));
    }
    if !c.pos.is_empty() {
        let mut cells: HashMap<[i32; 3], usize> = HashMap::new();
        for p in &c.pos {
            *cells.entry(*p).or_default() += 1;
        }
        let overlaps = cells.values().filter(|&&n| n > 1).count();
        if overlaps > 0 {
            r.problems.push(format!("{overlaps} cells hold more than one part"));
        }
        // do the parts touch each other, so that they form one piece
        let mut left: HashSet<[i32; 3]> = cells.keys().copied().collect();
        let mut pieces = 0;
        while let Some(&start) = left.iter().next() {
            pieces += 1;
            left.remove(&start);
            let mut stack = vec![start];
            while let Some(p) = stack.pop() {
                for d in [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]] {
                    let q = [p[0] + d[0], p[1] + d[1], p[2] + d[2]];
                    if left.remove(&q) {
                        stack.push(q);
                    }
                }
            }
        }
        if pieces > 1 {
            r.warnings.push(format!("the parts do not touch: {pieces} separate pieces (timers are counted as one block)"));
        }
    }
    r
}
