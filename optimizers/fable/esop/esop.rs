// Minimise an XOR-of-terms form (ESOP) for a multi-output truth table by annealing over exorlink rewrites.
// Same moves and cost as sm_blueprint_lib's xor_rom (_minimize), just millions of steps per second.
//
// A term is (plain mask, inverted mask) over n variables with a set of outputs it feeds.
// Cost = terms with 2+ literals + variables that need an inverter (inverted in a term that also has plain ones).
//
// usage: esop <start> <out> <threads> <seconds> [seed]
// start: "n m base" then lines "plain inverted outputs"; base is added to the printed cost (output gates etc.)
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::{env, fs, thread};

#[derive(Default)]
struct Fx(u64);
impl Hasher for Fx {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, b: &[u8]) {
        for &x in b {
            self.0 = (self.0.rotate_left(5) ^ x as u64).wrapping_mul(0x517cc1b727220a95);
        }
    }
    fn write_u32(&mut self, x: u32) {
        self.0 = (x as u64).wrapping_mul(0x9E3779B97F4A7C15);
        self.0 ^= self.0 >> 29;
    }
}
type Map = HashMap<u32, (u16, u32), BuildHasherDefault<Fx>>;

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

struct St {
    n: usize,
    map: Map,
    keys: Vec<u32>,
    terms: i32,
    inv_cnt: [i32; 16],
    inv: i32,
}

#[inline]
fn state(c: u32, i: usize) -> u32 {
    if c >> i & 1 == 1 {
        1
    } else if c >> (16 + i) & 1 == 1 {
        0
    } else {
        2
    }
}
#[inline]
fn with_state(c: u32, i: usize, s: u32) -> u32 {
    let c = c & !(1 << i) & !(1 << (16 + i));
    match s {
        1 => c | 1 << i,
        0 => c | 1 << (16 + i),
        _ => c,
    }
}

impl St {
    fn new(n: usize) -> St {
        St { n, map: Map::default(), keys: vec![], terms: 0, inv_cnt: [0; 16], inv: 0 }
    }
    fn charge(&mut self, c: u32, sign: i32) {
        let (p, q) = (c & 0xFFFF, c >> 16);
        if (p | q).count_ones() >= 2 {
            self.terms += sign;
            if p != 0 && q != 0 {
                let mut m = q;
                while m != 0 {
                    let i = m.trailing_zeros() as usize;
                    m &= m - 1;
                    let before = self.inv_cnt[i];
                    self.inv_cnt[i] += sign;
                    if before == 0 && sign > 0 {
                        self.inv += 1;
                    }
                    if before == 1 && sign < 0 {
                        self.inv -= 1;
                    }
                }
            }
        }
    }
    #[inline]
    fn get(&self, c: u32) -> u16 {
        match self.map.get(&c) {
            Some(v) => v.0,
            None => 0,
        }
    }
    fn set(&mut self, c: u32, om: u16) {
        match self.map.get(&c).copied() {
            Some((_, idx)) => {
                if om == 0 {
                    self.map.remove(&c);
                    let last = self.keys.pop().unwrap();
                    if last != c {
                        self.keys[idx as usize] = last;
                        self.map.get_mut(&last).unwrap().1 = idx;
                    }
                    self.charge(c, -1);
                } else {
                    self.map.get_mut(&c).unwrap().0 = om;
                }
            }
            None => {
                if om != 0 {
                    self.map.insert(c, (om, self.keys.len() as u32));
                    self.keys.push(c);
                    self.charge(c, 1);
                }
            }
        }
    }
    fn xor_in(&mut self, c: u32, om: u16, j: &mut Vec<(u32, u16)>) {
        let old = self.get(c);
        j.push((c, old));
        self.set(c, old ^ om);
    }
    fn merge(&mut self, mut c: u32, j: &mut Vec<(u32, u16)>) {
        loop {
            let om = self.get(c);
            if om == 0 {
                return;
            }
            let mut found = false;
            'outer: for i in 0..self.n {
                let si = state(c, i);
                for s in 0..3 {
                    if s == si {
                        continue;
                    }
                    let nb = with_state(c, i, s);
                    if self.get(nb) == om {
                        self.xor_in(c, om, j);
                        self.xor_in(nb, om, j);
                        let merged = with_state(c, i, 3 - si - s);
                        self.xor_in(merged, om, j);
                        c = merged;
                        found = true;
                        break 'outer;
                    }
                }
            }
            if !found {
                return;
            }
        }
    }
    fn cost(&self) -> i32 {
        self.terms + self.inv
    }
    fn snapshot(&self) -> Vec<(u32, u16)> {
        self.keys.iter().map(|&k| (k, self.get(k))).collect()
    }
    fn load(&mut self, cubes: &[(u32, u16)]) {
        self.map.clear();
        self.keys.clear();
        self.terms = 0;
        self.inv = 0;
        self.inv_cnt = [0; 16];
        let mut j = vec![];
        for &(c, om) in cubes {
            self.xor_in(c, om, &mut j);
        }
        let ks = self.keys.clone();
        for c in ks {
            self.merge(c, &mut j);
        }
    }
}

fn dump(n: usize, m: usize, base: i32, cost: i32, cubes: &[(u32, u16)], path: &str) {
    let mut s = format!("{} {} {} {}\n", n, m, base, cost + base);
    for &(c, om) in cubes {
        s += &format!("{} {} {}\n", c & 0xFFFF, c >> 16, om);
    }
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
    let h: Vec<i64> = lines.next().unwrap().split_whitespace().map(|x| x.parse().unwrap()).collect();
    let (n, m, base) = (h[0] as usize, h[1] as usize, h[2] as i32);
    let mut start = vec![];
    for l in lines {
        let t: Vec<u32> = l.split_whitespace().map(|x| x.parse().unwrap()).collect();
        if t.len() == 3 {
            start.push((t[0] | t[1] << 16, t[2] as u16));
        }
    }
    let mut st0 = St::new(n);
    st0.load(&start);
    println!("start cost {} ({} terms)", st0.cost() + base, st0.keys.len());
    let best = Arc::new(Mutex::new((st0.cost(), st0.snapshot())));
    let t0 = Instant::now();
    let mut hs = vec![];
    for t in 0..threads {
        let (best, out_path) = (best.clone(), out_path.clone());
        hs.push(thread::spawn(move || {
            let mut rng = Rng(0xA0761D6478BD642F ^ (seed0 * 1000 + t as u64 + 1).wrapping_mul(0xE7037ED1A0B428DB));
            let mut st = St::new(n);
            let mut j: Vec<(u32, u16)> = vec![];
            let mut cands: Vec<(u32, usize, usize)> = vec![];
            let t_hi = 0.25 + 0.35 * (t as f64 / threads.max(2) as f64);
            let t_lo = 0.06;
            let cycle: u64 = 12_000_000;
            let mut my_best = i32::MAX;
            loop {
                {
                    let b = best.lock().unwrap();
                    st.load(&b.1);
                }
                for it in 0..cycle {
                    if it % 200_000 == 0 && t0.elapsed().as_secs_f64() > secs {
                        break;
                    }
                    let temp = t_hi * (t_lo / t_hi).powf(it as f64 / cycle as f64);
                    if st.keys.len() < 2 {
                        break;
                    }
                    let a = st.keys[rng.below(st.keys.len())];
                    let om = st.get(a);
                    cands.clear();
                    for i in 0..n {
                        let si = state(a, i);
                        for s in 0..3 {
                            if s == si {
                                continue;
                            }
                            let nb = with_state(a, i, s);
                            let o = st.get(nb);
                            if o != 0 && o != om {
                                cands.push((nb, i, 99));
                            }
                            for k in (i + 1)..n {
                                let sk = state(a, k);
                                for s2 in 0..3 {
                                    if s2 == sk {
                                        continue;
                                    }
                                    let nb2 = with_state(nb, k, s2);
                                    if st.get(nb2) == om {
                                        cands.push((nb2, i, k));
                                    }
                                }
                            }
                        }
                    }
                    if cands.is_empty() {
                        continue;
                    }
                    let (b, i, k) = cands[rng.below(cands.len())];
                    let before = st.cost();
                    j.clear();
                    let (ai, bi) = (state(a, i), state(b, i));
                    let xi = with_state(a, i, 3 - ai - bi);
                    let coin = rng.next() & 1 == 0;
                    if k == 99 {
                        let omb = st.get(b);
                        st.xor_in(a, om, &mut j);
                        st.xor_in(b, omb, &mut j);
                        if coin {
                            st.xor_in(xi, om, &mut j);
                            st.xor_in(b, om ^ omb, &mut j);
                            st.merge(xi, &mut j);
                            st.merge(b, &mut j);
                        } else {
                            st.xor_in(a, om ^ omb, &mut j);
                            st.xor_in(xi, omb, &mut j);
                            st.merge(a, &mut j);
                            st.merge(xi, &mut j);
                        }
                    } else {
                        let (ak, bk) = (state(a, k), state(b, k));
                        st.xor_in(a, om, &mut j);
                        st.xor_in(b, om, &mut j);
                        let (c1, c2) = if coin {
                            (xi, with_state(b, k, 3 - ak - bk))
                        } else {
                            (with_state(a, k, 3 - ak - bk), with_state(b, i, 3 - ai - bi))
                        };
                        st.xor_in(c1, om, &mut j);
                        st.xor_in(c2, om, &mut j);
                        st.merge(c1, &mut j);
                        st.merge(c2, &mut j);
                    }
                    let d = st.cost() - before;
                    if d > 0 && rng.f() >= (-(d as f64) / temp).exp() {
                        for &(c, old) in j.iter().rev() {
                            st.set(c, old);
                        }
                    } else if st.cost() < my_best {
                        my_best = st.cost();
                        let mut b = best.lock().unwrap();
                        if my_best < b.0 {
                            *b = (my_best, st.snapshot());
                            dump(n, m, base, b.0, &b.1, &out_path);
                            println!("t={:.0}s cost {} (thread {})", t0.elapsed().as_secs_f64(), b.0 + base, t);
                        }
                    }
                }
                if t0.elapsed().as_secs_f64() > secs {
                    break;
                }
                my_best = best.lock().unwrap().0;
            }
        }));
    }
    for h in hs {
        h.join().unwrap();
    }
    let b = best.lock().unwrap();
    dump(n, m, base, b.0, &b.1, &out_path);
    println!("done cost {}", b.0 + base);
}
