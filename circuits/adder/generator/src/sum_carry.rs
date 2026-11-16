//! Adders that read the carry back from the sum, and a planner that mixes that with lookahead to meet a tick
//! limit with few gates. Ported from the search result tools/evolve/tasks/adder/runs/between/cand_08.py.
//!
//! The carry out of bit i, with s its sum bit, is s xor or(a, b, s) xor and(a, b, s). So a bit costs three
//! gates: the sum (one wide XOR over a, b and the three carry terms of the bit below), the OR and the AND.
//! The XOR that would join the three terms into a carry is never built: the next sum takes the terms as they
//! are. That is 2 ticks per bit.
//!
//! The planner goes up the bits with a dynamic program. Per bit it picks how the carry is made: the 3-gate
//! step above, or lookahead from a "source" lower down (a carry, its terms, or the sum at the source bit),
//! optionally over group generates. It keeps the cheapest plans for every depth it can still afford.

use crate::net::{ticks, FxMap as HashMap, Net};
use logic_sim::build::{AND, OR, XOR};
use std::rc::Rc;

/// Construction (a): 3 gates per bit, 2 ticks per bit.
pub fn ripple(n: usize) -> (Net, Vec<u32>) {
    let mut net = Net::new(n);
    let nn = n as u32;
    let mut outs = vec![net.g(XOR, &[0, nn])];
    let mut c = vec![net.g(AND, &[0, nn])];
    for i in 1..nn {
        let mut xs = vec![i, nn + i];
        xs.extend(&c);
        let s = net.g(XOR, &xs);
        outs.push(s);
        c = vec![s, net.g(OR, &[i, nn + i, s]), net.g(AND, &[i, nn + i, s])];
    }
    outs.push(if c.len() == 1 { c[0] } else { net.g(XOR, &c) });
    (net, outs)
}

/// A group generate over the bits u .. v: "x" (every monomial, depth 2) or "t" (terms over p, depth 3).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    X,
    T,
}

/// Group generates in the order they were added (the order decides ties, as in the Python).
type Blocks = Vec<((usize, usize), usize, Kind)>;

/// A segment of the carry cover: (u, w, top). top means it ends at the bit asked for and is used as it is.
type Seg = (usize, usize, bool);

#[derive(Clone)]
enum Segs {
    None,
    Exp,
    List(Rc<Vec<Seg>>),
}

impl Segs {
    fn len(&self) -> usize {
        match self {
            Segs::List(s) => s.len(),
            _ => 0,
        }
    }
}

#[derive(Clone)]
enum Tag {
    R,
    S(Segs),
    L(Segs),
}

#[derive(Clone, Copy, PartialEq)]
enum How {
    Keep,
    Drop,
    Sum,
    Mat,
    Virt,
    End,
}

/// (source position, source kind, source terms, source depth, depth of the previous sum, highest propagate
/// already built). Kinds: 0 none, 1 a carry signal, 2 the sum at the source bit, 3 a carry as terms.
type St = [usize; 6];
const END: St = [usize::MAX; 6];

type Val = (f64, Option<(St, Tag, How)>);

/// A map that remembers the order keys were first put in, like a Python dict.
#[derive(Default)]
struct OMap {
    keys: Vec<St>,
    vals: Vec<Val>,
    idx: HashMap<St, usize>,
}

impl OMap {
    fn get(&self, k: &St) -> Option<&Val> {
        self.idx.get(k).map(|&i| &self.vals[i])
    }
    fn set(&mut self, k: St, v: Val) {
        match self.idx.get(&k) {
            Some(&i) => self.vals[i] = v,
            None => {
                self.idx.insert(k, self.keys.len());
                self.keys.push(k);
                self.vals.push(v);
            }
        }
    }
    fn len(&self) -> usize {
        self.keys.len()
    }
}

/// A state as Python prints it: the beam's last tie break is the order of these strings.
fn st_str(s: &St) -> String {
    format!("({}, {}, {}, {}, {}, {})", s[0], s[1], s[2], s[3], s[4], s[5])
}

struct Cover {
    ends: HashMap<usize, Vec<(usize, usize)>>,
    lam: f64,
    /// what a single bit that starts a carry costs on top of its gate (for a - b: its inverter)
    inv: f64,
    memo: HashMap<(usize, usize, usize), Option<(f64, Rc<Vec<Seg>>)>>,
}

impl Cover {
    fn new(blocks: &Blocks, lam: f64, inv: f64) -> Cover {
        let mut ends: HashMap<usize, Vec<(usize, usize)>> = HashMap::default();
        for &((u, v), d, _) in blocks {
            ends.entry(v).or_default().push((u, d));
        }
        Cover { ends, lam, inv, memo: HashMap::default() }
    }

    /// XOR terms for the generate of bits lo .. j at depth at most d: (cost, segments).
    fn cover(&mut self, lo: usize, j: usize, d: usize) -> Option<(f64, Rc<Vec<Seg>>)> {
        if let Some(r) = self.memo.get(&(lo, j, d)) {
            return r.clone();
        }
        let (lam, one) = (self.lam, 1.0 + self.inv);
        let inf = f64::INFINITY;
        let mut f = vec![inf; j + 1];
        let mut back = vec![usize::MAX; j + 1];
        f[lo] = 0.0;
        let none = vec![];
        for w in lo + 1..j {
            if d >= 2 && f[w - 1] + one < f[w] {
                f[w] = f[w - 1] + one;
                back[w] = w - 1;
            }
            for &(u, bd) in self.ends.get(&w).unwrap_or(&none) {
                if u >= lo && bd.max(1) < d && f[u] + 1.0 + lam < f[w] {
                    f[w] = f[u] + 1.0 + lam;
                    back[w] = u;
                }
            }
        }
        let mut best: Option<(f64, usize)> = None;
        if d >= 1 && f[j - 1] < inf {
            best = Some((f[j - 1] + one, j - 1));
        }
        for &(u, bd) in self.ends.get(&j).unwrap_or(&none) {
            if u >= lo && bd <= d && f[u] < inf && best.is_none_or(|b| f[u] + lam < b.0) {
                best = Some((f[u] + lam, u));
            }
        }
        let r = best.map(|(c, b)| {
            let mut segs = vec![(b, j, true)];
            let mut w = b;
            while w > lo {
                segs.push((back[w], w, false));
                w = back[w];
            }
            (c, Rc::new(segs))
        });
        self.memo.insert((lo, j, d), r.clone());
        r
    }
}

/// Drop states that another with the same source beats in cost, source depth, previous sum depth and
/// propagates already paid.
fn prune(states: OMap) -> OMap {
    let mut order: Vec<[usize; 3]> = vec![];
    let mut groups: HashMap<[usize; 3], Vec<(St, Val)>> = HashMap::default();
    for (k, v) in states.keys.into_iter().zip(states.vals) {
        let g = [k[0], k[1], k[2]];
        groups.entry(g).or_insert_with(|| {
            order.push(g);
            vec![]
        });
        groups.get_mut(&g).unwrap().push((k, v));
    }
    let mut out = OMap::default();
    for g in order {
        let mut items = groups.remove(&g).unwrap();
        items.sort_by(|a, b| a.1.0.partial_cmp(&b.1.0).unwrap());
        let mut kept: Vec<(St, Val)> = vec![];
        for (st, val) in items {
            let c = val.0;
            let dom = kept.iter().any(|(s2, v2)| s2[3] <= st[3] && s2[4] <= st[4] && v2.0 + (st[5].saturating_sub(s2[5])) as f64 <= c);
            if !dom {
                kept.push((st, val));
            }
        }
        for (st, val) in kept {
            out.set(st, val);
        }
    }
    out
}

type LaOpts = Rc<Vec<(usize, f64, Segs)>>;

fn plan(n: usize, t: usize, blocks: &Blocks, lam: f64, inv: f64, sub: bool) -> Option<Vec<(usize, Tag, How)>> {
    // a sum as the source costs its OR gate. For a - b also "a equals b" and the sum's inverse.
    let sum_cost = if sub { 3.0 } else { 1.0 };
    const W: usize = 8;
    const BEAM: usize = 400;
    let mut cover = Cover::new(blocks, lam, inv);
    // from this depth on the cover cannot change, so deeper limits add no option
    let flat = blocks.iter().map(|b| b.1 + 1).max().unwrap_or(0).max(2);
    let mut front: HashMap<(usize, usize), LaOpts> = HashMap::default();
    let mut la_opts = |lo: usize, j: usize| -> LaOpts {
        if let Some(o) = front.get(&(lo, j)) {
            return o.clone();
        }
        let mut opts = vec![];
        let mut last: Option<f64> = None;
        if lo == 0 && (2..=4).contains(&j) {
            // every monomial of the carry, all at depth 1
            let c = ((1usize << j) - 1) as f64;
            opts.push((1, c, Segs::Exp));
            last = Some(c);
        }
        for d in 1..t.min(flat + 1) {
            if let Some((c, segs)) = cover.cover(lo, j, d) {
                if last.is_none_or(|l| c < l) {
                    opts.push((d, c, Segs::List(segs)));
                    last = Some(c);
                }
            }
        }
        let o = Rc::new(opts);
        front.insert((lo, j), o.clone());
        o
    };

    let mut states = OMap::default();
    states.set([0, 0, 0, 0, 1, 0], (1.0, None));
    let mut hist: Vec<OMap> = vec![];
    for j in 1..=n {
        let mut new = OMap::default();
        for si in 0..states.len() {
            let st = states.keys[si];
            let cost = states.vals[si].0;
            let [src, kind, ksrc, dsrc, dprev, pk] = st;
            // (tag, cost, depth of the carry, number of carry terms, highest propagate built)
            let mut opts: Vec<(Tag, f64, usize, usize, usize)> = vec![];
            if j >= 2 {
                opts.push((Tag::R, 2.0, dprev + 1, 3, pk));
            }
            let plo = if src > 0 { src } else { 1 };
            let newp = (j - 1).saturating_sub(pk.max(plo - 1)) as f64;
            if kind == 2 {
                if j == src + 1 {
                    opts.push((Tag::S(Segs::None), 1.0 + newp, (dsrc + 1).max(1), 2, pk.max(j - 1)));
                } else {
                    for (d, c, segs) in la_opts(src + 1, j).iter() {
                        opts.push((Tag::S(segs.clone()), c + 2.0 + newp, (*d).max(2).max(dsrc + 1), segs.len() + 2, pk.max(j - 1)));
                    }
                }
            } else {
                for (d, c, segs) in la_opts(src, j).iter() {
                    let (mut l, mut c, mut k) = (*d, *c, if matches!(segs, Segs::Exp) { *c as usize } else { segs.len() });
                    if src > 0 {
                        l = (*d).max(dsrc + 1);
                        c += ksrc as f64;
                        k += ksrc;
                    }
                    opts.push((Tag::L(segs.clone()), c + newp, l, k, pk.max(j - 1)));
                }
            }
            for (tag, c, l, k, pk2) in opts {
                if j < n {
                    if l + 1 > t {
                        continue;
                    }
                    let base = cost + c + 1.0;
                    let mut ch: Vec<(St, f64, How)> = vec![];
                    if src > 0 && j + 1 - src > W {
                        // a source this far back is never worth a lookahead: forget it
                        ch.push(([0, 0, 0, 0, l + 1, 0], base, How::Drop));
                    } else {
                        ch.push(([src, kind, ksrc, dsrc, l + 1, pk2], base, How::Keep));
                    }
                    ch.push(([j, 2, 1, l + 1, l + 1, j - 1], base + sum_cost, How::Sum));
                    if k == 1 {
                        ch.push(([j, 1, 1, l, l + 1, j - 1], base, How::Mat));
                    } else {
                        ch.push(([j, 3, k, l, l + 1, j - 1], base, How::Virt));
                    }
                    for (ns, nc, how) in ch {
                        if new.get(&ns).is_none_or(|v| nc < v.0) {
                            new.set(ns, (nc, Some((st, tag.clone(), how))));
                        }
                    }
                } else {
                    let extra = if k > 1 { 1 } else { 0 };
                    if l + extra > t {
                        continue;
                    }
                    let nc = cost + c + extra as f64;
                    if new.get(&END).is_none_or(|v| nc < v.0) {
                        new.set(END, (nc, Some((st, tag.clone(), How::End))));
                    }
                }
            }
        }
        if j < n {
            new = prune(new);
            if new.len() > BEAM {
                // keep the cheapest and the shallowest, deterministically
                let mut items: Vec<(St, Val, String)> = new.keys.into_iter().zip(new.vals).map(|(k, v)| (k, v, st_str(&k))).collect();
                items.sort_by(|a, b| a.1.0.partial_cmp(&b.1.0).unwrap().then(a.0[4].cmp(&b.0[4])).then_with(|| a.2.cmp(&b.2)));
                let mut keep = OMap::default();
                for (k, v, _) in items.iter().take(BEAM / 2) {
                    keep.set(*k, v.clone());
                }
                items.sort_by(|a, b| a.0[4].cmp(&b.0[4]).then(a.1.0.partial_cmp(&b.1.0).unwrap()).then_with(|| a.2.cmp(&b.2)));
                for (k, v, _) in items {
                    if keep.len() >= BEAM {
                        break;
                    }
                    keep.set(k, v);
                }
                new = keep;
            }
        }
        hist.push(std::mem::replace(&mut states, new));
    }
    states.get(&END)?;
    let mut steps = vec![];
    let mut st = END;
    let mut cur = &states;
    for j in (1..=n).rev() {
        let (prev, tag, how) = cur.get(&st).unwrap().1.clone().unwrap();
        steps.push((j, tag, how));
        st = prev;
        cur = &hist[j - 1];
    }
    steps.reverse();
    Some(steps)
}

enum Src {
    None,
    Terms(Vec<u32>),
    Sum(u32, u32),
}

struct Realize {
    n: u32,
    net: Net,
    p: Vec<Option<u32>>,
    x: HashMap<(usize, usize), u32>,
    kinds: HashMap<(usize, usize), Kind>,
}

impl Realize {
    fn p(&mut self, i: usize) -> u32 {
        if self.p[i].is_none() {
            self.p[i] = Some(self.net.g(XOR, &[i as u32, self.n + i as u32]));
        }
        self.p[i].unwrap()
    }

    fn ps(&mut self, r: std::ops::Range<usize>) -> Vec<u32> {
        r.map(|k| self.p(k)).collect()
    }

    /// Every monomial of "a carry starts in bits lo .. hi and gets out": g_i and a or b of every bit above.
    fn monomials(&mut self, lo: usize, hi: usize) -> Vec<u32> {
        let n = self.n;
        let mut mons = vec![];
        for i in lo..hi {
            let mut combos: Vec<Vec<u32>> = vec![vec![i as u32, n + i as u32]];
            for k in i + 1..hi {
                let k = k as u32;
                let mut next: Vec<Vec<u32>> = combos.iter().map(|c| [c.as_slice(), &[k]].concat()).collect();
                next.extend(combos.iter().map(|c| [c.as_slice(), &[n + k]].concat()));
                combos = next;
            }
            for c in combos {
                mons.push(self.net.g(AND, &c));
            }
        }
        mons
    }

    fn blk(&mut self, u: usize, v: usize) -> u32 {
        if let Some(&s) = self.x.get(&(u, v)) {
            return s;
        }
        let s = if self.kinds[&(u, v)] == Kind::X {
            let mons = self.monomials(u, v);
            self.net.g(XOR, &mons)
        } else {
            let mut terms = vec![];
            for i in u..v {
                let mut xs = vec![i as u32, self.n + i as u32];
                xs.extend(self.ps(i + 1..v));
                terms.push(self.net.g(AND, &xs));
            }
            self.net.g(XOR, &terms)
        };
        self.x.insert((u, v), s);
        s
    }

    fn segterms(&mut self, segs: &Segs, j: usize) -> Vec<u32> {
        match segs {
            Segs::None => vec![],
            Segs::Exp => self.monomials(0, j),
            Segs::List(segs) => {
                let mut c = vec![];
                for &(u, w, top) in segs.iter() {
                    let ps = self.ps(w..j);
                    if w - u == 1 {
                        let mut xs = vec![u as u32, self.n + u as u32];
                        xs.extend(ps);
                        c.push(self.net.g(AND, &xs));
                    } else {
                        let x = self.blk(u, w);
                        c.push(if top {
                            x
                        } else {
                            let mut xs = vec![x];
                            xs.extend(ps);
                            self.net.g(AND, &xs)
                        });
                    }
                }
                c
            }
        }
    }
}

fn realize(n: usize, steps: &[(usize, Tag, How)], blocks: &Blocks, sub: bool) -> (Net, Vec<u32>) {
    let nn = n as u32;
    let mut r = Realize { n: nn, net: if sub { Net::wide(n) } else { Net::new(n) }, p: vec![None; n], x: HashMap::default(), kinds: blocks.iter().map(|&(uv, _, k)| (uv, k)).collect() };
    let mut outs = vec![r.p(0)];
    let (mut src, mut srcsig) = (0usize, Src::None);
    for (j, tag, how) in steps {
        let j = *j;
        let mut c: Vec<u32>;
        match tag {
            Tag::R => {
                let s = outs[j - 1];
                let (a, b) = (j as u32 - 1, nn + j as u32 - 1);
                c = if sub {
                    // a - b: the borrow out is a xor or(a, b, d) xor and(a, b, d), d the difference bit
                    let (a, d) = (2 * nn + a, r.net.inverse(s));
                    vec![a, r.net.g(OR, &[a, b, d]), r.net.g(AND, &[a, b, d])]
                } else {
                    vec![s, r.net.g(OR, &[a, b, s]), r.net.g(AND, &[a, b, s])]
                };
            }
            Tag::S(segs) => {
                let Src::Sum(s, t) = srcsig else { unreachable!("a sum source") };
                c = r.segterms(segs, j);
                let mut xs = vec![s];
                xs.extend(r.ps(src..j));
                c.push(r.net.g(AND, &xs));
                c.push(if j == src + 1 {
                    t
                } else {
                    let mut xs = vec![t];
                    xs.extend(r.ps(src + 1..j));
                    r.net.g(AND, &xs)
                });
            }
            Tag::L(segs) => {
                c = r.segterms(segs, j);
                if src > 0 {
                    let ps = r.ps(src..j);
                    let Src::Terms(terms) = &srcsig else { unreachable!("a carry source") };
                    for &t in terms {
                        let mut xs = vec![t];
                        xs.extend(&ps);
                        c.push(r.net.g(AND, &xs));
                    }
                }
            }
        }
        if *how == How::End {
            outs.push(if c.len() == 1 { c[0] } else { r.net.g(XOR, &c) });
            break;
        }
        let mut xs = vec![j as u32, nn + j as u32];
        xs.extend(&c);
        outs.push(r.net.g(XOR, &xs));
        match how {
            How::Mat => {
                let carry = if c.len() == 1 { c[0] } else { r.net.g(XOR, &c) };
                (src, srcsig) = (j, Src::Terms(vec![carry]));
            }
            How::Virt => (src, srcsig) = (j, Src::Terms(c)),
            How::Sum => {
                let t = r.net.g(OR, &[j as u32, nn + j as u32]);
                (src, srcsig) = (j, Src::Sum(outs[j], t));
            }
            How::Drop => (src, srcsig) = (0, Src::None),
            _ => {}
        }
    }
    (r.net, outs)
}

/// Candidate sets of group generates: none, or a tiling by one size with an offset.
fn block_sets(n: usize, t: usize) -> Vec<Blocks> {
    let mut out: Vec<Blocks> = vec![vec![]];
    if t > 6 {
        return out;
    }
    let small = n <= 12;
    // the first few groups expanded to depth 2, then nothing
    if t == 4 {
        for b in [2, 3] {
            let rs: Vec<usize> = if small { (1..n / b + 1).collect() } else { vec![1, 2, n / b] };
            for r in rs {
                out.push((0..r).map(|i| ((i * b, i * b + b), 2, Kind::X)).collect());
            }
        }
    }
    let best_fit = (2..6).map(|c: i64| (c * c - (n / 2) as i64).abs()).min().unwrap();
    for b in 2..6usize {
        for (kind, d) in [(Kind::X, 2), (Kind::T, 3)] {
            if (kind == Kind::X) != (t == 4) || (kind == Kind::X && b > 3) {
                continue;
            }
            if !small && ((b * b) as i64 - (n / 2) as i64).abs() > best_fit {
                continue;
            }
            let offsets: Vec<usize> = if small { (0..b).collect() } else { vec![0] };
            for o in offsets {
                let mut cuts: Vec<usize> = vec![0, n];
                cuts.extend((o..=n).step_by(b));
                cuts.sort_unstable();
                cuts.dedup();
                let bl: Blocks = cuts.windows(2).filter(|w| w[1] - w[0] >= 2).map(|w| ((w[0], w[1]), d, kind)).collect();
                if !bl.is_empty() {
                    out.push(bl);
                }
            }
        }
    }
    out
}

/// Gates and ticks of a plan as it will be built. For a - b that is after `minus`, which adds inverters.
fn size(n: usize, net: &Net, outs: &[u32], sub: bool) -> Option<(usize, usize)> {
    if !sub {
        return Some((net.gates.len(), net.depth(outs) as usize));
    }
    let m = crate::minus::minus(n, net.inputs(), &net.gates, outs).ok()?;
    Some((m.gates.len(), ticks(&m)))
}

type Built = (usize, Net, Vec<u32>);

/// The smallest plan for these group generates, built: (gates, net, outputs).
fn try_blocks(n: usize, t: usize, bl: &Blocks, sub: bool) -> Option<Built> {
    let mut best: Option<Built> = None;
    let lams: &[f64] = if bl.is_empty() { &[0.0] } else { &[0.0, 0.5] };
    let invs: &[f64] = if sub { &[0.0, 1.0] } else { &[0.0] };
    for &lam in lams {
        for &inv in invs {
            let Some(steps) = plan(n, t, bl, lam, inv, sub) else { continue };
            let (net, outs) = realize(n, &steps, bl, sub);
            let Some((gates, depth)) = size(n, &net, &outs, sub) else { continue };
            if depth <= t && best.as_ref().is_none_or(|b| gates < b.0) {
                best = Some((gates, net, outs));
            }
        }
    }
    best
}

/// Construction (b): the smallest adder the planner finds that is at most t ticks deep (t >= 3).
/// `sub`: for a - b. The result is then an adder of "not a" and b, to be handed to `minus`.
pub fn within(n: usize, t: usize, sub: bool) -> Option<(Net, Vec<u32>)> {
    let mut best: Option<(Built, Blocks)> = None;
    for bl in block_sets(n, t) {
        if let Some(r) = try_blocks(n, t, &bl, sub) {
            if best.as_ref().is_none_or(|b| r.0 < b.0.0) {
                best = Some((r, bl));
            }
        }
    }
    // then a short local search over the group generates (add, remove, or replace a group), while the work
    // stays small: it pays at low limits, where the groups carry the circuit
    let mut budget: i64 = if t <= 6 { (6000 / (n * n)) as i64 } else { 0 };
    let kinds: &[(Kind, usize)] = if t == 4 { &[(Kind::X, 2)] } else { &[(Kind::X, 2), (Kind::T, 3)] };
    let mut improved = true;
    while improved && budget > 0 && best.is_some() {
        improved = false;
        let mut bl = best.as_ref().unwrap().1.clone();
        for u in 0..n {
            for v in u + 2..=n.min(u + 6) {
                for &(kind, d) in kinds {
                    if (kind == Kind::X && v - u > 3) || budget <= 0 {
                        continue;
                    }
                    let mut b2 = bl.clone();
                    if let Some(pos) = b2.iter().position(|b| b.0 == (u, v)) {
                        b2.remove(pos);
                    } else {
                        b2.retain(|b| b.0.1 <= u || b.0.0 >= v);
                        b2.push(((u, v), d, kind));
                    }
                    budget -= 1;
                    if let Some(r) = try_blocks(n, t, &b2, sub) {
                        if r.0 < best.as_ref().unwrap().0.0 {
                            bl = b2.clone();
                            best = Some((r, b2));
                            improved = true;
                        }
                    }
                }
            }
        }
    }
    best.map(|b| (b.0.1, b.0.2))
}
