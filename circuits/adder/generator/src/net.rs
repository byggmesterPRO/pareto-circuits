//! A small netlist under construction for the searches: equal gates are shared, and every gate knows its
//! depth, so a search can count and time a circuit cheaply before it is handed to the `Builder`.
//!
//! Signals: input bit i is i (a is 0 .. n-1, b is n .. 2n-1), gate g is 2n + g.

use logic_sim::build::{Builder, Netlist, S};
use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

/// A quick hash for the searches' small keys (the standard one is slow for them). Nothing depends on the
/// order a map iterates in, so this changes no result.
#[derive(Default, Clone, Copy)]
pub struct Fx(u64);

impl Hasher for Fx {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for c in bytes.chunks(8) {
            let mut w = [0u8; 8];
            w[..c.len()].copy_from_slice(c);
            self.write_u64(u64::from_le_bytes(w));
        }
    }
    fn write_u64(&mut self, x: u64) {
        self.0 = (self.0.rotate_left(5) ^ x).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    fn write_u32(&mut self, x: u32) {
        self.write_u64(x as u64);
    }
    fn write_usize(&mut self, x: usize) {
        self.write_u64(x as u64);
    }
    fn write_u8(&mut self, x: u8) {
        self.write_u64(x as u64);
    }
}

pub type FxMap<K, V> = HashMap<K, V, BuildHasherDefault<Fx>>;
pub type FxSet<K> = HashSet<K, BuildHasherDefault<Fx>>;

pub struct Net {
    inputs: usize,
    pub gates: Vec<(u8, Vec<u32>)>,
    key: FxMap<(u8, Vec<u32>), u32>,
    dep: Vec<u32>,
}

impl Net {
    pub fn new(n: usize) -> Net {
        Net { inputs: 2 * n, gates: vec![], key: FxMap::default(), dep: vec![0; 2 * n] }
    }

    /// A gate over the signals, or the equal gate made before.
    pub fn g(&mut self, mode: u8, ins: &[u32]) -> u32 {
        let mut ins = ins.to_vec();
        ins.sort_unstable();
        let next = (self.inputs + self.gates.len()) as u32;
        let s = *self.key.entry((mode, ins.clone())).or_insert(next);
        if s == next {
            self.dep.push(1 + ins.iter().map(|&i| self.dep[i as usize]).max().unwrap_or(0));
            self.gates.push((mode, ins));
        }
        s
    }

    pub fn depth(&self, outs: &[u32]) -> u32 {
        outs.iter().map(|&o| self.dep[o as usize]).max().unwrap_or(0)
    }

    /// The finished netlist, made by the `Builder` (which may still fold or share a gate or two).
    pub fn finish(&self, outs: &[u32]) -> Result<Netlist, String> {
        let mut b = Builder::new(self.inputs);
        let mut sig: Vec<S> = (0..self.inputs).map(|i| b.input(i)).collect();
        for (mode, ins) in &self.gates {
            let xs: Vec<S> = ins.iter().map(|&i| sig[i as usize]).collect();
            sig.push(b.gate(*mode, &xs));
        }
        let outs: Vec<S> = outs.iter().map(|&o| sig[o as usize]).collect();
        b.finish(&outs)
    }
}

/// Ticks of a netlist without timers: gates on the longest path, the inputs not counted.
pub fn ticks(net: &Netlist) -> usize {
    let mut d = vec![0usize; net.inputs];
    for (_, ins) in &net.gates {
        d.push(1 + ins.iter().map(|&i| d[i as usize]).max().unwrap_or(0));
    }
    net.outputs.iter().flatten().map(|&o| d[o as usize]).max().unwrap_or(0)
}
