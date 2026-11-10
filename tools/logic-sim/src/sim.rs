//! Tick-accurate simulator that runs many independent copies of the circuit at once.
//!
//! Every element's state is a row of bits, one per copy ("lane"). A gate is then a few word-wide AND/OR/XOR
//! operations, so 64 test cases cost about the same as one. Only elements whose inputs changed in the last tick
//! are looked at again.
//!
//! Model (the same as both Python simulators; not yet checked against the game):
//! - Every part reads its inputs' state from the previous tick, so each part adds 1 tick.
//! - A gate with no inputs is off. XOR is on for an odd number of inputs on.
//! - A timer outputs its input `delay` ticks later, plus the 1 tick every part has.
//! - Forced elements (inputs, switches) take the forced value on the next tick.
//! - Feedback loops are fine; that is how memory and clocks are made.

use crate::circuit::{Circuit, Kind};

pub struct Sim<'a> {
    c: &'a Circuit,
    /// Words per element: lanes = 64 * w.
    pub w: usize,
    state: Vec<u64>,
    forced_mask: Vec<u64>,
    forced_val: Vec<u64>,
    readers: Vec<Vec<u32>>,
    dirty: Vec<bool>,
    queue: Vec<u32>,
    timers: Vec<u32>,
    rings: Vec<Vec<u64>>,
    max_delay: u32,
    scratch: Vec<u64>,
    pub tick_count: u64,
}

impl<'a> Sim<'a> {
    pub fn new(c: &'a Circuit, lanes: usize) -> Sim<'a> {
        let w = lanes.div_ceil(64).max(1);
        let n = c.len();
        let mut state = vec![0u64; n * w];
        let mut rings = vec![vec![]; n];
        let mut timers = vec![];
        let mut max_delay = 0;
        for e in 0..n {
            if c.initial[e] {
                state[e * w..(e + 1) * w].fill(!0);
            }
            if let Kind::Timer(d) = c.kind[e] {
                rings[e] = vec![0u64; (d as usize + 1) * w];
                timers.push(e as u32);
                max_delay = max_delay.max(d);
            }
        }
        Sim {
            c,
            w,
            state,
            forced_mask: vec![0; n * w],
            forced_val: vec![0; n * w],
            readers: c.readers(),
            dirty: vec![true; n],
            queue: (0..n as u32).collect(),
            timers,
            rings,
            max_delay,
            scratch: vec![],
            tick_count: 0,
        }
    }

    fn touch(&mut self, e: u32) {
        if !self.dirty[e as usize] {
            self.dirty[e as usize] = true;
            self.queue.push(e);
        }
    }

    /// Drive an element from outside, one bit per lane, like a switch. It shows the value after the next tick.
    pub fn force(&mut self, e: u32, values: &[u64]) {
        let (a, w) = (e as usize * self.w, self.w);
        self.forced_mask[a..a + w].fill(!0);
        self.forced_val[a..a + w].copy_from_slice(values);
        self.touch(e);
    }

    /// The same value in every lane.
    pub fn force_all(&mut self, e: u32, on: bool) {
        let v = vec![if on { !0u64 } else { 0 }; self.w];
        self.force(e, &v);
    }

    pub fn release(&mut self, e: u32) {
        let (a, w) = (e as usize * self.w, self.w);
        self.forced_mask[a..a + w].fill(0);
        self.touch(e);
    }

    pub fn get(&self, e: u32) -> &[u64] {
        &self.state[e as usize * self.w..(e as usize + 1) * self.w]
    }

    /// One game tick. Returns true if anything changed.
    pub fn tick(&mut self) -> bool {
        self.tick_count += 1;
        let w = self.w;
        let c = self.c;
        for i in 0..self.timers.len() {
            self.touch(self.timers[i]);
        }
        let list = std::mem::take(&mut self.queue);
        let mut next = std::mem::take(&mut self.scratch);
        next.clear();
        next.resize(list.len() * w, 0);
        for (k, &e) in list.iter().enumerate() {
            let e = e as usize;
            self.dirty[e] = false;
            let out = &mut next[k * w..(k + 1) * w];
            let ins = &c.ins[e];
            match c.kind[e] {
                Kind::Gate(mode) => {
                    if let Some((&first, rest)) = ins.split_first() {
                        out.copy_from_slice(&self.state[first as usize * w..(first as usize + 1) * w]);
                        for &i in rest {
                            let src = &self.state[i as usize * w..(i as usize + 1) * w];
                            match mode % 3 {
                                0 => out.iter_mut().zip(src).for_each(|(o, s)| *o &= s),
                                1 => out.iter_mut().zip(src).for_each(|(o, s)| *o |= s),
                                _ => out.iter_mut().zip(src).for_each(|(o, s)| *o ^= s),
                            }
                        }
                        if mode >= 3 {
                            out.iter_mut().for_each(|o| *o = !*o);
                        }
                    }
                }
                Kind::Timer(d) => {
                    let slots = d as u64 + 1;
                    let ring = &mut self.rings[e];
                    let write = (self.tick_count % slots) as usize * w;
                    ring[write..write + w].fill(0);
                    for &i in ins {
                        let src = &self.state[i as usize * w..(i as usize + 1) * w];
                        ring[write..write + w].iter_mut().zip(src).for_each(|(o, s)| *o |= s);
                    }
                    let read = ((self.tick_count + 1) % slots) as usize * w;
                    out.copy_from_slice(&ring[read..read + w]);
                }
                Kind::Lamp => {
                    for &i in ins {
                        let src = &self.state[i as usize * w..(i as usize + 1) * w];
                        out.iter_mut().zip(src).for_each(|(o, s)| *o |= s);
                    }
                }
                Kind::Source => out.copy_from_slice(&self.state[e * w..(e + 1) * w]),
            }
            for q in 0..w {
                let m = self.forced_mask[e * w + q];
                out[q] = (out[q] & !m) | (self.forced_val[e * w + q] & m);
            }
        }
        let mut changed = false;
        for (k, &e) in list.iter().enumerate() {
            let new = &next[k * w..(k + 1) * w];
            let old = &mut self.state[e as usize * w..(e as usize + 1) * w];
            if old != new {
                old.copy_from_slice(new);
                changed = true;
                for r in 0..self.readers[e as usize].len() {
                    self.touch(self.readers[e as usize][r]);
                }
            }
        }
        self.scratch = next;
        changed
    }

    /// Tick until nothing changes. Returns the ticks it took, or None if it never settles (a clock).
    pub fn settle(&mut self, max_ticks: u64) -> Option<u64> {
        // a timer can hide a change for its whole delay, so wait that long for quiet
        let quiet_needed = self.max_delay as u64 + 2;
        let mut quiet = 0;
        for t in 0..max_ticks {
            quiet = if self.tick() { 0 } else { quiet + 1 };
            if quiet >= quiet_needed {
                return Some(t + 1 - quiet_needed);
            }
        }
        None
    }
}
