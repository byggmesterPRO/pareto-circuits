//! Balance a circuit: pad the short paths with timers until every path from an input to an output is the same
//! number of ticks. The outputs then always belong to one tick's inputs, so the inputs can change every tick.
//!
//! Every gate gets a level, the tick at which it shows a new input (an input gate is level 1, the outputs are
//! all at the last level). A gate has to be at least one level after everything it reads. Where it is more
//! than one after, that wire needs a delay, and a delay of k ticks is one timer set to k - 1. Readers of the
//! same signal that need the same delay share the timer, so a signal costs one timer per different delay its
//! readers need.
//!
//! How many timers that takes depends on the levels, and there is freedom in them: a gate off the longest
//! path can sit early with the delay behind it, or late with the delay in front. Finding the levels with the
//! fewest timers is a search here, not a proof: it starts from every gate as early as possible and from every
//! gate as late as possible, moves single gates while that saves timers, shakes the result with random moves
//! that are sometimes allowed to cost one, and keeps the best. The count is a best-known, like gates are.

use crate::build::Netlist;
use crate::circuit::{MAX_CONNECTIONS, MAX_TIMER};
use std::collections::BTreeMap;

struct Levels<'a> {
    /// Signals each gate reads that an input reaches, without repeats. Empty for an input pin.
    reads: &'a [Vec<u32>],
    /// Gates that read each signal.
    readers: &'a [Vec<u32>],
    output: &'a [bool],
    last: u32,
    level: Vec<u32>,
    /// For each signal, how many of its readers sit at each level.
    at: Vec<BTreeMap<u32, u32>>,
}

impl Levels<'_> {
    fn new<'a>(reads: &'a [Vec<u32>], readers: &'a [Vec<u32>], output: &'a [bool], last: u32, level: Vec<u32>) -> Levels<'a> {
        let mut at = vec![BTreeMap::new(); level.len()];
        for (s, rs) in readers.iter().enumerate() {
            for &r in rs {
                *at[s].entry(level[r as usize]).or_insert(0) += 1;
            }
        }
        Levels { reads, readers, output, last, level, at }
    }

    /// Timers behind signal s if it sat at level l.
    fn timers(&self, s: usize, l: u32) -> i64 {
        self.at[s].range(l + 2..).count() as i64 + (self.output[s] && l < self.last) as i64
    }

    fn total(&self) -> i64 {
        (0..self.level.len()).map(|s| self.timers(s, self.level[s])).sum()
    }

    /// The levels gate g can move to without moving anything else.
    fn window(&self, g: usize) -> (u32, u32) {
        let lo = self.reads[g].iter().map(|&p| self.level[p as usize] + 1).max().unwrap_or(1);
        let hi = self.readers[g].iter().map(|&r| self.level[r as usize] - 1).min().unwrap_or(self.last).min(self.last);
        (lo, hi)
    }

    /// Timers added (or saved, if negative) by moving gate g to level to.
    fn change(&self, g: usize, to: u32) -> i64 {
        let from = self.level[g];
        let mut d = self.timers(g, to) - self.timers(g, from);
        for &p in &self.reads[g] {
            let (p, lp) = (p as usize, self.level[p as usize]);
            // g leaves `from`: a timer goes if g was its only reader. g comes to `to`: one is needed if none is there.
            d -= (from > lp + 1 && self.at[p][&from] == 1) as i64;
            d += (to > lp + 1 && !self.at[p].contains_key(&to)) as i64;
        }
        d
    }

    fn put(&mut self, g: usize, to: u32) {
        let (from, reads) = (self.level[g], self.reads);
        for &p in &reads[g] {
            let at = &mut self.at[p as usize];
            if at[&from] == 1 {
                at.remove(&from);
            } else {
                *at.get_mut(&from).unwrap() -= 1;
            }
            *at.entry(to).or_insert(0) += 1;
        }
        self.level[g] = to;
    }

    /// Move single gates for as long as a move saves a timer.
    fn settle(&mut self, gates: &[usize]) {
        let mut order = gates.to_vec();
        loop {
            let mut saved = false;
            for &g in &order {
                let (lo, hi) = self.window(g);
                let best = (lo..=hi).filter(|&l| l != self.level[g]).map(|l| (self.change(g, l), l)).min();
                if let Some((_, l)) = best.filter(|b| b.0 < 0) {
                    self.put(g, l);
                    saved = true;
                }
            }
            if !saved {
                return;
            }
            order.reverse();
        }
    }

    /// Random single moves: one that saves a timer or costs none is taken, one that costs a timer is taken
    /// now and then, less often as it goes on. Returns the best levels seen.
    fn shake(&mut self, gates: &[usize], moves: usize, seed: u64) -> (i64, Vec<u32>) {
        let mut s = 0x9E3779B97F4A7C15u64 ^ seed.wrapping_mul(0xD1B54A32D192ED03);
        let mut word = || {
            s ^= s >> 12;
            s ^= s << 25;
            s ^= s >> 27;
            s.wrapping_mul(0x2545F4914F6CDD1D)
        };
        let mut now = self.total();
        let mut best = (now, self.level.clone());
        for k in 0..moves {
            let g = gates[(word() % gates.len() as u64) as usize];
            let (lo, hi) = self.window(g);
            if lo == hi {
                continue;
            }
            let mut to = lo + (word() % (hi - lo) as u64) as u32;
            to += (to >= self.level[g]) as u32;
            let d = self.change(g, to);
            // a move that costs d timers is taken with chance 2^-(heat * d), and heat goes from 2 to 12
            let heat = 2.0 + 10.0 * k as f64 / moves as f64;
            if d <= 0 || (word() >> 11) as f64 / (1u64 << 53) as f64 * 2f64.powf(heat * d as f64) < 1.0 {
                self.put(g, to);
                now += d;
                if now < best.0 {
                    best = (now, self.level.clone());
                }
            }
        }
        best
    }
}

/// How hard `balance` looks for the levels with the fewest timers: random moves per gate, and at most in all,
/// from each of the two starts. Ten times more found nothing on most builds and a timer or two on a few.
pub const MOVES_PER_GATE: usize = 4000;
pub const MOVES_AT_MOST: usize = 20_000_000;

/// The netlist with timers added so that every path from an input to an output is `ticks` ticks, or as many
/// as its longest path if `ticks` is None. The gates stay as they are (apart from any that no output depends
/// on, which go), and so do the outputs' values. Fails if `ticks` is less than the longest path, or if the
/// netlist has memory or timers already.
pub fn balance(net: &Netlist, ticks: Option<u32>) -> Result<Netlist, String> {
    if !net.timers.is_empty() {
        return Err("the netlist has timers already".into());
    }
    let mut net = net.clone();
    net.prune();
    let (n, signals) = (net.inputs, net.inputs + net.gates.len());
    if net.gates.iter().enumerate().any(|(g, (_, ins))| ins.iter().any(|&i| i as usize >= n + g)) {
        return Err("only a circuit without memory can be balanced: this one has a gate that reads a later gate".into());
    }
    // a signal no input reaches is a constant: it has no timing, and a wire from it needs no delay
    let mut reached = vec![false; signals];
    reached[..n].fill(true);
    let mut reads: Vec<Vec<u32>> = vec![vec![]; signals];
    let mut readers: Vec<Vec<u32>> = vec![vec![]; signals];
    for (g, (_, ins)) in net.gates.iter().enumerate() {
        let mut r: Vec<u32> = ins.iter().copied().filter(|&i| reached[i as usize]).collect();
        r.sort_unstable();
        r.dedup();
        reached[n + g] = !r.is_empty();
        for &i in &r {
            readers[i as usize].push((n + g) as u32);
        }
        reads[n + g] = r;
    }
    let mut output = vec![false; signals];
    for &o in net.outputs.iter().flatten() {
        output[o as usize] = reached[o as usize];
    }
    let gates: Vec<usize> = (n..signals).filter(|&g| reached[g]).collect();
    let mut early = vec![1u32; signals];
    for &g in &gates {
        early[g] = 1 + reads[g].iter().map(|&p| early[p as usize]).max().unwrap();
    }
    let longest = (0..signals).filter(|&s| output[s]).map(|s| early[s]).max().unwrap_or(1);
    let last = ticks.unwrap_or(longest);
    if last < longest {
        return Err(format!("the longest path is {longest} ticks, so it cannot be balanced at {last}"));
    }
    let mut late = early.clone();
    for &g in gates.iter().rev() {
        late[g] = readers[g].iter().map(|&r| late[r as usize] - 1).min().unwrap_or(last).min(last);
    }
    let mut best: Option<(i64, Vec<u32>)> = None;
    for (k, start) in [early, late].into_iter().enumerate() {
        let mut levels = Levels::new(&reads, &readers, &output, last, start);
        levels.settle(&gates);
        let mut found = (levels.total(), levels.level.clone());
        if !gates.is_empty() && found.0 > 0 {
            let shaken = levels.shake(&gates, (MOVES_PER_GATE * gates.len()).min(MOVES_AT_MOST), k as u64);
            levels = Levels::new(&reads, &readers, &output, last, shaken.1);
            levels.settle(&gates);
            found = (levels.total(), levels.level.clone());
        }
        if best.as_ref().is_none_or(|b| found.0 < b.0) {
            best = Some(found);
        }
    }
    let level = best.unwrap().1;

    // a delay of `ticks` behind signal s: one timer, or a row of them if it is longer than a timer goes
    let mut timers: Vec<(u32, u32)> = vec![];
    let mut delayed = |s: u32, mut ticks: u32| -> u32 {
        let mut from = s;
        while ticks > 0 {
            let part = ticks.min(MAX_TIMER + 1);
            timers.push((from, part - 1));
            from = (signals + timers.len() - 1) as u32;
            ticks -= part;
        }
        from
    };
    for s in 0..signals {
        let mut by_delay: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for &r in &readers[s] {
            let wait = level[r as usize] - level[s] - 1;
            if wait > 0 {
                by_delay.entry(wait).or_default().push(r);
            }
        }
        for (wait, rs) in by_delay {
            for group in rs.chunks(MAX_CONNECTIONS) {
                let t = delayed(s as u32, wait);
                for &r in group {
                    net.gates[r as usize - n].1.iter_mut().filter(|i| **i as usize == s).for_each(|i| *i = t);
                }
            }
        }
        if output[s] && level[s] < last {
            let t = delayed(s as u32, last - level[s]);
            net.outputs.iter_mut().flatten().filter(|o| **o as usize == s).for_each(|o| *o = t);
        }
    }
    net.timers = timers;
    Ok(net)
}
