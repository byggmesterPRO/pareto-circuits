use logic_sim::check::check;
use logic_sim::circuit::Circuit;
use logic_sim::families::family;
use logic_sim::script;
use logic_sim::sim::Sim;
use serde_json::{json, Value};

const GATE: &str = "9f0f56e8-2c31-4d83-996c-d00a9b296c3f";
const TIMER: &str = "8f7fd0e7-c46e-4944-a414-7ce2437bb30f";

fn gate(id: i64, mode: u8, to: &[i64], x: i64) -> Value {
    json!({"color": "222222", "pos": {"x": x, "y": 0, "z": 0}, "shapeId": GATE,
           "controller": {"id": id, "mode": mode, "controllers": to.iter().map(|t| json!({"id": t})).collect::<Vec<_>>()}})
}

fn blueprint(parts: Vec<Value>) -> Value {
    json!({"bodies": [{"childs": parts}], "version": 4})
}

fn on(sim: &Sim, e: u32) -> bool {
    sim.get(e)[0] & 1 == 1
}

#[test]
fn half_adder_every_input() {
    let net = json!({"inputs": 2, "gates": [[2, [0, 1]], [0, [0, 1]]], "outputs": [2, 3]});
    let c = Circuit::from_netlist(&net).unwrap();
    assert_eq!((c.gates(), c.ticks()), (4, Some(2)));
    let res = check(&c, &|x| ((x & 1) ^ (x >> 1)) as u128 | (((x & 1) & (x >> 1)) as u128) << 1, 2).unwrap();
    assert!(res.exhaustive && res.tested == 4 && res.wrong == 0);
    // a wrong reference: inputs 2 and 3 now disagree
    let res = check(&c, &|x| (x & 1) as u128, 2).unwrap();
    assert_eq!(res.wrong, 2);
}

#[test]
fn a_gate_with_no_inputs_is_off() {
    let c = Circuit::from_netlist(&json!({"inputs": 1, "gates": [[4, []], [3, []]], "outputs": [1, 2]})).unwrap();
    let mut sim = Sim::new(&c, 1);
    sim.settle(100).unwrap();
    assert!(!on(&sim, 1) && !on(&sim, 2));
}

#[test]
fn nor_latch_remembers() {
    // 0 = S, 1 = R, 2 = Q = NOR(R, Qn), 3 = Qn = NOR(S, Q)
    let bp = blueprint(vec![gate(0, 1, &[3], 0), gate(1, 1, &[2], 1), gate(2, 4, &[3], 2), gate(3, 4, &[2], 3)]);
    let pins = json!({"inputs": [0, 1], "outputs": [2, 3]});
    let c = Circuit::from_blueprint(&bp, Some(&pins)).unwrap();
    assert_eq!(c.ticks(), None);
    let run = script::run(&c, "set in0 1\ntick 4\ncheck out0 1\nset in0 0\ntick 4\ncheck out0 1   // still set\nset in1 1\ntick 4\ncheck out0 0\nset in1 0\ntick 4\ncheck out0 0\ncheck out1 1").unwrap();
    assert_eq!((run.correct, run.total), (5, 5));
}

#[test]
fn timer_delays_by_its_ticks_plus_one() {
    // 0 = input gate -> 1 = timer of 3 ticks
    let mut parts = vec![gate(0, 1, &[1], 0)];
    parts.push(json!({"color": "222222", "pos": {"x": 1, "y": 0, "z": 0}, "shapeId": TIMER,
                      "controller": {"id": 1, "seconds": 0, "ticks": 3, "controllers": null}}));
    let c = Circuit::from_blueprint(&blueprint(parts), None).unwrap();
    let mut sim = Sim::new(&c, 1);
    sim.force_all(0, true);
    let seen: Vec<bool> = (0..6).map(|_| { sim.tick(); on(&sim, 1) }).collect();
    // tick 1: the input gate turns on. The timer shows it 3 + 1 ticks later, at tick 5.
    assert_eq!(seen, [false, false, false, false, true, true]);
}

#[test]
fn lanes_are_independent() {
    let c = Circuit::from_netlist(&json!({"inputs": 2, "gates": [[2, [0, 1]]], "outputs": [2]})).unwrap();
    let mut sim = Sim::new(&c, 128);
    sim.force(0, &[0xAAAA_AAAA_AAAA_AAAA, 0x1]);
    sim.force(1, &[0xCCCC_CCCC_CCCC_CCCC, 0x3]);
    sim.tick();
    sim.tick();
    assert_eq!(sim.get(2), &[0x6666_6666_6666_6666, 0x2]);
}

#[test]
fn double_dabble_function() {
    let f = family("double-dabble", "12bit").unwrap();
    assert_eq!((f.bits, f.out_bits), (12, 16));
    assert_eq!((f.f)(4095), 0x4095);
    let f = family("double-dabble", "64bit").unwrap();
    assert_eq!(f.out_bits, 80);
    assert_eq!((f.f)(u64::MAX), 0x18446744073709551615);
}
