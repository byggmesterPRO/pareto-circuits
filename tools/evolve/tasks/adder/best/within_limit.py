"""Adder for Scrap Mechanic logic, as one function that a search can rewrite.

The task: build(n, max_ticks) returns a circuit that adds two n-bit numbers, is at most max_ticks deep, and
uses as few gates as possible.

Rules of the game's logic:
- Gate modes: AND, OR, XOR, NAND, NOR, XNOR. A gate takes ANY number of inputs (up to 255) and costs the same
  whatever the number. XOR is on when an odd number of its inputs are on. A wide gate is as cheap as a narrow
  one, so the best circuits here do not look like textbook two-input circuits.
- A signal may feed at most 255 gates.
- Ticks: every gate on a path costs one tick. The depth of the circuit is the longest path, counted in gates.
  The inputs themselves cost nothing.
- Size: the number of gates. The inputs are not counted.

Pins: a is input bits 0 .. n-1, b is input bits n .. 2n-1, lowest bit first. Outputs: the n sum bits, lowest
first, then the carry out.

Known so far: 4 gates per bit with a full adder per bit; 3.5 per bit with the 7-gate block for two bits below
(exact search: 6 gates for two bits with carry in and out is impossible); 3 ticks at any size with carry
lookahead, at about n*n/2 gates. Nothing good is known in between: a circuit that must finish in, say, n ticks
is built here as the full lookahead, which is far bigger than it needs to be.
"""
AND, OR, XOR, NAND, NOR, XNOR = range(6)


class Net:
    """A circuit under construction. gate() returns the new gate's signal number."""

    def __init__(self, inputs):
        self.inputs, self.gates = inputs, []

    def gate(self, mode, *signals):
        self.gates.append([mode, list(signals)])
        return self.inputs + len(self.gates) - 1

    def done(self, outputs):
        return {"inputs": self.inputs, "gates": self.gates, "outputs": list(outputs)}


# EVOLVE-BLOCK-START
class Hb:
    """Hash-consed gate builder that tracks depth."""

    def __init__(self, n):
        self.n, self.gates, self.key, self.dep = n, [], {}, [0] * (2 * n)

    def g(self, mode, *ins):
        ins = tuple(sorted(ins))
        k = (mode, ins)
        if k not in self.key:
            self.gates.append([mode, list(ins)])
            self.key[k] = 2 * self.n + len(self.gates) - 1
            self.dep.append(1 + max(self.dep[i] for i in ins))
        return self.key[k]


def make_cover(n, blocks, lam):
    """blocks: {(u, v): depth}. cover(lo, j, D) -> (cost, segs) or None: XOR-terms for G[lo, j) of depth <= D.
    segs: list of (u, w, top); top means the segment ends at j and is used directly."""
    ends = {}
    for (u, v), d in blocks.items():
        ends.setdefault(v, []).append((u, d))
    memo = {}

    def cover(lo, j, D):
        key = (lo, j, D)
        if key in memo:
            return memo[key]
        INF = float("inf")
        f = [INF] * (j + 1)
        back = [None] * (j + 1)
        f[lo] = 0
        for w in range(lo + 1, j):
            if D >= 2 and f[w - 1] + 1 < f[w]:
                f[w], back[w] = f[w - 1] + 1, w - 1
            for u, d in ends.get(w, ()):
                if u >= lo and max(d, 1) + 1 <= D and f[u] + 1 + lam < f[w]:
                    f[w], back[w] = f[u] + 1 + lam, u
        best = None
        if D >= 1 and f[j - 1] < INF:
            best = (f[j - 1] + 1, j - 1)
        for u, d in ends.get(j, ()):
            if u >= lo and d <= D and f[u] < INF and (best is None or f[u] + lam < best[0]):
                best = (f[u] + lam, u)
        if best is None:
            memo[key] = None
            return None
        segs = [(best[1], j, True)]
        w = best[1]
        while w > lo:
            segs.append((back[w], w, False))
            w = back[w]
        memo[key] = (best[0], segs)
        return memo[key]

    return cover


def prune(states):
    """Drop states dominated in (cost, source depth, previous sum depth, propagates already paid)."""
    groups = {}
    for st, val in states.items():
        groups.setdefault(st[:3], []).append((st, val))
    out = {}
    for items in groups.values():
        items.sort(key=lambda x: x[1][0])
        kept = []
        for st, val in items:
            c = val[0]
            dom = False
            for st2, val2 in kept:
                if st2[3] <= st[3] and st2[4] <= st[4] and val2[0] + max(0, st[5] - st2[5]) <= c:
                    dom = True
                    break
            if not dom:
                kept.append((st, val))
        for st, val in kept:
            out[st] = val
    return out


def plan(n, T, blocks, lam, W=8, beam=400):
    """DP over the bits. State: (source position, source kind, source terms, source depth, depth of the
    previous sum, highest propagate already built). Source kinds: 0 none, 1 a carry signal, 2 the sum at
    the source bit (with t = a OR b there), 3 a carry as a list of terms."""
    cover = make_cover(n, blocks, lam)
    front = {}

    def la_opts(lo, j):
        k = (lo, j)
        if k not in front:
            opts, last = [], None
            if lo == 0 and 2 <= j <= 4:
                # every monomial of the carry, all at depth 1
                opts.append((1, 2 ** j - 1, "EXP"))
                last = 2 ** j - 1
            for D in range(1, T):
                r = cover(lo, j, D)
                if r is not None and (last is None or r[0] < last):
                    opts.append((D, r[0], r[1]))
                    last = r[0]
            front[k] = opts
        return front[k]

    states = {(0, 0, 0, 0, 1, 0): (1, None)}
    hist = []
    for j in range(1, n + 1):
        new = {}
        hist.append(states)
        for st, (cost, _) in states.items():
            src, kind, ksrc, dsrc, dprev, pk = st
            opts = []
            if j >= 2:
                opts.append((("R",), 2, dprev + 1, 3, pk))
            plo = src if src > 0 else 1
            newp = max(0, (j - 1) - max(pk, plo - 1))
            if kind == 2:
                # terms: AND(s_src, p_src..p_{j-1}), AND(t_src, p_src+1..p_{j-1}) (t itself when j = src+1),
                # and the g-terms over (src, j)
                if j == src + 1:
                    opts.append((("S", None), 1 + newp, max(1, dsrc + 1), 2, max(pk, j - 1)))
                else:
                    for D, c, segs in la_opts(src + 1, j):
                        opts.append((("S", segs), c + 2 + newp, max(D, 2, dsrc + 1), len(segs) + 2, max(pk, j - 1)))
            else:
                for D, c, segs in la_opts(src, j):
                    L, k = D, (c if segs == "EXP" else len(segs))
                    if src > 0:
                        L, c, k = max(D, dsrc + 1), c + ksrc, k + ksrc
                    opts.append((("L", segs), c + newp, L, k, max(pk, j - 1)))
            for tag, c, L, k, pk2 in opts:
                if j < n:
                    if L + 1 > T:
                        continue
                    base = cost + c + 1
                    if src > 0 and j + 1 - src > W:
                        # a source this far back is never worth a lookahead: forget it
                        ch = [((0, 0, 0, 0, L + 1, 0), base, (tag, "drop"))]
                    else:
                        ch = [((src, kind, ksrc, dsrc, L + 1, pk2), base, (tag, "keep"))]
                    ch.append(((j, 2, 1, L + 1, L + 1, j - 1), base + 1, (tag, "sum")))
                    if k == 1:
                        ch.append(((j, 1, 1, L, L + 1, j - 1), base, (tag, "mat")))
                    else:
                        ch.append(((j, 3, k, L, L + 1, j - 1), base, (tag, "virt")))
                    for ns, nc, how in ch:
                        if ns not in new or nc < new[ns][0]:
                            new[ns] = (nc, (st, how))
                else:
                    if L + (1 if k > 1 else 0) > T:
                        continue
                    nc = cost + c + (1 if k > 1 else 0)
                    if ("end",) not in new or nc < new[("end",)][0]:
                        new[("end",)] = (nc, (st, (tag, "end")))
        if j < n:
            new = prune(new)
            if len(new) > beam:
                # keep the cheapest and the shallowest, deterministically
                items = sorted(new.items(), key=lambda x: (x[1][0], x[0][4], str(x[0])))
                keep = dict(items[:beam // 2])
                for st, val in sorted(items, key=lambda x: (x[0][4], x[1][0], str(x[0]))):
                    if len(keep) >= beam:
                        break
                    keep[st] = val
                new = keep
        states = new
    if ("end",) not in states:
        return None
    steps, st, cur = [], ("end",), states
    for j in range(n, 0, -1):
        _, (prev, how) = cur[st]
        steps.append((j, how))
        st, cur = prev, hist[j - 1]
    steps.reverse()
    return steps


def realize(n, steps, kinds):
    net = Hb(n)
    p = {}

    def P(i):
        if i not in p:
            p[i] = net.g(XOR, i, n + i)
        return p[i]

    X = {}

    def blk(u, v):
        if (u, v) not in X:
            if kinds[(u, v)] == "x":
                mons = []
                for i in range(u, v):
                    combos = [[i, n + i]]
                    for k in range(i + 1, v):
                        combos = [c + [k] for c in combos] + [c + [n + k] for c in combos]
                    mons += [net.g(AND, *c) for c in combos]
                X[(u, v)] = net.g(XOR, *mons)
            else:
                X[(u, v)] = net.g(XOR, *[net.g(AND, i, n + i, *[P(k) for k in range(i + 1, v)]) for i in range(u, v)])
        return X[(u, v)]

    def segterms(segs, j):
        C = []
        if segs == "EXP":
            for i in range(j):
                combos = [[i, n + i]]
                for k in range(i + 1, j):
                    combos = [c + [k] for c in combos] + [c + [n + k] for c in combos]
                C += [net.g(AND, *c) for c in combos]
            return C
        for u, w, top in segs or ():
            ps = [P(k) for k in range(w, j)]
            if w - u == 1:
                C.append(net.g(AND, u, n + u, *ps))
            else:
                x = blk(u, w)
                C.append(x if top else net.g(AND, x, *ps))
        return C

    outs = [P(0)]
    src, skind, srcsig = 0, 0, None
    for j, (tag, how) in steps:
        if tag[0] == "R":
            s = outs[j - 1]
            C = [s, net.g(OR, j - 1, n + j - 1, s), net.g(AND, j - 1, n + j - 1, s)]
        elif tag[0] == "S":
            s, t = srcsig
            C = segterms(tag[1], j)
            C.append(net.g(AND, s, *[P(k) for k in range(src, j)]))
            C.append(t if j == src + 1 else net.g(AND, t, *[P(k) for k in range(src + 1, j)]))
        else:
            C = segterms(tag[1], j)
            if src > 0:
                ps = [P(k) for k in range(src, j)]
                C += [net.g(AND, t, *ps) for t in srcsig]
        if how == "end":
            outs.append(C[0] if len(C) == 1 else net.g(XOR, *C))
            break
        outs.append(net.g(XOR, j, n + j, *C))
        if how == "mat":
            src, skind, srcsig = j, 1, [C[0] if len(C) == 1 else net.g(XOR, *C)]
        elif how == "virt":
            src, skind, srcsig = j, 3, C
        elif how == "sum":
            src, skind, srcsig = j, 2, (outs[j], net.g(OR, j, n + j))
        elif how == "drop":
            src, skind, srcsig = 0, 0, None
    return {"inputs": 2 * n, "gates": net.gates, "outputs": outs}


def ripple(n):
    """3 gates a bit: the carry out of bit i is s XOR (a OR b OR s) XOR (a AND b AND s), s the sum of bit i."""
    net = Hb(n)
    outs, C = [net.g(XOR, 0, n)], [net.g(AND, 0, n)]
    for i in range(1, n):
        s = net.g(XOR, i, n + i, *C)
        outs.append(s)
        C = [s, net.g(OR, i, n + i, s), net.g(AND, i, n + i, s)]
    outs.append(C[0] if len(C) == 1 else net.g(XOR, *C))
    return {"inputs": 2 * n, "gates": net.gates, "outputs": outs}


def block_sets(n, T):
    """Candidate sets of group generates: none, or a tiling by one size with an offset."""
    yield {}, {}
    if T > 6:
        return
    small = n <= 12
    # the first few groups expanded to depth 2, then nothing
    if T == 4:
        for b in (2, 3):
            for r in (range(1, n // b + 1) if small else (1, 2, n // b)):
                bl = {(i * b, i * b + b): 2 for i in range(r)}
                yield bl, {k: "x" for k in bl}
    for b in range(2, 6):
        for kind, d in (("x", 2), ("t", 3)):
            if (kind == "x") != (T == 4) or (kind == "x" and b > 3):
                continue
            if not small and abs(b * b - n // 2) > min(abs(c * c - n // 2) for c in range(2, 6)):
                continue
            for o in (range(b) if small else (0,)):
                cuts = sorted(set([0] + list(range(o, n + 1, b)) + [n]))
                bl, kd = {}, {}
                for u, v in zip(cuts, cuts[1:]):
                    if v - u >= 2:
                        bl[(u, v)], kd[(u, v)] = d, kind
                if bl:
                    yield bl, kd


def depth_of(net, n):
    dep = [0] * (2 * n)
    for mode, ins in net["gates"]:
        dep.append(1 + max(dep[i] for i in ins))
    return max(dep[o] for o in net["outputs"])


def try_blocks(n, T, bl, kd):
    best = None
    for lam in ((0.0, 0.5) if bl else (0.0,)):
        steps = plan(n, T, bl, lam)
        if steps is None:
            continue
        net = realize(n, steps, kd)
        if depth_of(net, n) <= T and (best is None or len(net["gates"]) < len(best["gates"])):
            best = net
    return best


def best_for(n, T):
    best, where = None, None
    for bl, kd in block_sets(n, T):
        net = try_blocks(n, T, bl, kd)
        if net is not None and (best is None or len(net["gates"]) < len(best["gates"])):
            best, where = net, (bl, kd)
    # then a short local search over the group generates (add, remove, or replace a group), while the
    # work stays small: it pays at low limits, where the groups carry the circuit
    budget = 6000 // (n * n) if T <= 6 else 0
    kinds = [("x", 2)] if T == 4 else [("x", 2), ("t", 3)]
    improved = True
    while improved and budget > 0 and best is not None:
        improved = False
        bl, kd = where
        for u in range(n):
            for v in range(u + 2, min(n, u + 6) + 1):
                for kind, d in kinds:
                    if (kind == "x" and v - u > 3) or budget <= 0:
                        continue
                    b2, k2 = dict(bl), dict(kd)
                    if (u, v) in b2:
                        del b2[(u, v)], k2[(u, v)]
                    else:
                        for o in list(b2):
                            if not (o[1] <= u or o[0] >= v):
                                del b2[o], k2[o]
                        b2[(u, v)], k2[(u, v)] = d, kind
                    budget -= 1
                    net = try_blocks(n, T, b2, k2)
                    if net is not None and len(net["gates"]) < len(best["gates"]):
                        best, where, improved = net, (b2, k2), True
                        bl, kd = b2, k2
    return best


def build(n, max_ticks):
    """Smallest of several DP plans: per bit, the 3-gate ripple step (carry read back from the sum with
    OR/AND) or lookahead from a source (a carry, or the sum at the source bit), optionally over tiled group
    generates. Any circuit for a smaller limit also fits, so the best over all limits up to max_ticks wins."""
    if max_ticks is None or max_ticks >= 2 * n:
        return ripple(n)
    return best_for(n, max(3, max_ticks))
# EVOLVE-BLOCK-END


def run_experiment(n, max_ticks):
    """Called by the evaluator, once per case."""
    return {"n": n, "max_ticks": max_ticks, "netlist": build(n, max_ticks)}
