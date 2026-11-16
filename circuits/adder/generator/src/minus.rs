//! Turn an adder into a subtractor. a - b is the inverse of ("not a" + b), and its borrow is that sum's
//! carry. So an adder becomes a subtractor by reading a inverted and giving the sum bits inverted.
//!
//! Inverting costs little. An XOR gate takes a itself and turns into an XNOR. An AND or OR gate that only
//! reads a bits turns into a NOR or NAND of them. Only a gate that mixes a bits with other signals needs a
//! true "not a": an inverter, or, when the gate reads b of the same bit too, "a differs from b" (in an AND)
//! or "a equals b" (in an OR), which the circuit often has already.
//!
//! One more saving: "not a and b", the carry a bit starts, is b xor (a and b). Where only XOR gates read it,
//! it is given as those two terms, which is one gate at one tick where the plain way takes two of each.
//!
//! The adder may have a third row of inputs, 2n .. 3n-1, that stands for the plain a.

use logic_sim::build::{Builder, Netlist, S, AND, OR, XOR};

pub fn minus(n: usize, inputs: usize, gates: &[(u8, Vec<u32>)], outs: &[u32]) -> Result<Netlist, String> {
    let is_x = |i: u32| (i as usize) < n;
    // which bits need a real inverter: the bit is mixed into a gate without its b, or into both kinds of gate
    let (mut alone, mut in_and, mut in_or) = (vec![false; n], vec![false; n], vec![false; n]);
    // gates that only XOR gates read
    let mut only_xor = vec![true; gates.len()];
    let mut mark = |i: u32| {
        if i as usize >= inputs {
            only_xor[i as usize - inputs] = false;
        }
    };
    for (mode, ins) in gates {
        if mode % 3 != XOR {
            ins.iter().for_each(|&i| mark(i));
        }
    }
    outs.iter().for_each(|&o| mark(o));
    // "not a and b" of one bit, read by XOR gates only: the bit it is of
    let start = |g: usize| -> Option<usize> {
        let (mode, ins) = &gates[g];
        let i = *ins.iter().min()? as usize;
        (*mode == AND && only_xor[g] && ins.len() == 2 && i < n && ins.contains(&((n + i) as u32))).then_some(i)
    };
    for (g, (mode, ins)) in gates.iter().enumerate() {
        if mode % 3 == XOR || ins.iter().all(|&i| is_x(i)) || start(g).is_some() {
            continue;
        }
        for &i in ins.iter().filter(|&&i| is_x(i)) {
            let i = i as usize;
            if !ins.contains(&((n + i) as u32)) {
                alone[i] = true;
            } else if mode % 3 == AND {
                in_and[i] = true;
            } else {
                in_or[i] = true;
            }
        }
    }
    let mut b = Builder::new(2 * n);
    let (a, bb): (Vec<S>, Vec<S>) = ((0..n).map(|i| b.input(i)).collect(), (0..n).map(|i| b.input(n + i)).collect());
    let mut made: Vec<(u8, Vec<S>)> = vec![];
    let mut sig: Vec<S> = vec![];
    let plain = |i: u32, sig: &[S]| -> S {
        let i = i as usize;
        if i < 2 * n {
            bb[i - n]
        } else if i < inputs {
            a[i - 2 * n]
        } else {
            sig[i - inputs]
        }
    };
    for (g, (mode, ins)) in gates.iter().enumerate() {
        let (mut mode, base) = (*mode, mode % 3);
        let mut xs: Vec<S> = vec![];
        if start(g).is_some() {
            // never read as a signal: its readers take the two terms
            sig.push(S::Zero);
            made.push((mode, xs));
            continue;
        }
        if base == XOR {
            for &i in ins {
                if let Some(k) = (i as usize).checked_sub(inputs).and_then(start) {
                    xs.extend([bb[k], b.and(&[a[k], bb[k]])]);
                } else if is_x(i) {
                    xs.push(a[i as usize]);
                    mode = (mode + 3) % 6;
                } else {
                    xs.push(plain(i, &sig));
                }
            }
        } else if ins.iter().all(|&i| is_x(i)) {
            // and of inverses is nor, or of inverses is nand
            xs = ins.iter().map(|&i| a[i as usize]).collect();
            mode = (if base == AND { OR } else { AND }) + if mode < 3 { 3 } else { 0 };
        } else {
            for &i in ins {
                let k = i as usize;
                xs.push(if !is_x(i) {
                    plain(i, &sig)
                } else if alone[k] || (in_and[k] && in_or[k]) {
                    b.not(a[k])
                } else if base == AND {
                    b.xor(&[a[k], bb[k]])
                } else {
                    b.xnor(&[a[k], bb[k]])
                });
            }
        }
        sig.push(b.gate(mode, &xs));
        made.push((mode, xs));
    }
    let mut result = vec![];
    for (k, &o) in outs.iter().enumerate() {
        let o = o as usize;
        result.push(if k < n {
            // a difference bit: the inverse of the sum bit
            if o < n {
                a[o]
            } else if o < inputs {
                let p = plain(o as u32, &sig);
                b.not(p)
            } else {
                let (mode, xs) = &made[o - inputs];
                b.gate((mode + 3) % 6, xs)
            }
        } else if o < n {
            b.not(a[o])
        } else {
            plain(o as u32, &sig)
        });
    }
    b.finish(&result)
}
