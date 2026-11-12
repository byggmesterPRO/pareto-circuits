//! Building circuits: a netlist type, a builder that folds constants and shares equal gates, and a writer
//! for blueprint.json. Generators use this.
//!
//! A netlist has `inputs` input bits (signal i < inputs is input bit i), then gates (signal inputs + g is
//! gate g). Without memory the gates are in feed-forward order. A circuit with memory has gates that read
//! later gates, or themselves. Gate modes are the game's: 0 and, 1 or, 2 xor, 3 nand, 4 nor, 5 xnor.
//! An output is a signal, or None for a bit that is always 0 and has no gate.
//!
//! A netlist can also hold timers, which `balance` adds to even out the paths. They come after the gates:
//! signal inputs + gates + t is timer t.

use crate::circuit::{Circuit, Kind, LOGIC_GATE, MAX_CONNECTIONS, TICKS_PER_SECOND, TIMER};
use serde_json::{json, Value};
use std::collections::HashMap;

pub const AND: u8 = 0;
pub const OR: u8 = 1;
pub const XOR: u8 = 2;
pub const NAND: u8 = 3;
pub const NOR: u8 = 4;
pub const XNOR: u8 = 5;

pub const INPUT_COLOR: &str = "E8C547";
pub const OUTPUT_COLOR: &str = "19E753";
pub const GATE_COLOR: &str = "222222";
pub const TIMER_COLOR: &str = "DF7F00";

/// A signal while building: a wire, or a constant that needs no gate.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum S {
    Zero,
    One,
    Wire(u32),
}

#[derive(Clone, Debug)]
pub struct Netlist {
    pub inputs: usize,
    pub gates: Vec<(u8, Vec<u32>)>,
    /// Timers, as (the signal it reads, its setting in ticks). A timer set to d passes its input on d + 1
    /// ticks later, like d + 1 gates in a row. Generators leave this empty.
    pub timers: Vec<(u32, u32)>,
    pub outputs: Vec<Option<u32>>,
    /// Output gates per row in the blueprint; 0 puts them all in one row.
    pub grid: usize,
}

pub struct Builder {
    inputs: usize,
    gates: Vec<(u8, Vec<u32>)>,
    memo: HashMap<(u8, Vec<u32>), u32>,
}

impl Builder {
    pub fn new(inputs: usize) -> Builder {
        Builder { inputs, gates: vec![], memo: HashMap::new() }
    }

    pub fn input(&self, i: usize) -> S {
        assert!(i < self.inputs);
        S::Wire(i as u32)
    }

    fn raw(&mut self, mode: u8, mut ins: Vec<u32>) -> S {
        ins.sort_unstable();
        let next = (self.inputs + self.gates.len()) as u32;
        let gates = &mut self.gates;
        S::Wire(*self.memo.entry((mode, ins.clone())).or_insert_with(|| {
            gates.push((mode, ins));
            next
        }))
    }

    /// One gate of the given mode over the signals, or less: constants are folded away, a signal given twice
    /// counts once (and cancels out in an xor), and a gate that would just copy its input is left out.
    pub fn gate(&mut self, mode: u8, xs: &[S]) -> S {
        let (base, mut invert) = (mode % 3, mode >= 3);
        let konst = |on: bool| if on { S::One } else { S::Zero };
        if base == XOR {
            invert ^= xs.iter().filter(|&&x| x == S::One).count() % 2 == 1;
            let mut wires: Vec<u32> = xs.iter().filter_map(|x| if let S::Wire(w) = x { Some(*w) } else { None }).collect();
            wires.sort_unstable();
            let mut odd = vec![];
            for w in wires {
                if odd.last() == Some(&w) {
                    odd.pop();
                } else {
                    odd.push(w);
                }
            }
            return match (odd.len(), invert) {
                (0, _) => konst(invert),
                (1, false) => S::Wire(odd[0]),
                _ => self.raw(if invert { XNOR } else { XOR }, odd),
            };
        }
        // and: a 0 decides it, a 1 changes nothing. or: the other way round.
        let (decides, neutral) = if base == AND { (S::Zero, S::One) } else { (S::One, S::Zero) };
        if xs.contains(&decides) {
            return konst((decides == S::One) != invert);
        }
        let mut wires: Vec<u32> = xs.iter().filter_map(|x| if let S::Wire(w) = x { Some(*w) } else { None }).collect();
        wires.sort_unstable();
        wires.dedup();
        match (wires.len(), invert) {
            (0, _) => konst((neutral == S::One) != invert),
            (1, false) => S::Wire(wires[0]),
            _ => self.raw(base + if invert { 3 } else { 0 }, wires),
        }
    }

    /// A gate whose inputs are given afterwards with `wire`, for loops: a gate that reads itself, or a gate
    /// further on, is how memory is made. It is never shared with another gate.
    pub fn later(&mut self, mode: u8) -> S {
        self.gates.push((mode, vec![]));
        S::Wire((self.inputs + self.gates.len() - 1) as u32)
    }

    /// Give a gate made with `later` its inputs. Constants are not allowed here.
    pub fn wire(&mut self, gate: S, ins: &[S]) {
        let S::Wire(g) = gate else { panic!("wire: not a gate") };
        let ins: Vec<u32> = ins.iter().map(|x| if let S::Wire(w) = x { *w } else { panic!("wire: a constant input") }).collect();
        self.gates[g as usize - self.inputs].1 = ins;
    }

    /// A gate that only copies its input, so that an output has a gate of its own. A constant stays a constant.
    pub fn buffer(&mut self, x: S) -> S {
        match x {
            S::Wire(w) => self.raw(OR, vec![w]),
            constant => constant,
        }
    }

    pub fn and(&mut self, xs: &[S]) -> S {
        self.gate(AND, xs)
    }
    pub fn or(&mut self, xs: &[S]) -> S {
        self.gate(OR, xs)
    }
    pub fn xor(&mut self, xs: &[S]) -> S {
        self.gate(XOR, xs)
    }
    pub fn nand(&mut self, xs: &[S]) -> S {
        self.gate(NAND, xs)
    }
    pub fn nor(&mut self, xs: &[S]) -> S {
        self.gate(NOR, xs)
    }
    pub fn xnor(&mut self, xs: &[S]) -> S {
        self.gate(XNOR, xs)
    }
    pub fn not(&mut self, x: S) -> S {
        self.gate(NOR, &[x])
    }

    /// The finished netlist: gates nothing reads are dropped, and no part has more than 255 wires.
    pub fn finish(self, outputs: &[S]) -> Result<Netlist, String> {
        let mut outs = vec![];
        for (b, o) in outputs.iter().enumerate() {
            outs.push(match o {
                S::Zero => None,
                S::Wire(w) => Some(*w),
                S::One => return Err(format!("output bit {b} is always 1, which has no gate to stand for it")),
            });
        }
        let mut net = Netlist { inputs: self.inputs, gates: self.gates, timers: vec![], outputs: outs, grid: 0 };
        net.prune();
        net.split_fan_out()?;
        Ok(net)
    }
}

impl Netlist {
    pub fn from_json(v: &Value) -> Result<Netlist, String> {
        Netlist::from_circuit(&Circuit::from_netlist(v)?)
    }

    /// A circuit as a netlist: its input pins become the input bits, and the gates and timers keep their
    /// order (gates first). Fails if there are other parts than gates and timers, or no pins.
    pub fn from_circuit(c: &Circuit) -> Result<Netlist, String> {
        let n = c.inputs.len();
        if n == 0 {
            return Err("a netlist needs to know the inputs and outputs: give a build slot, a netlist, or pins".into());
        }
        let mut sig = vec![u32::MAX; c.len()];
        for (i, &e) in c.inputs.iter().enumerate() {
            if sig[e as usize] != u32::MAX {
                return Err(format!("input {i} is the same part as an earlier input"));
            }
            sig[e as usize] = i as u32;
        }
        // gates keep their order if that is feed-forward already, and are put in such an order if not
        let mut gates: Vec<usize> = (0..c.len()).filter(|&e| sig[e] == u32::MAX && matches!(c.kind[e], Kind::Gate(_))).collect();
        let timers: Vec<usize> = (0..c.len()).filter(|&e| matches!(c.kind[e], Kind::Timer(_))).collect();
        if n + gates.len() + timers.len() != c.len() {
            return Err("only gates and timers fit in a netlist".into());
        }
        let mut seen: Vec<bool> = sig.iter().map(|&s| s != u32::MAX).collect();
        let forward = gates.iter().all(|&e| {
            seen[e] = true;
            c.ins[e].iter().all(|&i| seen[i as usize])
        });
        if !forward {
            if let Some(order) = c.order() {
                gates = order.iter().map(|&e| e as usize).filter(|&e| sig[e] == u32::MAX && matches!(c.kind[e], Kind::Gate(_))).collect();
            }
        }
        for (k, &e) in gates.iter().chain(&timers).enumerate() {
            sig[e] = (n + k) as u32;
        }
        let mut net = Netlist { inputs: n, gates: vec![], timers: vec![], outputs: c.outputs.iter().map(|o| o.map(|o| sig[o as usize])).collect(), grid: 0 };
        for &e in gates.iter().chain(&timers) {
            let ins: Vec<u32> = c.ins[e].iter().map(|&i| sig[i as usize]).collect();
            match c.kind[e] {
                Kind::Gate(m) => net.gates.push((m, ins)),
                _ if ins.len() != 1 => return Err(format!("a timer with {} wires going in; a timer takes one", ins.len())),
                Kind::Timer(d) => net.timers.push((ins[0], d)),
                _ => unreachable!(),
            }
        }
        Ok(net)
    }

    /// The netlist with its timers taken out and their readers wired to what the timers read. What is left
    /// settles on the same outputs, since a timer only delays.
    pub fn without_timers(&self) -> Netlist {
        let first = (self.inputs + self.gates.len()) as u32;
        let source = |mut s: u32| {
            while s >= first {
                s = self.timers[(s - first) as usize].0;
            }
            s
        };
        let gates = self.gates.iter().map(|(m, ins)| (*m, ins.iter().map(|&i| source(i)).collect())).collect();
        Netlist { inputs: self.inputs, gates, timers: vec![], outputs: self.outputs.iter().map(|o| o.map(source)).collect(), grid: self.grid }
    }

    pub fn to_json(&self) -> Value {
        let gates = self.gates.iter().map(|(m, i)| json!([m, i])).collect::<Vec<_>>();
        if self.timers.is_empty() {
            return json!({ "inputs": self.inputs, "gates": gates, "outputs": self.outputs });
        }
        json!({ "inputs": self.inputs, "gates": gates, "timers": self.timers, "outputs": self.outputs })
    }

    pub fn circuit(&self) -> Circuit {
        Circuit::from_netlist(&self.to_json()).expect("a netlist is always a valid circuit")
    }

    fn readers(&self) -> Vec<Vec<u32>> {
        let mut r = vec![vec![]; self.inputs + self.gates.len() + self.timers.len()];
        for (g, (_, ins)) in self.gates.iter().enumerate() {
            for &i in ins {
                r[i as usize].push((self.inputs + g) as u32);
            }
        }
        for (t, (src, _)) in self.timers.iter().enumerate() {
            r[*src as usize].push((self.inputs + self.gates.len() + t) as u32);
        }
        r
    }

    /// Drop gates that no output depends on. For a netlist without timers.
    pub fn prune(&mut self) {
        assert!(self.timers.is_empty(), "prune comes before balance");
        let n = self.inputs;
        let mut live = vec![false; n + self.gates.len()];
        for o in self.outputs.iter().flatten() {
            live[*o as usize] = true;
        }
        // more than one pass only when there are loops
        loop {
            let mut grew = false;
            for g in (0..self.gates.len()).rev() {
                if live[n + g] {
                    for &i in &self.gates[g].1 {
                        grew |= !live[i as usize] && i as usize > n + g;
                        live[i as usize] = true;
                    }
                }
            }
            if !grew {
                break;
            }
        }
        let mut new = vec![u32::MAX; n + self.gates.len()];
        (0..n).for_each(|i| new[i] = i as u32);
        let mut gates = vec![];
        for (g, (mode, ins)) in self.gates.iter().enumerate() {
            if live[n + g] {
                new[n + g] = (n + gates.len()) as u32;
                gates.push((*mode, ins.clone()));
            }
        }
        for (_, ins) in gates.iter_mut() {
            ins.iter_mut().for_each(|i| *i = new[*i as usize]);
        }
        self.gates = gates;
        for o in self.outputs.iter_mut().flatten() {
            *o = new[*o as usize];
        }
    }

    /// A gate that drives more than 255 parts is doubled: copies with the same inputs share its readers. That
    /// adds gates but no ticks. An input pin that drives too many gets buffers (OR gates with one input)
    /// between it and its readers, which costs a tick on those paths. Fails if a gate has too many inputs.
    pub fn split_fan_out(&mut self) -> Result<(), String> {
        assert!(self.timers.is_empty(), "split_fan_out comes before balance");
        loop {
            let n = self.inputs;
            if let Some((g, (_, ins))) = self.gates.iter().enumerate().find(|(_, (_, ins))| ins.len() > MAX_CONNECTIONS) {
                return Err(format!("gate {g} has {} inputs, the game allows {MAX_CONNECTIONS}", ins.len()));
            }
            let readers = self.readers();
            // the last one first: doubling a gate adds readers to the gates before it
            let Some(sig) = (0..readers.len()).rev().find(|&s| readers[s].len() > MAX_CONNECTIONS) else { return Ok(()) };
            if sig < n {
                // inverters and other one-input gates stay on the pin, so they are not a tick behind the buffers
                let mut behind: Vec<u32> = readers[sig].iter().copied().filter(|&r| self.gates[r as usize - n].1.len() > 1).collect();
                if behind.is_empty() {
                    behind = readers[sig].clone();
                }
                let copies = behind.len().div_ceil(MAX_CONNECTIONS);
                for (_, gins) in self.gates.iter_mut() {
                    gins.iter_mut().filter(|i| **i as usize >= n).for_each(|i| *i += copies as u32);
                }
                for o in self.outputs.iter_mut().flatten().filter(|o| **o as usize >= n) {
                    *o += copies as u32;
                }
                for _ in 0..copies {
                    self.gates.insert(0, (OR, vec![sig as u32]));
                }
                for (k, &r) in behind.iter().enumerate() {
                    let buffer = (n + k / MAX_CONNECTIONS) as u32;
                    for i in self.gates[r as usize - n + copies].1.iter_mut().filter(|i| **i as usize == sig) {
                        *i = buffer;
                    }
                }
                continue;
            }
            let copies = readers[sig].len().div_ceil(MAX_CONNECTIONS);
            let (mode, ins) = self.gates[sig - n].clone();
            // insert the copies right after the gate, so the order stays feed-forward
            let at = sig - n + 1;
            for _ in 1..copies {
                self.gates.insert(at, (mode, ins.clone()));
            }
            let shift = (copies - 1) as u32;
            let moved = |s: u32| if s as usize >= n + at { s + shift } else { s };
            for (g, (_, gins)) in self.gates.iter_mut().enumerate() {
                if g >= at && g < at + copies - 1 {
                    continue;
                }
                for i in gins.iter_mut() {
                    *i = moved(*i);
                }
            }
            for o in self.outputs.iter_mut().flatten() {
                *o = moved(*o);
            }
            // hand the readers out over the gate and its copies
            for (k, &r) in readers[sig].iter().enumerate() {
                let copy = k / MAX_CONNECTIONS;
                if copy > 0 {
                    let reader = (moved(r) as usize) - n;
                    for i in self.gates[reader].1.iter_mut() {
                        if *i as usize == sig {
                            *i = (sig + copy) as u32;
                        }
                    }
                }
            }
        }
    }

    /// The blueprint and its pins ({"inputs": [controller id], "outputs": [controller id or null]}).
    ///
    /// Layout: a solid block `width` wide. Row 0 holds the input gates at x = bit, row 1 the output gates at
    /// x = bit (with `grid` set: `grid` to a row, in as many rows as it takes), and the other gates fill the
    /// cells that are left and then the rows behind. Parts next to each other touch, so the build is one piece
    /// as long as there are enough other gates to fill the gaps.
    ///
    /// A timer takes a cell like a gate does and stands two blocks tall in it (its box is 1 x 2 x 1, and with
    /// the axes used here the long side points up), so it is in nobody's way.
    pub fn blueprint(&self) -> (Value, Value) {
        let n = self.inputs;
        let readers = self.readers();
        let per_row = if self.grid > 0 { self.grid } else { self.outputs.len().max(1) };
        let width = n.max(per_row) as i64;
        let out_rows = self.outputs.len().div_ceil(per_row).max(1) as i64;
        let cell_of = |b: usize| ((b % per_row) as i64, 1 + (b / per_row) as i64);
        let mut out_bit: HashMap<u32, usize> = HashMap::new();
        for (b, o) in self.outputs.iter().enumerate() {
            if let Some(o) = o {
                if *o as usize >= n {
                    out_bit.entry(*o).or_insert(b);
                }
            }
        }
        // free cells for the other gates: gaps in the output row, the rest of the input row, then rows behind
        let taken: std::collections::HashSet<(i64, i64)> = out_bit.values().map(|&b| cell_of(b)).collect();
        let mut free: Vec<(i64, i64)> = (1..=out_rows).flat_map(|y| (0..width).map(move |x| (x, y))).filter(|c| !taken.contains(c)).collect();
        free.extend((n as i64..width).map(|x| (x, 0)));
        let mut cells = free.into_iter().chain((1 + out_rows..).flat_map(|y| (0..width).map(move |x| (x, y))));
        let part = |sig: usize, mode: u8, color: &str, x: i64, y: i64| {
            let to: Vec<Value> = readers[sig].iter().map(|r| json!({ "id": r })).collect();
            json!({
                "color": color,
                "controller": { "active": false, "controllers": if to.is_empty() { Value::Null } else { Value::Array(to) }, "id": sig, "joints": null, "mode": mode },
                "joints": null,
                "pos": { "x": x, "y": y, "z": 0 },
                "shapeId": LOGIC_GATE,
                "xaxis": 1,
                "zaxis": -2,
            })
        };
        let mut childs: Vec<Value> = (0..n).map(|i| part(i, OR, INPUT_COLOR, i as i64, 0)).collect();
        for (g, (mode, _)) in self.gates.iter().enumerate() {
            let sig = n + g;
            childs.push(match out_bit.get(&(sig as u32)) {
                Some(&b) => part(sig, *mode, OUTPUT_COLOR, cell_of(b).0, cell_of(b).1),
                None => {
                    let (x, y) = cells.next().unwrap();
                    part(sig, *mode, GATE_COLOR, x, y)
                }
            });
        }
        for (t, (_, setting)) in self.timers.iter().enumerate() {
            let sig = n + self.gates.len() + t;
            let (color, (x, y)) = match out_bit.get(&(sig as u32)) {
                Some(&b) => (OUTPUT_COLOR, cell_of(b)),
                None => (TIMER_COLOR, cells.next().unwrap()),
            };
            let mut timer = part(sig, 0, color, x, y);
            timer["shapeId"] = json!(TIMER);
            let ctrl = timer["controller"].as_object_mut().unwrap();
            ctrl.remove("mode");
            ctrl.insert("seconds".into(), json!(setting / TICKS_PER_SECOND));
            ctrl.insert("ticks".into(), json!(setting % TICKS_PER_SECOND));
            childs.push(timer);
        }
        let pins = json!({ "inputs": (0..n).collect::<Vec<_>>(), "outputs": self.outputs });
        (json!({ "bodies": [{ "childs": childs }], "joints": null, "version": 4 }), pins)
    }
}

/// One way of building a circuit, as a family's generator hands it over.
pub struct Design {
    pub name: String,
    pub netlist: Netlist,
}

impl Design {
    pub fn new(name: impl Into<String>, netlist: Result<Netlist, String>) -> Result<Design, String> {
        let name = name.into();
        Ok(Design { netlist: netlist.map_err(|e| format!("{name}: {e}"))?, name })
    }
}
