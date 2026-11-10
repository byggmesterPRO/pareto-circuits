// Evolves a gate circuit (Scrap Mechanic logic gates, any fan-in) towards fewer gates.
// (1+1) evolution with neutral drift on a feed-forward netlist: a child replaces its parent when it has no more
// wrong output bits and no more active gates. Several threads run their own lineage and share the best.
//
// usage: evolve <spec> <out> <threads> <seconds> [seed]
// spec:  "n_in n_out rows n_nodes", then n_in input columns and n_out target columns as 0/1 strings,
//        then n_nodes lines "mode k i1..ik" (signal i < n_in is an input, n_in+g is node g; inputs must be
//        earlier signals), then "outs o1..o_n_out". A line "random" instead of the nodes starts from scratch.
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::{env, fs, thread};

#[derive(Clone)]
struct Geno {
    mode: Vec<u8>,
    mask: Vec<u64>,
    outs: Vec<u32>,
}

struct Ctx {
    n: usize,
    m: usize,
    nn: usize,
    w: usize,
    mw: usize,
    ins: Vec<u64>,
    tgt: Vec<u64>,
    last: u64,
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn below(&mut self, k: usize) -> usize {
        (self.next() >> 11) as usize % k
    }
    fn f(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn bits(s: &str, w: usize) -> Vec<u64> {
    let mut v = vec![0u64; w];
    for (i, c) in s.trim().bytes().enumerate() {
        if c == 49 {
            v[i / 64] |= 1 << (i % 64);
        }
    }
    v
}

fn eval(c: &Ctx, g: &Geno, vals: &mut [u64], act: &mut [bool], acc: &mut [u64]) -> (u32, u32) {
    let (n, w, mw) = (c.n, c.w, c.mw);
    for a in act.iter_mut() {
        *a = false;
    }
    for &o in &g.outs {
        act[o as usize] = true;
    }
    let mut cost = 0;
    for k in (0..c.nn).rev() {
        if !act[n + k] {
            continue;
        }
        cost += 1;
        for j in 0..mw {
            let mut m = g.mask[k * mw + j];
            while m != 0 {
                let i = j * 64 + m.trailing_zeros() as usize;
                m &= m - 1;
                act[i] = true;
            }
        }
    }
    for k in 0..c.nn {
        if !act[n + k] {
            continue;
        }
        let md = g.mode[k];
        let mut first = true;
        for j in 0..mw {
            let mut m = g.mask[k * mw + j];
            while m != 0 {
                let i = j * 64 + m.trailing_zeros() as usize;
                m &= m - 1;
                let src = &vals[i * w..i * w + w];
                if first {
                    acc.copy_from_slice(src);
                    first = false;
                } else {
                    match md % 3 {
                        0 => {
                            for q in 0..w {
                                acc[q] &= src[q];
                            }
                        }
                        1 => {
                            for q in 0..w {
                                acc[q] |= src[q];
                            }
                        }
                        _ => {
                            for q in 0..w {
                                acc[q] ^= src[q];
                            }
                        }
                    }
                }
            }
        }
        let dst = &mut vals[(n + k) * w..(n + k) * w + w];
        if first {
            for q in 0..w {
                dst[q] = 0;
            }
        } else if md >= 3 {
            for q in 0..w {
                dst[q] = !acc[q];
            }
        } else {
            dst.copy_from_slice(acc);
        }
    }
    let mut err = 0;
    for (j, &o) in g.outs.iter().enumerate() {
        let o = o as usize;
        for q in 0..w {
            let mut d = vals[o * w + q] ^ c.tgt[j * w + q];
            if q == w - 1 {
                d &= c.last;
            }
            err += d.count_ones();
        }
    }
    (err, cost)
}

fn dump(c: &Ctx, g: &Geno, err: u32, cost: u32, path: &str) {
    let mut s = format!("{} {}\n", err, cost);
    for k in 0..c.nn {
        let mut ins = vec![];
        for j in 0..c.mw {
            let mut m = g.mask[k * c.mw + j];
            while m != 0 {
                ins.push(j * 64 + m.trailing_zeros() as usize);
                m &= m - 1;
            }
        }
        s += &format!("{} {}", g.mode[k], ins.len());
        for i in ins {
            s += &format!(" {}", i);
        }
        s += "\n";
    }
    s += "outs";
    for o in &g.outs {
        s += &format!(" {}", o);
    }
    s += "\n";
    let tmp = format!("{}.tmp", path);
    fs::write(&tmp, s).unwrap();
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path).unwrap();
}

fn main() {
    let a: Vec<String> = env::args().collect();
    let text = fs::read_to_string(&a[1]).unwrap();
    let out_path = a[2].clone();
    let threads: usize = a[3].parse().unwrap();
    let secs: f64 = a[4].parse().unwrap();
    let seed0: u64 = if a.len() > 5 { a[5].parse().unwrap() } else { 1 };
    let mut lines = text.lines();
    let h: Vec<usize> = lines.next().unwrap().split_whitespace().map(|x| x.parse().unwrap()).collect();
    let (n, m, rows, nn) = (h[0], h[1], h[2], h[3]);
    let w = (rows + 63) / 64;
    let mw = (n + nn + 63) / 64;
    let mut ins = vec![];
    for _ in 0..n {
        ins.extend(bits(lines.next().unwrap(), w));
    }
    let mut tgt = vec![];
    for _ in 0..m {
        tgt.extend(bits(lines.next().unwrap(), w));
    }
    let last = if rows % 64 == 0 { !0u64 } else { (1u64 << (rows % 64)) - 1 };
    let ctx = Arc::new(Ctx { n, m, nn, w, mw, ins, tgt, last });
    let mut g0 = Geno { mode: vec![0; nn], mask: vec![0; nn * mw], outs: vec![0; m] };
    let mut rng = Rng(0x9E3779B97F4A7C15 ^ seed0.wrapping_mul(0xD1B54A32D192ED03));
    let rest: Vec<&str> = lines.collect();
    if rest[0].trim() == "random" {
        for k in 0..nn {
            g0.mode[k] = rng.below(6) as u8;
            for _ in 0..2 {
                let i = rng.below(n + k);
                g0.mask[k * mw + i / 64] |= 1 << (i % 64);
            }
        }
        for j in 0..m {
            g0.outs[j] = rng.below(n + nn) as u32;
        }
    } else {
        for k in 0..nn {
            let t: Vec<usize> = rest[k].split_whitespace().map(|x| x.parse().unwrap()).collect();
            g0.mode[k] = t[0] as u8;
            for &i in &t[2..] {
                assert!(i < n + k);
                g0.mask[k * mw + i / 64] |= 1 << (i % 64);
            }
        }
        let t: Vec<&str> = rest[nn].split_whitespace().collect();
        for j in 0..m {
            g0.outs[j] = t[j + 1].parse().unwrap();
        }
    }
    let mut vals = vec![0u64; (n + nn) * w];
    vals[..n * w].copy_from_slice(&ctx.ins);
    let mut act = vec![false; n + nn];
    let mut acc = vec![0u64; w];
    let (e0, c0) = eval(&ctx, &g0, &mut vals, &mut act, &mut acc);
    println!("start err={} gates={}", e0, c0);
    let best = Arc::new(Mutex::new((e0, c0, g0.clone())));
    let t0 = Instant::now();
    let mut hs = vec![];
    for t in 0..threads {
        let (ctx, best, out_path, g0) = (ctx.clone(), best.clone(), out_path.clone(), g0.clone());
        hs.push(thread::spawn(move || {
            let c = &*ctx;
            let mut rng = Rng(0xA0761D6478BD642F ^ (seed0 * 1000 + t as u64 + 1).wrapping_mul(0xE7037ED1A0B428DB));
            let mut vals = vec![0u64; (c.n + c.nn) * c.w];
            vals[..c.n * c.w].copy_from_slice(&c.ins);
            let mut acc = vec![0u64; c.w];
            let mut pact = vec![false; c.n + c.nn];
            let mut cact = vec![false; c.n + c.nn];
            let mut par = g0.clone();
            let (mut pe, mut pc) = eval(c, &par, &mut vals, &mut pact, &mut acc);
            let mut child = par.clone();
            let mut it: u64 = 0;
            loop {
                it += 1;
                if it % 20000 == 0 {
                    if t0.elapsed().as_secs_f64() > secs {
                        break;
                    }
                    let mut b = best.lock().unwrap();
                    if (pe, pc) < (b.0, b.1) {
                        *b = (pe, pc, par.clone());
                        dump(c, &par, pe, pc, &out_path);
                        println!("t={:.0}s err={} gates={} (thread {})", t0.elapsed().as_secs_f64(), pe, pc, t);
                    } else if (b.0, b.1) < (pe, pc) && it % 400000 == 0 {
                        par = b.2.clone();
                        let r = eval(c, &par, &mut vals, &mut pact, &mut acc);
                        pe = r.0;
                        pc = r.1;
                    }
                }
                child.mode.copy_from_slice(&par.mode);
                child.mask.copy_from_slice(&par.mask);
                child.outs.copy_from_slice(&par.outs);
                loop {
                    let r = rng.f();
                    if r < 0.03 {
                        let j = rng.below(c.m);
                        child.outs[j] = rng.below(c.n + c.nn) as u32;
                        break;
                    }
                    let k = rng.below(c.nn);
                    if r < 0.15 {
                        child.mode[k] = rng.below(6) as u8;
                    } else {
                        let i = rng.below(c.n + k);
                        child.mask[k * c.mw + i / 64] ^= 1 << (i % 64);
                    }
                    if pact[c.n + k] {
                        break;
                    }
                }
                let (ce, cc) = eval(c, &child, &mut vals, &mut cact, &mut acc);
                if ce < pe || (ce == pe && cc <= pc) {
                    std::mem::swap(&mut par, &mut child);
                    std::mem::swap(&mut pact, &mut cact);
                    pe = ce;
                    pc = cc;
                }
            }
            let mut b = best.lock().unwrap();
            if (pe, pc) < (b.0, b.1) {
                *b = (pe, pc, par.clone());
                dump(c, &par, pe, pc, &out_path);
            }
        }));
    }
    for h in hs {
        h.join().unwrap();
    }
    let b = best.lock().unwrap();
    println!("done err={} gates={}", b.0, b.1);
}
