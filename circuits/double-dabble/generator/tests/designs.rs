//! Every design, at every size from 1 to 14 bits, on every input.
use logic_sim::check::{check, Expect};
use logic_sim::families::family;

#[test]
fn every_design_converts() {
    for bits in 1..=14 {
        let fam = family("double-dabble", &format!("{bits}bit")).unwrap();
        for d in double_dabble_generator::designs(bits).unwrap() {
            let res = check(&d.netlist.circuit(), &Expect::Run(&*fam.f, &*fam.run), 4, &fam.hard).unwrap();
            assert!(res.exhaustive && res.wrong == 0, "{} at {bits} bits: {} wrong", d.name, res.wrong);
        }
    }
}
