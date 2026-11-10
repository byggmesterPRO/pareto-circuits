"""Shrink a gate netlist by resubstitution with don't cares.

For each gate g: work out on which inputs its value matters at all (flip it, see which outputs change), then try
to rebuild it from signals that already exist, with 1, 2 or 3 new gates. If that takes fewer gates than the logic
that only g needs, swap it in. Gates take any number of inputs, so "g is the AND of every signal that covers it"
and "g is an XOR of some set of signals" (linear algebra over GF(2)) are single gates. A second move swaps a
signal for a different one-gate signal and rewrites the gates that read it, which lets the encoding between two
stages change.

It only looks at gate count, so the result is usually slow. Every input is tried, so it stops at 16 input bits.
A signal is a Python int with one bit per input value.

    python resub.py <family> <size> <start netlist.json> <out netlist.json> [seed] [seconds] [max new gates]

The out file is rewritten each time a smaller circuit is found. Check it with tools/bygg-logic-sim/verify.py.
"""
import random
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "tools" / "bygg-logic-sim"))
import netlist
from families import FAMILIES

AND, OR, XOR, NAND, NOR, XNOR = range(6)
N = ROWS = FULL = N_OUT = 0
X, TARGET, OUT_BITS = [], [], []


def setup(family, size):
    """Input vectors and target vectors for one circuit. Output bits that are always 0 are left out."""
    global N, ROWS, FULL, N_OUT, X, TARGET, OUT_BITS
    N, N_OUT, f = FAMILIES[family](size)
    if N > 16:
        raise SystemExit("resub tries every input, so it stops at 16 input bits")
    ROWS = 1 << N
    FULL = (1 << ROWS) - 1
    X = [sum(((x >> i) & 1) << x for x in range(ROWS)) for i in range(N)]
    want = [f(x) for x in range(ROWS)]
    cols = [sum(((w >> b) & 1) << x for x, w in enumerate(want)) for b in range(N_OUT)]
    OUT_BITS = [b for b in range(N_OUT) if cols[b]]
    TARGET = [cols[b] for b in OUT_BITS]


class Net:
    """gates: {id: (mode, [signal ids])}; signals 0..11 are inputs; outs: one signal id per target."""

    def __init__(self, gates, outs):
        self.g = dict(gates)
        self.outs = list(outs)
        self.next = max(list(self.g) + [N]) + 1

    def copy(self):
        n = Net(self.g, self.outs)
        n.next = self.next
        return n

    def order(self):
        seen, out = set(range(N)), []

        def visit(s):
            stack = [(s, iter(self.g[s][1]))]
            seen.add(s)
            while stack:
                node, it = stack[-1]
                for i in it:
                    if i not in seen:
                        seen.add(i)
                        stack.append((i, iter(self.g[i][1])))
                        break
                else:
                    out.append(node)
                    stack.pop()

        for o in self.outs:
            if o not in seen:
                visit(o)
        return out

    def prune(self):
        live = set(self.order())
        self.g = {k: v for k, v in self.g.items() if k in live}

    def sim(self, order=None, val=None, start=None):
        val = dict(enumerate(X)) if val is None else val
        for s in (order or self.order()):
            if start is not None and s not in start:
                continue
            val[s] = fn(self.g[s][0], [val[i] for i in self.g[s][1]])
        return val

    def ok(self, val=None):
        val = val or self.sim()
        return all(val[o] == t for o, t in zip(self.outs, TARGET))

    def size(self):
        return len(self.g)


def fn(mode, vs):
    if not vs:
        return 0
    v = vs[0]
    if mode in (AND, NAND):
        for u in vs[1:]:
            v &= u
    elif mode in (OR, NOR):
        for u in vs[1:]:
            v |= u
    else:
        for u in vs[1:]:
            v ^= u
    return v ^ FULL if mode >= 3 else v


def find1(t, c, D, val, rng, linear=True):
    """One gate over signals in D that equals t wherever c is set. Returns (mode, [signals]) or None."""
    tc, nc = t & c, (t ^ FULL) & c
    for mode, want, covers in ((AND, tc, True), (NAND, nc, True), (OR, tc, False), (NOR, nc, False)):
        if covers:      # AND of everything that contains `want`
            U = [s for s in D if want & ~val[s] == 0]
            acc = FULL
            for s in U:
                acc &= val[s]
        else:           # OR of everything inside `want`
            U = [s for s in D if val[s] & c & ~want == 0]
            acc = 0
            for s in U:
                acc |= val[s]
        if U and acc & c == want:
            rng.shuffle(U)
            U.sort(key=lambda s: s >= N)        # try to drop gates before inputs
            keep = list(U)
            for s in reversed(U):
                rest = [u for u in keep if u != s]
                if not rest:
                    continue
                acc = FULL if covers else 0
                for u in rest:
                    acc = acc & val[u] if covers else acc | val[u]
                if acc & c == want:
                    keep = rest
            return mode, keep
    if linear:
        basis = []      # (vector, combination) with distinct leading bits
        for j, s in enumerate(D):
            v, combo = val[s] & c, 1 << j
            for bv, bc in basis:
                if v ^ bv < v:
                    v ^= bv
                    combo ^= bc
            if v:
                basis.append((v, combo))
                basis.sort(reverse=True)
        for mode, want in ((XOR, tc), (XNOR, nc)):
            v, combo = want, 0
            for bv, bc in basis:
                if v ^ bv < v:
                    v ^= bv
                    combo ^= bc
            if v == 0 and combo:
                return mode, [s for j, s in enumerate(D) if combo >> j & 1]
    return None


def find(t, c, D, val, rng, k, local=()):
    """Up to k new gates computing t on c. Returns a list of (mode, inputs) where input -1 means the gate
    before it in the list; the last gate is the result. None if not found."""
    r = find1(t, c, D, val, rng)
    if r:
        return [r]
    if k < 2:
        return None
    tc, nc = t & c, (t ^ FULL) & c
    tops = [(AND, tc, True), (NAND, nc, True), (OR, tc, False), (NOR, nc, False)]
    rng.shuffle(tops)
    for mode, want, covers in tops:
        if covers:
            U = [s for s in D if want & ~val[s] == 0]
            acc = FULL
            for s in U:
                acc &= val[s]
            c2 = c & acc            # the helper only has to be right where the others are all 1
        else:
            U = [s for s in D if val[s] & c & ~want == 0]
            acc = 0
            for s in U:
                acc |= val[s]
            c2 = c & ~acc           # ... or where the others are all 0
        if c2 == c:
            continue
        target = t if mode in (AND, OR) else t ^ FULL
        sub = find(target, c2, D, val, rng, k - 1, local)
        if sub:
            # keep only the U members that are needed
            hv = eval_chain(sub, val)
            keep = list(U)
            for s in list(U):
                rest = [u for u in keep if u != s]
                acc = hv
                for u in rest:
                    acc = acc & val[u] if covers else acc | val[u]
                if acc & c == want:
                    keep = rest
            return sub + [(mode, keep + [-1])]
    # XOR of existing signals and one small AND/OR of two of them
    basis = []
    for j, s in enumerate(D):
        v, combo = val[s] & c, 1 << j
        for bv, bc in basis:
            if v ^ bv < v:
                v ^= bv
                combo ^= bc
        if v:
            basis.append((v, combo))
            basis.sort(reverse=True)
    def in_span(want_h):
        for mode, want in ((XOR, tc), (XNOR, nc)):
            v, combo = want ^ want_h, 0
            for bv, bc in basis:
                if v ^ bv < v:
                    v ^= bv
                    combo ^= bc
            if v == 0:
                return mode, [s for j, s in enumerate(D) if combo >> j & 1]
        return None

    # ... or one AND/OR over some of the signals the replaced logic was reading
    local = list(local)[:14]
    if len(local) >= 3:
        n_sub = 1 << len(local)
        picks = range(1, n_sub) if n_sub <= 600 else [rng.randrange(1, n_sub) for _ in range(600)]
        for bits in picks:
            sub = [s for j, s in enumerate(local) if bits >> j & 1]
            if len(sub) < 3:
                continue
            a = o = None
            for s_ in sub:
                a = val[s_] if a is None else a & val[s_]
                o = val[s_] if o is None else o | val[s_]
            for hmode, hv in ((AND, a), (OR, o)):
                r = in_span(hv & c)
                if r:
                    return [(hmode, sub), (r[0], r[1] + [-1])]
    pairs = [(a, b) for i, a in enumerate(D) for b in D[i + 1:]]
    rng.shuffle(pairs)
    for a, b in pairs[:6000]:
        h = val[a] & val[b] & c
        for mode, want in ((XOR, tc), (XNOR, nc)):
            v, combo = want ^ h, 0
            for bv, bc in basis:
                if v ^ bv < v:
                    v ^= bv
                    combo ^= bc
            if v == 0:
                return [(AND, [a, b]), (mode, [s for j, s in enumerate(D) if combo >> j & 1] + [-1])]
    return None


def eval_chain(chain, val):
    prev = None
    for mode, ins in chain:
        prev = fn(mode, [prev if i == -1 else val[i] for i in ins])
    return prev


def bypass_buffers(net):
    """A one-input and/or/xor gate is a wire: point its users at its input."""
    changed = True
    while changed:
        changed = False
        for g, (mode, ins) in list(net.g.items()):
            if len(ins) == 1 and mode < 3:
                src = ins[0]
                for k, (m2, i2) in net.g.items():
                    if g in i2:
                        new = [src if i == g else i for i in i2]
                        if len(set(new)) < len(new):      # src was already an input: a duplicate wire
                            if m2 in (XOR, XNOR):
                                continue                    # would cancel out; leave this buffer alone
                            new = list(dict.fromkeys(new))
                        net.g[k] = (m2, new)
                if not any(g in i2 for _, i2 in net.g.values()):
                    net.outs = [src if o == g else o for o in net.outs]
                    del net.g[g]
                    changed = True
    net.prune()


def improve(net, rng, max_new=3, neutral=0.0, uphill=0.0, log=None, deadline=None):
    """One pass over all gates. Returns True if the net got smaller."""
    got = False
    ids = list(net.g)
    rng.shuffle(ids)
    for g in ids:
        if g not in net.g or (deadline and time.time() > deadline):
            continue
        order = net.order()
        if g not in order:
            continue
        val = net.sim(order)
        # fanout cone and the logic only g needs
        users = {}
        for s in order:
            for i in net.g[s][1]:
                users.setdefault(i, []).append(s)
        tfo, stack = {g}, [g]
        while stack:
            for u in users.get(stack.pop(), ()):
                if u not in tfo:
                    tfo.add(u)
                    stack.append(u)
        refs = {s: len(users.get(s, ())) + net.outs.count(s) for s in order}
        mffc, stack = {g}, [g]
        while stack:
            for i in net.g[stack.pop()][1]:
                if i >= N:
                    refs[i] -= 1
                    if refs[i] == 0:
                        mffc.add(i)
                        stack.append(i)
        slack = 0
        if uphill and rng.random() < uphill:
            slack = 1
        budget = len(mffc) - 1 + slack if rng.random() >= neutral else len(mffc) + slack
        if budget < 1:
            continue
        # where does g matter?
        v2 = dict(val)
        v2[g] = val[g] ^ FULL
        net.sim(order, v2, tfo - {g})
        care = 0
        for o in net.outs:
            care |= val[o] ^ v2[o]
        if care == 0:
            continue
        D = [s for s in list(range(N)) + order if s not in tfo and s not in mffc]
        dset = set(D)
        local = sorted({i for m in mffc for i in net.g[m][1] if i in dset})
        chain = find(val[g], care, D, val, rng, min(budget, max_new), local)
        if not chain:
            continue
        trial = net.copy()
        prev = None
        for j, (mode, ins) in enumerate(chain):
            nid = g if j == len(chain) - 1 else trial.next
            if nid != g:
                trial.next += 1
            trial.g[nid] = (mode, [prev if i == -1 else i for i in ins])
            prev = nid
        trial.prune()
        bypass_buffers(trial)
        if trial.ok() and trial.size() <= net.size() + slack:
            if trial.size() < net.size():
                got = True
                if log:
                    log(f"  gate {g}: {net.size()} -> {trial.size()} ({len(chain)} new for {len(mffc)})")
            net.g, net.next, net.outs = trial.g, trial.next, trial.outs
    return got


def prep(t, c, D, val):
    """Everything about "one gate over D equal to t on c" that doesn't depend on an extra signal."""
    tc, nc = t & c, (t ^ FULL) & c
    types = []
    for mode, want, covers in ((AND, tc, True), (NAND, nc, True), (OR, tc, False), (NOR, nc, False)):
        if covers:
            U = [s for s in D if want & ~val[s] == 0]
            acc = FULL
            for s in U:
                acc &= val[s]
        else:
            U = [s for s in D if val[s] & c & ~want == 0]
            acc = 0
            for s in U:
                acc |= val[s]
        types.append((mode, want, covers, U, acc))
    basis = []
    for j, s in enumerate(D):
        v, combo = val[s] & c, 1 << j
        for bv, bc in basis:
            if v ^ bv < v:
                v ^= bv
                combo ^= bc
        if v:
            basis.append((v, combo))
            basis.sort(reverse=True)

    def red(v):
        combo = 0
        for bv, bc in basis:
            if v ^ bv < v:
                v ^= bv
                combo ^= bc
        return v, combo

    return {"c": c, "D": D, "types": types, "red": red, "rt": red(tc), "rn": red(nc)}


def with_new(info, sv, val, rng):
    """One gate over D and a new signal whose value is sv. Returns (mode, [signals], uses_new) or None."""
    c = info["c"]
    for mode, want, covers, U, acc in info["types"]:
        if covers:
            fits = want & ~sv == 0
            total = acc & sv if fits else acc
        else:
            fits = sv & c & ~want == 0
            total = acc | sv if fits else acc
        if total & c != want or not (U or fits):
            continue
        keep = list(U)
        rng.shuffle(keep)
        keep.sort(key=lambda s: s >= N)
        for s in reversed(list(keep)):          # drop inputs that aren't needed, gates first
            rest = [u for u in keep if u != s]
            a = sv if fits else (FULL if covers else 0)
            for u in rest:
                a = a & val[u] if covers else a | val[u]
            if a & c == want and (rest or fits):
                keep = rest
        used = fits
        if fits and keep:                       # is the new signal itself needed?
            a = FULL if covers else 0
            for u in keep:
                a = a & val[u] if covers else a | val[u]
            if a & c == want:
                used = False
        return mode, keep, used
    rv, cv = info["red"](sv & c)
    for mode, (rt, ct) in ((XOR, info["rt"]), (XNOR, info["rn"])):
        if rt == 0 and ct:
            return mode, [s for j, s in enumerate(info["D"]) if ct >> j & 1], False
        if rv and rt == rv:
            return mode, [s for j, s in enumerate(info["D"]) if (ct ^ cv) >> j & 1], True
    return None


def reencode(net, rng, neutral=0.0, deadline=None):
    """Swap a signal for a different one-gate signal and rewrite the gates that read it, when that frees the
    logic the old signal needed. This is how the encoding of a value between two stages can change."""
    got = False
    ids = list(net.g)
    rng.shuffle(ids)
    for s in ids:
        if s not in net.g or s in net.outs or (deadline and time.time() > deadline):
            continue
        order = net.order()
        if s not in order:
            continue
        users = {}
        for k in order:
            for i in net.g[k][1]:
                users.setdefault(i, []).append(k)
        cons = users.get(s, [])
        if not cons or len(cons) > 4:
            continue
        tfo, stack = {s}, [s]
        while stack:
            for u in users.get(stack.pop(), ()):
                if u not in tfo:
                    tfo.add(u)
                    stack.append(u)
        refs = {k: len(users.get(k, ())) + net.outs.count(k) for k in order}
        mffc, stack = {s}, [s]
        while stack:
            for i in net.g[stack.pop()][1]:
                if i >= N:
                    refs[i] -= 1
                    if refs[i] == 0:
                        mffc.add(i)
                        stack.append(i)
        need = 0 if rng.random() < neutral else 1
        if len(mffc) - 1 < need:
            continue
        D = [k for k in list(range(N)) + order if k not in tfo and k not in mffc]
        dset = set(D)
        local = sorted({i for m in mffc for i in net.g[m][1] if i in dset})
        if len(local) < 2 or len(local) > 16:
            continue
        val = net.sim(order)
        infos = []
        for u in cons:
            tfo_u, stack = {u}, [u]
            while stack:
                for k in users.get(stack.pop(), ()):
                    if k not in tfo_u:
                        tfo_u.add(k)
                        stack.append(k)
            v2 = dict(val)
            v2[u] = val[u] ^ FULL
            net.sim(order, v2, tfo_u - {u})
            care = 0
            for o in net.outs:
                care |= val[o] ^ v2[o]
            if care == 0:
                infos = None
                break
            infos.append(prep(val[u], care, D, val))
        if not infos:
            continue
        n_sub = 1 << len(local)
        picks = list(range(1, n_sub)) if n_sub <= 512 else [rng.randrange(1, n_sub) for _ in range(500)]
        rng.shuffle(picks)
        done = False
        for bits in picks:
            sub = [k for j, k in enumerate(local) if bits >> j & 1]
            if len(sub) < 2:
                continue
            a = o = x = None
            for k in sub:
                a = val[k] if a is None else a & val[k]
                o = val[k] if o is None else o | val[k]
                x = val[k] if x is None else x ^ val[k]
            for mode, sv in ((AND, a), (OR, o), (XOR, x), (NAND, a ^ FULL), (NOR, o ^ FULL), (XNOR, x ^ FULL)):
                if sv == val[s]:
                    continue
                res = []
                for info in infos:
                    r = with_new(info, sv, val, rng)
                    if r is None:
                        break
                    res.append(r)
                if len(res) < len(infos):
                    continue
                trial = net.copy()
                nid = trial.next
                trial.next += 1
                trial.g[nid] = (mode, sub)
                for u, (m2, ins2, used) in zip(cons, res):
                    trial.g[u] = (m2, ins2 + ([nid] if used else []))
                trial.prune()
                bypass_buffers(trial)
                if trial.size() <= net.size() - need and trial.ok():
                    if trial.size() < net.size():
                        got = True
                    net.g, net.next, net.outs = trial.g, trial.next, trial.outs
                    done = True
                    break
            if done:
                break
    return got


def load(path):
    net = netlist.load(path)
    return Net({N + i: (m, list(ins)) for i, (m, ins) in enumerate(net["gates"])}, [net["outputs"][b] for b in OUT_BITS])


def save(net, path):
    order = net.order()
    new = {s: s for s in range(N)}
    gates = []
    for s in order:
        new[s] = N + len(gates)
        gates.append([net.g[s][0], [new[i] for i in net.g[s][1]]])
    outputs = [None] * N_OUT
    for b, o in zip(OUT_BITS, net.outs):
        outputs[b] = new[o]
    netlist.save(path, {"inputs": N, "gates": gates, "outputs": outputs})


if __name__ == "__main__":
    family, size, start, out = sys.argv[1:5]
    seed = int(sys.argv[5]) if len(sys.argv) > 5 else 0
    seconds = float(sys.argv[6]) if len(sys.argv) > 6 else 60
    max_new = int(sys.argv[7]) if len(sys.argv) > 7 else 3
    setup(family, size)
    rng = random.Random(seed)
    net = load(start)
    net.prune()
    bypass_buffers(net)
    assert net.ok(), "the start netlist is wrong"
    best, best_net = net.size(), net.copy()
    save(net, out)
    t0 = time.time()
    stale = 0
    while time.time() - t0 < seconds:
        improve(net, rng, max_new=max_new, neutral=0.0 if stale == 0 else 0.6,
                uphill=0.04 if stale > 5 else 0.0, deadline=t0 + seconds)
        reencode(net, rng, neutral=0.0 if stale == 0 else 0.3, deadline=t0 + seconds)
        assert net.ok()
        if net.size() < best:
            best, best_net = net.size(), net.copy()
            save(net, out)
            stale = 0
            print(f"seed {seed}: {best} gates plus {N} inputs ({time.time() - t0:.0f}s)", flush=True)
        else:
            stale += 1
        if stale > 60 or net.size() > best + 3:
            net, stale = best_net.copy(), 1
    print(f"best {best}", flush=True)
