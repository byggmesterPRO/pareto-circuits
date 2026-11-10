//! Every design, at every size from 1 to 9 bits, on every input.
use logic_sim::check::{check, Expect};
use logic_sim::families::family;

#[test]
fn every_design_adds() {
    for n in 1..=9 {
        let fam = family("adder", &format!("{n}bit")).unwrap();
        for d in adder_generator::designs(n).unwrap() {
            let res = check(&d.netlist.circuit(), &Expect::Run(&*fam.f, &*fam.run), 4, &fam.hard).unwrap();
            assert!(res.exhaustive && res.wrong == 0, "{} at {n} bits: {} wrong", d.name, res.wrong);
        }
    }
}

#[test]
fn large_sizes_on_hard_cases() {
    for n in [33, 64] {
        let fam = family("adder", &format!("{n}bit")).unwrap();
        for d in adder_generator::designs(n).unwrap() {
            let res = check(&d.netlist.circuit(), &Expect::Run(&*fam.f, &*fam.run), 4, &fam.hard).unwrap();
            assert_eq!(res.wrong, 0, "{} at {n} bits", d.name);
        }
    }
}
