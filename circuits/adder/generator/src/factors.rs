//! Adders of 3 and 4 ticks at any size, ported from the search result
//! tools/evolve/tasks/adder/runs/fast/cand_06.py.
//!
//! The carry into bit i is the XOR of pieces that are never on two at a time, one per block of a partition
//! of the bits below i: "block B starts a carry and every bit between B and i passes it on", which is
//! AND(p above B, G_B). For G_B there are three ways:
//! - a single bit j: G = a_j and b_j, so the piece is AND(a_j, b_j, p above), depth 2.
//! - a small "clauses" block: G_B is an AND of OR gates over the raw inputs (its prime implicates). The ORs
//!   are shared by every bit above, and the piece is AND(the clauses, p above), depth 2.
//! - a "factors" block x .. y: G_B is an AND of depth-2 factors, OR(g_x .. g_y) and, for every bit m above x,
//!   OR(g_m+1 .. g_y, a_m, b_m) ("bit m does not kill, or a bit above it starts a carry"). The factors are
//!   shared by every bit above y, and the piece is one AND of them with p above, depth 3.
//!
//! A sum bit is XOR(p_i, pieces): 3 ticks with single bits and clause blocks, 4 with factor blocks.

use crate::net::ticks;
use logic_sim::build::{Builder, Netlist, S};
use crate::net::{FxMap as HashMap, FxSet as HashSet};
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Block {
    Clauses,
    Factors,
}

type Blocks = BTreeMap<(usize, usize), Block>;

#[derive(Hash, PartialEq, Eq)]
enum Key {
    G(usize),
    C(Rc<Vec<usize>>),
    Fb(usize),
    F0(usize, usize),
    F(usize, usize),
}

struct Ctx {
    n: usize,
    clauses: HashMap<(usize, usize), Rc<Vec<Rc<Vec<usize>>>>>,
}

impl Ctx {
    /// The prime implicates of "a carry starts in bits x .. y and gets out", as sorted lists of inputs.
    fn clauses_of(&mut self, x: usize, y: usize) -> Rc<Vec<Rc<Vec<usize>>>> {
        let n = self.n;
        self.clauses
            .entry((x, y))
            .or_insert_with(|| {
                fn rec(n: usize, x: usize, top: usize) -> Vec<Vec<usize>> {
                    let (a, b) = (top, n + top);
                    if top == x {
                        return vec![vec![a], vec![b]];
                    }
                    let sub = rec(n, x, top - 1);
                    let mut out = vec![vec![a, b]];
                    out.extend(sub.iter().map(|c| [&[a], c.as_slice()].concat()));
                    out.extend(sub.iter().map(|c| [&[b], c.as_slice()].concat()));
                    out
                }
                Rc::new(
                    rec(n, x, y)
                        .into_iter()
                        .map(|mut c| {
                            c.sort_unstable();
                            Rc::new(c)
                        })
                        .collect(),
                )
            })
            .clone()
    }

    /// The cheapest way down from every bit: choice[e] is where the segment ending at bit e - 1 starts.
    fn choices(&self, blocks: &Blocks) -> Vec<usize> {
        let n = self.n;
        let mut ends: HashMap<usize, Vec<usize>> = HashMap::default();
        for &(x, y) in blocks.keys() {
            ends.entry(y + 1).or_default().push(x);
        }
        let (mut best, mut choice) = (vec![0usize; n + 1], vec![0usize; n + 1]);
        for i in 1..=n {
            best[i] = best[i - 1] + 1;
            choice[i] = i - 1;
            if let Some(xs) = ends.get_mut(&i) {
                xs.sort_unstable();
                for &x in xs.iter() {
                    if best[x] + 1 < best[i] {
                        best[i] = best[x] + 1;
                        choice[i] = x;
                    }
                }
            }
        }
        choice
    }

    /// The gate count of `build(blocks, bot)`, without building it.
    fn plan_cost(&mut self, blocks: &Blocks, bot: usize) -> usize {
        let n = self.n;
        let choice = self.choices(blocks);
        let mut shared: HashSet<Key> = HashSet::default();
        let mut pieces = 0usize;
        let mut used: HashSet<(usize, usize)> = HashSet::default();
        let mut need = vec![false; n + 1];
        need[0] = true;
        if bot >= 2 {
            pieces += (1 << bot) - 2; // the implicants of c_bot other than g_bot-1
            shared.insert(Key::G(bot - 1));
        }
        for i in 1..=n {
            if i == bot && bot >= 2 {
                continue;
            }
            let mut e = i;
            while e > 0 {
                let (x, y) = (choice[e], e - 1);
                if x == y && y == i - 1 {
                    shared.insert(Key::G(x));
                } else {
                    pieces += 1;
                    if x != y {
                        used.insert((x, y));
                    }
                }
                e = x;
            }
            let mut e = i;
            while choice[e] > 0 {
                e = choice[e];
            }
            need[e..i].iter_mut().for_each(|x| *x = true); // the bottom segment is 0 .. e-1: p_e .. p_i-1 are used
        }
        for (x, y) in used {
            if blocks[&(x, y)] == Block::Clauses {
                for c in self.clauses_of(x, y).iter() {
                    if c.len() > 1 {
                        shared.insert(Key::C(c.clone()));
                    }
                }
            } else {
                let lo = if x == 0 && bot >= 2 && y + 1 >= bot {
                    shared.insert(Key::Fb(y));
                    bot
                } else {
                    shared.insert(Key::F0(x, y));
                    x + 1
                };
                for j in x..=y {
                    shared.insert(Key::G(j));
                }
                for m in lo..=y {
                    shared.insert(if m == y { Key::C(Rc::new(vec![y, n + y])) } else { Key::F(m, y) });
                }
            }
        }
        let (mut last, mut e) = (0, n);
        while e > 0 {
            last += 1;
            e = choice[e];
        }
        let outs = (n - 1) + if last > 1 || bot == n { 1 } else { 0 };
        need.iter().filter(|&&x| x).count() + shared.len() + pieces + outs
    }

    /// Clause blocks from the bottom up: c4 blocks of 4 bits, then c3 of 3, then c2 of 2, then single bits.
    /// The counts are chosen by trying them all. `whole`: the carry out is one factor block over all bits.
    fn clause_family(&mut self, bot: usize, whole: bool) -> Blocks {
        let n = self.n;
        let mut base = Blocks::new();
        if whole {
            base.insert((0, n - 1), Block::Factors);
        }
        let mut best = (self.plan_cost(&base, bot), base.clone());
        for c4 in 0..=(if n <= 24 { n / 4 } else { n / 8 }) {
            for c3 in 0..=(n - 4 * c4) / 3 {
                for c2 in 0..=(n - 4 * c4 - 3 * c3) / 2 {
                    let mut bl = base.clone();
                    let mut x = 0;
                    for (l, c) in [(4, c4), (3, c3), (2, c2)] {
                        for _ in 0..c {
                            bl.insert((x, x + l - 1), Block::Clauses);
                            x += l;
                        }
                    }
                    let v = self.plan_cost(&bl, bot);
                    if v < best.0 {
                        best = (v, bl);
                    }
                }
            }
        }
        best.1
    }

    fn levels_plan(&self, sizes: &[usize], cumulative: bool) -> Blocks {
        fn rec(blocks: &mut Blocks, sizes: &[usize], cumulative: bool, s: usize, e: usize, lv: usize) {
            if lv >= sizes.len() {
                return;
            }
            let k = sizes[lv];
            let (mut prev, mut b) = (s, s + k);
            while b <= e {
                if b - 1 > s {
                    blocks.insert((if cumulative { s } else { prev }, b - 1), Block::Factors);
                }
                rec(blocks, sizes, cumulative, prev, b.min(e), lv + 1);
                prev = b;
                b += k;
            }
            rec(blocks, sizes, cumulative, prev, e, lv + 1);
        }
        let mut blocks = Blocks::new();
        rec(&mut blocks, sizes, cumulative, 0, self.n, 0);
        blocks
    }

    /// Nested levels of factor blocks (sizes K1 > K2 > K3), cumulative from the start of the segment or not.
    /// The best two-level plans are then tried with a third level.
    fn factor_family(&mut self, bot: usize) -> Vec<Blocks> {
        const TOP: usize = 4;
        let n = self.n;
        let ks: Vec<usize> = if n <= 24 {
            (2..=n).collect()
        } else {
            let mut ks: Vec<usize> = (2..13).chain((13..=n).step_by((n / 24).max(1))).collect();
            ks.sort_unstable();
            ks.dedup();
            ks
        };
        let mut scored: Vec<(usize, Vec<usize>, bool, Blocks)> = vec![];
        for &k1 in &ks {
            let k2s: Vec<Option<usize>> = std::iter::once(None).chain(ks.iter().filter(|&&k| k < k1).map(|&k| Some(k))).collect();
            for k2 in k2s {
                for cum in [true, false] {
                    let sizes: Vec<usize> = [Some(k1), k2].into_iter().flatten().collect();
                    let bl = self.levels_plan(&sizes, cum);
                    scored.push((self.plan_cost(&bl, bot), sizes, cum, bl));
                }
            }
        }
        scored.sort_by_key(|r| r.0);
        let mut out: Vec<(usize, Blocks)> = vec![(self.plan_cost(&Blocks::new(), bot), Blocks::new())];
        out.extend(scored.iter().take(TOP).map(|r| (r.0, r.3.clone())));
        for (_, sizes, cum, _) in scored.iter().take(6) {
            if sizes.len() < 2 {
                continue;
            }
            for &k3 in ks.iter().filter(|&&k| k < sizes[1]) {
                let bl3 = self.levels_plan(&[sizes[0], sizes[1], k3], *cum);
                out.push((self.plan_cost(&bl3, bot), bl3));
            }
        }
        out.sort_by_key(|r| r.0);
        out.into_iter().take(TOP).map(|r| r.1).collect()
    }

    /// The adder for these blocks. bot = k >= 2: bits 0 .. k-1 are one "super bit" whose generate is the carry
    /// into bit k written as its implicants (depth 1). A factor block 0 .. y then needs one factor for the
    /// whole super bit, and the sum at bit k is the XOR of the implicants.
    fn build(&mut self, blocks: &Blocks, bot: usize) -> Result<Netlist, String> {
        let n = self.n;
        let mut b = Builder::new(2 * n);
        let inp: Vec<S> = (0..2 * n).map(|i| b.input(i)).collect();
        let choice = self.choices(blocks);
        let mut p: Vec<Option<S>> = vec![None; n];
        let mut pget = |b: &mut Builder, j: usize| -> S { *p[j].get_or_insert_with(|| b.xor(&[inp[j], inp[n + j]])) };
        let d: Vec<S> = if bot >= 2 { bottom_terms(n, bot).iter().map(|t| b.and(&t.iter().map(|&i| inp[i]).collect::<Vec<_>>())).collect() } else { vec![] };
        let mut terms: Vec<Vec<S>> = vec![vec![]];
        for i in 1..=n {
            if i == bot && bot >= 2 {
                terms.push(d.clone());
                continue;
            }
            let mut t = vec![];
            let mut e = i;
            while e > 0 {
                let (x, y) = (choice[e], e - 1);
                let above: Vec<S> = (y + 1..i).map(|j| pget(&mut b, j)).collect();
                let piece = if x == y {
                    b.and(&[&[inp[x], inp[n + x]], above.as_slice()].concat())
                } else if blocks[&(x, y)] == Block::Clauses {
                    let mut xs: Vec<S> = self.clauses_of(x, y).iter().map(|c| if c.len() == 1 { inp[c[0]] } else { b.or(&c.iter().map(|&i| inp[i]).collect::<Vec<_>>()) }).collect();
                    xs.extend(above);
                    b.and(&xs)
                } else {
                    let gg = |b: &mut Builder, j: usize| b.and(&[inp[j], inp[n + j]]);
                    let (mut fs, lo) = if x == 0 && bot >= 2 && y + 1 >= bot {
                        let mut xs: Vec<S> = (bot..=y).map(|j| gg(&mut b, j)).collect();
                        xs.extend(&d);
                        (vec![b.or(&xs)], bot)
                    } else {
                        let xs: Vec<S> = (x..=y).map(|j| gg(&mut b, j)).collect();
                        (vec![b.or(&xs)], x + 1)
                    };
                    for m in lo..=y {
                        let mut xs: Vec<S> = (m + 1..=y).map(|j| gg(&mut b, j)).collect();
                        xs.extend([inp[m], inp[n + m]]);
                        fs.push(b.or(&xs));
                    }
                    if fs.len() + above.len() > 1 {
                        fs.extend(above);
                        b.and(&fs)
                    } else {
                        fs[0]
                    }
                };
                t.push(piece);
                e = x;
            }
            terms.push(t);
        }
        let mut outs = vec![pget(&mut b, 0)];
        for i in 1..=n {
            if i < n {
                let mut xs = match p[i] {
                    Some(s) => vec![s],
                    None => vec![inp[i], inp[n + i]],
                };
                xs.extend(&terms[i]);
                outs.push(b.xor(&xs));
            } else {
                outs.push(if terms[i].len() == 1 { terms[i][0] } else { b.xor(&terms[i]) });
            }
        }
        b.finish(&outs)
    }
}

/// The carry into bit k as its prime implicants: g_j and, for every bit above j, a or b. Their OR is the
/// carry, and since they come in the pattern of the lookahead sum, their XOR is the carry too.
fn bottom_terms(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut terms = vec![];
    for j in 0..k {
        for pick in 0..1usize << (k - 1 - j) {
            let mut lits = vec![j, n + j];
            lits.extend((j + 1..k).map(|m| m + if pick >> (m - j - 1) & 1 == 1 { n } else { 0 }));
            terms.push(lits);
        }
    }
    terms
}

/// The smallest adder of at most `max_ticks` (3 or 4) that the plans here give: clause blocks for 3 ticks,
/// factor blocks for 4. None for n < 2.
pub fn fast(n: usize, max_ticks: usize) -> Option<Netlist> {
    if n < 2 {
        return None;
    }
    let mut ctx = Ctx { n, clauses: HashMap::default() };
    let bots: Vec<usize> = if n <= 32 { [0, 2, 3].into_iter().filter(|&b| b < n).collect() } else { vec![2] };
    let mut cands: Vec<(Blocks, usize)> = vec![];
    for &bot in &bots {
        if max_ticks >= 4 {
            cands.extend(ctx.factor_family(bot).into_iter().map(|bl| (bl, bot)));
        } else {
            for whole in if n <= 24 { vec![false, true] } else { vec![false] } {
                cands.push((ctx.clause_family(bot, whole), bot));
            }
        }
    }
    let mut best: Option<Netlist> = None;
    for (bl, bot) in cands {
        let Ok(net) = ctx.build(&bl, bot) else { continue };
        if ticks(&net) <= max_ticks && best.as_ref().is_none_or(|b| net.gates.len() < b.gates.len()) {
            best = Some(net);
        }
    }
    best
}
