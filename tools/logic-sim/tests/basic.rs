use logic_sim::check::{check, transpose64, Expect};
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
    let res = check(&c, &Expect::Function(&|x| ((x & 1) ^ (x >> 1)) as u128 | (((x & 1) & (x >> 1)) as u128) << 1), 2, &[]).unwrap();
    assert!(res.exhaustive && res.tested == 4 && res.wrong == 0);
    // a wrong reference: inputs 2 and 3 now disagree
    let res = check(&c, &Expect::Function(&|x| (x & 1) as u128), 2, &[]).unwrap();
    assert_eq!((res.wrong, res.example), (2, Some(2u128)));
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
    assert_eq!((f.f)(u64::MAX as u128), 0x18446744073709551615);
}

#[test]
fn transpose_turns_rows_into_columns() {
    let mut s = 0x1234_5678_9ABC_DEF1u64;
    let mut a = [0u64; 64];
    for v in a.iter_mut() {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        *v = s;
    }
    let before = a;
    transpose64(&mut a);
    for r in 0..64 {
        for b in 0..64 {
            assert_eq!(a[b] >> r & 1, before[r] >> b & 1);
        }
    }
}

#[test]
fn table_cases_and_reference() {
    // 3-input majority, two ways
    let a = Circuit::from_netlist(&json!({"inputs": 3, "gates": [[0, [0, 1]], [0, [0, 2]], [0, [1, 2]], [1, [3, 4, 5]]], "outputs": [6]})).unwrap();
    let b = Circuit::from_netlist(&json!({"inputs": 3, "gates": [[1, [0, 1]], [0, [0, 1]], [0, [2, 3]], [1, [4, 5]]], "outputs": [6]})).unwrap();
    let table: Vec<u128> = (0..8u32).map(|x| (x.count_ones() >= 2) as u128).collect();
    assert_eq!(check(&a, &Expect::Table(&table), 1, &[]).unwrap().wrong, 0);
    assert_eq!(check(&a, &Expect::Circuit(&b), 4, &[]).unwrap().wrong, 0);
    assert_eq!(check(&a, &Expect::Cases(&[(3, 1), (4, 0), (7, 1), (1, 1)]), 1, &[]).unwrap().wrong, 1);
    // an xor is not a majority: they agree only on 0 and 7
    let x = Circuit::from_netlist(&json!({"inputs": 3, "gates": [[2, [0, 1, 2]]], "outputs": [3]})).unwrap();
    let res = check(&a, &Expect::Circuit(&x), 2, &[]).unwrap();
    assert_eq!((res.wrong, res.example, res.tested), (6, Some(1u128), 8));
    assert!(check(&a, &Expect::Table(&table[..4]), 1, &[]).is_err());
}

#[test]
fn blueprint_parts_in_any_order() {
    // the output gate comes first in the file, its inputs after it
    let bp = blueprint(vec![gate(9, 0, &[], 0), gate(1, 1, &[9], 1), gate(2, 1, &[9], 2)]);
    let c = Circuit::from_blueprint(&bp, Some(&json!({"inputs": [1, 2], "outputs": [9]}))).unwrap();
    assert_eq!(check(&c, &Expect::Function(&|x| (x == 3) as u128), 1, &[]).unwrap().wrong, 0);
    assert_eq!((c.gates(), c.ticks()), (3, Some(2)));
}

#[test]
fn more_than_64_output_bits() {
    // a 64-bit "pass through twice": 128 outputs wired straight to the inputs
    let outs: Vec<u64> = (0..64).chain(0..64).collect();
    let c = Circuit::from_netlist(&json!({"inputs": 64, "gates": [], "outputs": outs})).unwrap();
    let res = check(&c, &Expect::Function(&|x| x & u64::MAX as u128 | (x & u64::MAX as u128) << 64), 4, &[]).unwrap();
    assert!(!res.exhaustive && res.wrong == 0 && res.tested > 900_000);
    let res = check(&c, &Expect::Function(&|x| x & u64::MAX as u128 | ((x ^ 1) & u64::MAX as u128) << 64), 4, &[]).unwrap();
    assert_eq!(res.wrong, res.tested);
}

#[test]
fn counting_in_decimal_matches_dividing() {
    for size in ["4bit", "12bit", "32bit", "64bit"] {
        let fam = family("double-dabble", size).unwrap();
        let top = if fam.bits == 64 { u64::MAX } else { (1u64 << fam.bits) - 1 };
        for first in [0u64, 9, 95, 999_990, top.saturating_sub(63), top.saturating_sub(10)] {
            let first = first.min(top);
            let n = ((top - first).min(63) + 1) as usize;
            let (mut lo, mut hi) = ([0u64; 64], [0u64; 64]);
            (fam.run)(first, &mut lo[..n], &mut hi[..n]);
            for l in 0..n {
                assert_eq!(lo[l] as u128 | (hi[l] as u128) << 64, (fam.f)(first as u128 + l as u128), "{size} {first} {l}");
            }
        }
    }
}

#[test]
fn a_broken_gate_is_caught_on_the_fast_path() {
    // 4-bit double dabble by table, then against the family with one gate changed
    let fam = family("double-dabble", "4bit").unwrap();
    // x >= 10 -> tens bit; ones = x or x - 10
    let net = json!({"inputs": 4, "gates": [
        [1, [1, 2]], [0, [3, 4]],                   // 5 = x >= 10
        [4, [5]],                                   // 6 = not that
        [2, [1, 5]],                                // 7 = ones bit 1: x1 xor tens
        [0, [1, 2, 5]], [0, [2, 6]], [1, [8, 9]],   // 10 = ones bit 2
        [0, [3, 6]]                                 // 11 = ones bit 3
    ], "outputs": [0, 7, 10, 11, 5, null, null, null]});
    let c = Circuit::from_netlist(&net).unwrap();
    let good = check(&c, &fam.expect(), 2, &[]).unwrap();
    assert_eq!((good.wrong, good.tested, good.exhaustive), (0, 16, true));
    let mut bad = net.clone();
    bad["gates"][3][0] = json!(5);
    let c = Circuit::from_netlist(&bad).unwrap();
    let res = check(&c, &fam.expect(), 2, &[]).unwrap();
    assert_eq!((res.wrong, res.example), (16, Some(0u128)));
}

#[test]
fn adder_family_and_128_input_bits() {
    use logic_sim::build::{Builder, S};
    // ripple adder from full adders: sum = xor(a, b, c), carry = xor(sum, and(a, b, c), or(a, b, c))
    for n in [3usize, 64] {
        let fam = family("adder", &format!("{n}bit")).unwrap();
        assert_eq!((fam.bits as usize, fam.out_bits as usize), (2 * n, n + 1));
        let mut b = Builder::new(2 * n);
        let (mut carry, mut outs) = (S::Zero, vec![]);
        for i in 0..n {
            let (x, y) = (b.input(i), b.input(n + i));
            let sum = b.xor(&[x, y, carry]);
            let (all, any) = (b.and(&[x, y, carry]), b.or(&[x, y, carry]));
            carry = b.xor(&[sum, all, any]);
            outs.push(sum);
        }
        outs.push(carry);
        let net = b.finish(&outs).unwrap();
        assert_eq!(net.gates.len(), 4 * n - 1);
        let c = net.circuit();
        let res = check(&c, &fam.expect(), 4, &fam.hard).unwrap();
        assert_eq!((res.wrong, res.exhaustive), (0, n == 3));
        // the same adder with the top carry dropped must fail, on the long carry chains at least
        let mut broken = net.clone();
        *broken.outputs.last_mut().unwrap() = None;
        let res = check(&broken.circuit(), &fam.expect(), 4, &fam.hard).unwrap();
        assert!(res.wrong > 0);
    }
}

#[test]
fn a_gate_driving_too_many_is_doubled() {
    use logic_sim::build::{Builder, S};
    let mut b = Builder::new(40);
    let hub = b.and(&[b.input(0), b.input(1)]);
    // 600 different gates that all read the hub, each with two of the other inputs
    let mut outs = vec![];
    for k in 0..600usize {
        let xs = [hub, b.input(2 + k % 38), b.input(2 + k / 38 % 38)];
        outs.push(b.gate((k % 3) as u8 + 3, &xs));
    }
    let parts: Vec<S> = outs.chunks(200).map(|c| b.xor(c)).collect();
    let all = b.xor(&parts);
    let net = b.finish(&[all]).unwrap();
    let c = net.circuit();
    let most = c.readers().iter().map(|r| r.len()).max().unwrap();
    assert!(most <= 255, "{most}");
    assert!(logic_sim::validate::validate(&c).problems.is_empty());
    // same function as the straightforward version without the limit
    let mut plain = net.clone();
    plain.prune();
    assert_eq!(check(&c, &Expect::Circuit(&plain.circuit()), 2, &[]).unwrap().wrong, 0);
}

#[test]
fn blueprint_round_trip() {
    use logic_sim::build::Netlist;
    let net = Netlist::from_json(&json!({"inputs": 2, "gates": [[2, [0, 1]], [0, [0, 1]]], "outputs": [2, 3, null]})).unwrap();
    let (bp, pins) = net.blueprint();
    let c = Circuit::from_blueprint(&bp, Some(&pins)).unwrap();
    assert_eq!((c.gates(), c.ticks()), (4, Some(2)));
    assert_eq!(check(&c, &Expect::Circuit(&net.circuit()), 1, &[]).unwrap().wrong, 0);
    assert!(logic_sim::validate::validate(&c).warnings.is_empty());
}

/// A timer set to d is d + 1 gates in a row: 0 is one gate, 1 is two, and so on. Not only for one change of
/// the input, but for an input that changes every tick: the timer passes on every value, in order.
#[test]
fn a_timer_set_to_d_is_d_plus_one_gates() {
    use logic_sim::circuit::Kind;
    for d in [0u32, 1, 2, 3, 7, 39, 40, 41, 100] {
        // input 0 feeds a row of d + 1 one-input OR gates, and a timer set to d
        let row: Vec<Value> = (0..=d).map(|g| json!([1, [g]])).collect();
        let (last, timer) = (d + 1, d + 2);
        let c = Circuit::from_netlist(&json!({"inputs": 1, "gates": row, "timers": [[0, d]], "outputs": [last, timer]})).unwrap();
        assert_eq!(c.kind[timer as usize], Kind::Timer(d));
        assert_eq!(c.ticks(), Some(d + 2));
        assert!(c.balance().unwrap().balanced);
        let mut sim = Sim::new(&c, 64);
        let mut s = 0x1234_5678_9ABC_DEF1u64 + d as u64;
        for tick in 0..600 {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            sim.force(0, &[s]);
            sim.tick();
            assert_eq!(sim.get(last), sim.get(timer), "timer of {d} against {} gates, tick {tick}", d + 1);
        }
        // and it is the delay that was meant: the input gate is tick 1, the timer shows it d + 1 ticks later
        let mut sim = Sim::new(&c, 1);
        sim.force_all(0, true);
        let first_on = (1..).find(|_| { sim.tick(); on(&sim, timer) }).unwrap();
        assert_eq!(first_on, d + 2);
    }
}

#[test]
fn a_timer_survives_the_blueprint() {
    use logic_sim::build::Netlist;
    use logic_sim::circuit::Kind;
    // settings on both sides of a second (40 ticks), and the longest there is
    let net = Netlist::from_json(&json!({"inputs": 1, "gates": [], "timers": [[0, 0], [0, 39], [0, 40], [0, 41], [0, 2399]], "outputs": [1, 2, 3, 4, 5]})).unwrap();
    let (bp, pins) = net.blueprint();
    let c = Circuit::from_blueprint(&bp, Some(&pins)).unwrap();
    let got: Vec<Kind> = c.outputs.iter().map(|o| c.kind[o.unwrap() as usize]).collect();
    assert_eq!(got, [0, 39, 40, 41, 2399].map(Kind::Timer));
    assert!(logic_sim::validate::validate(&c).problems.is_empty());
    assert!(Circuit::from_netlist(&json!({"inputs": 1, "gates": [], "timers": [[0, 2400]], "outputs": [1]})).is_err());
}

/// A 3 bit ripple adder: bit 0 of the sum is ready after 2 ticks, the carry out after 6.
fn ripple3() -> logic_sim::build::Netlist {
    use logic_sim::build::{Builder, S};
    let mut b = Builder::new(6);
    let (mut carry, mut outs) = (S::Zero, vec![]);
    for i in 0..3 {
        let ins = [b.input(i), b.input(3 + i), carry];
        let sum = b.xor(&ins);
        let (all, any) = (b.and(&ins), b.or(&ins));
        carry = if i == 0 { b.and(&ins[..2]) } else { b.xor(&[sum, all, any]) };
        outs.push(sum);
    }
    outs.push(carry);
    b.finish(&outs).unwrap()
}

#[test]
fn balancing_makes_every_path_the_same() {
    use logic_sim::balance::balance;
    use logic_sim::stream::stream;
    let net = ripple3();
    let c = net.circuit();
    let before = c.balance().unwrap();
    assert!(!before.balanced && before.shortest == 2 && before.longest == c.ticks().unwrap());
    // as it is, it cannot keep up with inputs that change every tick
    assert!(stream(&c, 200, 1).unwrap().wrong > 0);
    for extra in [0, 1, 5] {
        let ticks = c.ticks().unwrap() + extra;
        let b = balance(&net, if extra == 0 { None } else { Some(ticks) }).unwrap();
        let bc = b.circuit();
        assert_eq!(bc.balance().unwrap(), logic_sim::circuit::Balance { balanced: true, shortest: ticks, longest: ticks });
        // only timers were added, and the outputs settle on the same values
        assert_eq!((bc.gates(), b.without_timers().gates), (c.gates(), net.gates.clone()));
        assert!(bc.timers() > 0);
        let same = check(&bc, &Expect::Circuit(&c), 2, &[]).unwrap();
        assert!(same.exhaustive && same.wrong == 0);
        let res = stream(&bc, 200, 1).unwrap();
        assert_eq!((res.wrong, res.ticks, res.tested), (0, ticks, 200 * 256));
        assert!(logic_sim::validate::validate(&bc).problems.is_empty());
        // and it survives being written as a blueprint
        let (bp, pins) = b.blueprint();
        let from_bp = Circuit::from_blueprint(&bp, Some(&pins)).unwrap();
        assert!(from_bp.balance().unwrap().balanced && stream(&from_bp, 200, 2).unwrap().wrong == 0);
    }
    assert!(balance(&net, Some(c.ticks().unwrap() - 1)).is_err());
    assert!(balance(&balance(&net, None).unwrap(), None).is_err());
}

#[test]
fn a_delay_longer_than_a_timer_is_a_row_of_timers() {
    use logic_sim::balance::balance;
    // out 0 = in 0 and in 1 (2 ticks), balanced at 5000: the gate can sit anywhere, the delay is 4998 ticks
    let net = logic_sim::build::Netlist::from_json(&json!({"inputs": 2, "gates": [[0, [0, 1]]], "outputs": [2]})).unwrap();
    let b = balance(&net, Some(5000)).unwrap();
    let c = b.circuit();
    assert_eq!(c.balance().unwrap(), logic_sim::circuit::Balance { balanced: true, shortest: 5000, longest: 5000 });
    // 4998 ticks is three timers (2400 each at most) behind the gate, or three behind each input
    assert_eq!(c.timers(), 3);
    assert_eq!(logic_sim::stream::stream(&c, 50, 1).unwrap().wrong, 0);
}

#[test]
fn one_long_path_is_enough_to_be_unbalanced() {
    // out 0 = in 0 xor (in 0 and in 1): in 0 arrives by 2 ticks and by 3
    let c = Circuit::from_netlist(&json!({"inputs": 2, "gates": [[0, [0, 1]], [2, [0, 2]]], "outputs": [3]})).unwrap();
    assert_eq!(c.balance().unwrap(), logic_sim::circuit::Balance { balanced: false, shortest: 2, longest: 3 });
    // the same with the short wire delayed one tick by a timer set to 0
    let c = Circuit::from_netlist(&json!({"inputs": 2, "gates": [[0, [0, 1]], [2, [4, 2]]], "timers": [[0, 0]], "outputs": [3]})).unwrap();
    assert_eq!(c.balance().unwrap(), logic_sim::circuit::Balance { balanced: true, shortest: 3, longest: 3 });
    assert_eq!((c.gates(), c.timers(), c.ticks()), (4, 1, Some(3)));
    // a latch has memory: balanced does not apply
    let latch = Circuit::from_netlist(&json!({"inputs": 2, "gates": [[4, [1, 3]], [4, [0, 2]]], "outputs": [2, 3]})).unwrap();
    assert_eq!(latch.balance(), None);
}
