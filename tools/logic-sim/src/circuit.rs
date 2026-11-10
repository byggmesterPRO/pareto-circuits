//! The circuit model, and loading it from a blueprint.json, a build slot or a netlist.

use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub const LOGIC_GATE: &str = "9f0f56e8-2c31-4d83-996c-d00a9b296c3f";
pub const TIMER: &str = "8f7fd0e7-c46e-4944-a414-7ce2437bb30f";
pub const SWITCH: &str = "7cf717d7-d167-4f2d-a6e7-6b2c70aa3986";
pub const BUTTON: &str = "1e8d93a4-506b-470d-9ada-9c0a321e2db5";
pub const TICKS_PER_SECOND: u32 = 40;
/// The longest setting a timer takes: 59 seconds and 39 ticks.
pub const MAX_TIMER: u32 = 60 * TICKS_PER_SECOND - 1;
/// The game's limit on wires into or out of one part.
pub const MAX_CONNECTIONS: usize = 255;

/// Gate modes are the game's: 0 and, 1 or, 2 xor, 3 nand, 4 nor, 5 xnor.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Gate(u8),
    /// Delay in ticks, on top of the 1 tick every part has.
    Timer(u32),
    /// A switch, a button, or anything else that is driven from outside. Keeps its state until forced.
    Source,
    /// On when any input is on.
    Lamp,
}

pub struct Circuit {
    pub kind: Vec<Kind>,
    /// For each element, the elements it reads.
    pub ins: Vec<Vec<u32>>,
    /// Block position of each element; empty when the circuit came from a netlist.
    pub pos: Vec<[i32; 3]>,
    /// Controller id in the blueprint (the element index for a netlist).
    pub id: Vec<i64>,
    /// State at power-on (a switch that was saved switched on).
    pub initial: Vec<bool>,
    /// The element behind each input bit, least significant first.
    pub inputs: Vec<u32>,
    /// The element behind each output bit. None is a bit that is always 0 and has no gate.
    pub outputs: Vec<Option<u32>>,
}

/// The shortest and the longest path from an input to an output, in ticks. Balanced when they are the same.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Balance {
    pub balanced: bool,
    pub shortest: u32,
    pub longest: u32,
}

impl Circuit {
    pub fn len(&self) -> usize {
        self.kind.len()
    }

    pub fn readers(&self) -> Vec<Vec<u32>> {
        let mut r = vec![vec![]; self.len()];
        for (e, ins) in self.ins.iter().enumerate() {
            for &i in ins {
                r[i as usize].push(e as u32);
            }
        }
        r
    }

    /// Every logic gate, input and output gates included.
    pub fn gates(&self) -> usize {
        self.kind.iter().filter(|k| matches!(k, Kind::Gate(_))).count()
    }

    /// Elements in an order where each comes after everything it reads. None if there is a feedback loop.
    pub fn order(&self) -> Option<Vec<u32>> {
        let readers = self.readers();
        let mut left: Vec<usize> = self.ins.iter().map(|i| i.len()).collect();
        let mut order: Vec<u32> = (0..self.len() as u32).filter(|&e| left[e as usize] == 0).collect();
        let mut k = 0;
        while k < order.len() {
            for &r in &readers[order[k] as usize] {
                left[r as usize] -= 1;
                if left[r as usize] == 0 {
                    order.push(r);
                }
            }
            k += 1;
        }
        if order.len() == self.len() { Some(order) } else { None }
    }

    /// True for a circuit of gates only, without feedback: its outputs depend on the inputs alone.
    pub fn combinational(&self) -> bool {
        self.order().is_some() && self.kind.iter().all(|k| matches!(k, Kind::Gate(_)))
    }

    /// True for a circuit of gates and timers without feedback. A timer only delays, so once the inputs have
    /// been steady for long enough the outputs still depend on the inputs alone.
    pub fn feed_forward(&self) -> bool {
        self.order().is_some() && self.kind.iter().all(|k| matches!(k, Kind::Gate(_) | Kind::Timer(_)))
    }

    /// Timers in the circuit.
    pub fn timers(&self) -> usize {
        self.kind.iter().filter(|k| matches!(k, Kind::Timer(_))).count()
    }

    /// Ticks a part adds to a path through it: 1, and for a timer its setting on top of that. So a timer set
    /// to d stands for d + 1 gates in a row.
    pub fn delay(&self, e: usize) -> u32 {
        match self.kind[e] {
            Kind::Timer(d) => d + 1,
            _ => 1,
        }
    }

    /// Gates on the longest path up to and including each element (an input gate is 1).
    pub fn depths(&self) -> Option<Vec<u32>> {
        let order = self.order()?;
        let mut d = vec![0u32; self.len()];
        for &e in &order {
            d[e as usize] = 1 + self.ins[e as usize].iter().map(|&i| d[i as usize]).max().unwrap_or(0);
        }
        Some(d)
    }

    /// For each element, the ticks of the shortest and of the longest path from an input pin up to and
    /// including it (an input gate is 1, a timer counts its setting + 1). None for an element that no input
    /// reaches: a constant, which has no timing. None overall if there is a feedback loop.
    pub fn arrivals(&self) -> Option<Vec<Option<(u32, u32)>>> {
        let order = self.order()?;
        let mut a: Vec<Option<(u32, u32)>> = vec![None; self.len()];
        for &i in &self.inputs {
            a[i as usize] = Some((1, 1));
        }
        for &e in &order {
            let e = e as usize;
            if a[e].is_some() {
                continue;
            }
            let reached = self.ins[e].iter().filter_map(|&i| a[i as usize]);
            let (lo, hi) = reached.fold((u32::MAX, 0), |(lo, hi), (l, h)| (lo.min(l), hi.max(h)));
            if hi > 0 {
                a[e] = Some((lo + self.delay(e), hi + self.delay(e)));
            }
        }
        Some(a)
    }

    /// Ticks as the repo README counts them: gates on the longest path from an input to an output, both ends
    /// included, with a timer counted as its setting + 1 gates. None for a circuit with memory, where the
    /// family README defines ticks.
    pub fn ticks(&self) -> Option<u32> {
        if !self.feed_forward() {
            return None;
        }
        let a = self.arrivals()?;
        self.outputs.iter().flatten().filter_map(|&o| a[o as usize]).map(|(_, hi)| hi).max()
    }

    /// Is the circuit balanced: is every path from an input to an output the same number of ticks. Then all
    /// outputs show the result of the same inputs at the same tick, and the inputs can change every tick.
    /// None for a circuit with memory, or one where no input reaches an output.
    pub fn balance(&self) -> Option<Balance> {
        if !self.feed_forward() {
            return None;
        }
        let a = self.arrivals()?;
        let outs: Vec<(u32, u32)> = self.outputs.iter().flatten().filter_map(|&o| a[o as usize]).collect();
        let shortest = outs.iter().map(|o| o.0).min()?;
        let longest = outs.iter().map(|o| o.1).max()?;
        Some(Balance { balanced: shortest == longest, shortest, longest })
    }

    pub fn from_netlist(net: &Value) -> Result<Circuit, String> {
        let n = net["inputs"].as_u64().ok_or("netlist: no \"inputs\"")? as usize;
        let gates = net["gates"].as_array().ok_or("netlist: no \"gates\"")?;
        let mut c = Circuit {
            kind: vec![Kind::Gate(1); n],
            ins: vec![vec![]; n],
            pos: vec![],
            id: (0..(n + gates.len()) as i64).collect(),
            initial: vec![false; n + gates.len()],
            inputs: (0..n as u32).collect(),
            outputs: vec![],
        };
        for (g, gate) in gates.iter().enumerate() {
            let mode = gate[0].as_u64().filter(|&m| m < 6).ok_or(format!("netlist: gate {g} has a bad mode"))?;
            let mut ins = vec![];
            for s in gate[1].as_array().ok_or(format!("netlist: gate {g} has no input list"))? {
                let s = s.as_u64().ok_or("netlist: a signal is not a number")? as usize;
                if s >= n + gates.len() + net.get("timers").and_then(|t| t.as_array()).map_or(0, |t| t.len()) {
                    return Err(format!("netlist: gate {g} reads signal {s}, which does not exist"));
                }
                ins.push(s as u32);
            }
            c.kind.push(Kind::Gate(mode as u8));
            c.ins.push(ins);
        }
        // timers come after the gates: signal inputs + gates + t is timer t, [signal it reads, setting]
        let timers = net.get("timers").and_then(|t| t.as_array()).map(|t| t.as_slice()).unwrap_or(&[]);
        let total = n + gates.len() + timers.len();
        for (t, timer) in timers.iter().enumerate() {
            let src = timer[0].as_u64().filter(|&s| (s as usize) < total).ok_or(format!("netlist: timer {t} reads a signal that does not exist"))?;
            let d = timer[1].as_u64().filter(|&d| d <= MAX_TIMER as u64).ok_or(format!("netlist: timer {t} needs a setting of 0 to {MAX_TIMER} ticks"))?;
            c.kind.push(Kind::Timer(d as u32));
            c.ins.push(vec![src as u32]);
            c.id.push(c.id.len() as i64);
            c.initial.push(false);
        }
        for o in net["outputs"].as_array().ok_or("netlist: no \"outputs\"")? {
            c.outputs.push(match o.as_u64() {
                Some(s) if (s as usize) < c.len() => Some(s as u32),
                Some(s) => return Err(format!("netlist: output signal {s} does not exist")),
                None => None,
            });
        }
        Ok(c)
    }

    /// pins is {"inputs": [controller id], "outputs": [controller id or null]}, as in a slot's meta.json.
    pub fn from_blueprint(bp: &Value, pins: Option<&Value>) -> Result<Circuit, String> {
        let mut c = Circuit { kind: vec![], ins: vec![], pos: vec![], id: vec![], initial: vec![], inputs: vec![], outputs: vec![] };
        let mut index: HashMap<i64, u32> = HashMap::new();
        let mut wires: Vec<(i64, i64)> = vec![];
        for body in bp["bodies"].as_array().ok_or("blueprint: no \"bodies\"")? {
            for part in body["childs"].as_array().ok_or("blueprint: a body has no \"childs\"")? {
                let ctrl = &part["controller"];
                let Some(id) = ctrl["id"].as_i64() else { continue };
                let shape = part["shapeId"].as_str().unwrap_or("");
                let kind = if shape == LOGIC_GATE {
                    Kind::Gate(ctrl["mode"].as_u64().filter(|&m| m < 6).ok_or(format!("gate {id} has a bad mode"))? as u8)
                } else if shape == TIMER {
                    Kind::Timer(ctrl["seconds"].as_u64().unwrap_or(0) as u32 * TICKS_PER_SECOND + ctrl["ticks"].as_u64().unwrap_or(0) as u32)
                } else if ctrl.get("luminance").is_some() {
                    Kind::Lamp
                } else {
                    Kind::Source
                };
                if index.insert(id, c.len() as u32).is_some() {
                    return Err(format!("blueprint: controller id {id} is used twice"));
                }
                c.kind.push(kind);
                c.ins.push(vec![]);
                c.id.push(id);
                c.initial.push(shape == SWITCH && ctrl["active"].as_bool().unwrap_or(false));
                let p = &part["pos"];
                c.pos.push([p["x"].as_i64().unwrap_or(0) as i32, p["y"].as_i64().unwrap_or(0) as i32, p["z"].as_i64().unwrap_or(0) as i32]);
                for t in ctrl["controllers"].as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
                    if let Some(to) = t["id"].as_i64() {
                        wires.push((id, to));
                    }
                }
            }
        }
        for (from, to) in wires {
            // wires to parts that are not logic parts of this blueprint are ignored
            if let Some(&t) = index.get(&to) {
                c.ins[t as usize].push(index[&from]);
            }
        }
        if let Some(pins) = pins {
            for (i, p) in pins["inputs"].as_array().ok_or("pins: no \"inputs\"")?.iter().enumerate() {
                let id = p.as_i64().ok_or("pins: an input is not a controller id")?;
                let e = *index.get(&id).ok_or(format!("pins: input {i} is controller {id}, which is not in the blueprint"))?;
                c.inputs.push(e);
            }
            for (i, p) in pins["outputs"].as_array().ok_or("pins: no \"outputs\"")?.iter().enumerate() {
                c.outputs.push(match p.as_i64() {
                    Some(id) => Some(*index.get(&id).ok_or(format!("pins: output {i} is controller {id}, which is not in the blueprint"))?),
                    None => None,
                });
            }
        }
        Ok(c)
    }
}

pub fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Pins as given by a caller: {"inputs": [..], "outputs": [..]}, a meta.json that holds "pins", or a path to
/// either.
pub fn read_pins(v: &Value) -> Result<Value, String> {
    let v = match v.as_str() {
        Some(path) => read_json(Path::new(path))?,
        None => v.clone(),
    };
    Ok(v.get("pins").cloned().unwrap_or(v))
}

/// A circuit from a JSON value: a blueprint ("bodies") or a netlist ("inputs", "gates", "outputs").
pub fn from_value(v: &Value, pins: Option<&Value>) -> Result<Circuit, String> {
    if v.get("bodies").is_some() {
        Circuit::from_blueprint(v, pins)
    } else {
        Circuit::from_netlist(v)
    }
}

/// A build slot (a folder with blueprint/blueprint.json and meta.json), a blueprint.json or a netlist.json.
/// A slot brings its own pins and meta.json; a bare blueprint needs `pins` for anything that uses inputs and
/// outputs. Returns the circuit and, for a slot, its meta.json.
pub fn load(path: &Path, pins: Option<&Value>) -> Result<(Circuit, Option<Value>), String> {
    if path.is_dir() {
        let meta = read_json(&path.join("meta.json"))?;
        let bp = read_json(&path.join("blueprint").join("blueprint.json"))?;
        let c = Circuit::from_blueprint(&bp, Some(pins.or(meta.get("pins")).ok_or("meta.json has no \"pins\"")?))?;
        return Ok((c, Some(meta)));
    }
    Ok((from_value(&read_json(path)?, pins)?, None))
}
