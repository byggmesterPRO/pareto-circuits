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
def small(n):
    """Two bits at a time in 7 gates, the carry walking from pair to pair. About 2 ticks per bit."""
    net, outs, carry, i = Net(2 * n), [], None, 0
    while i < n:
        low = [i, n + i] + ([carry] if carry is not None else [])
        if i + 1 == n:      # an odd bit at the top: a full adder, or a half adder if there is no carry yet
            s = net.gate(XOR, *low)
            carry = net.gate(AND, *low) if carry is None else net.gate(XOR, s, net.gate(AND, *low), net.gate(OR, *low))
            outs.append(s)
            break
        a1, b1 = i + 1, n + i + 1
        any_ = net.gate(OR, *low)
        # "a, b and the carry are all on": never so in the first pair, which has no carry in
        all_ = [net.gate(AND, *low)] if carry is not None else []
        high = net.gate(XOR, *low, a1, b1, any_, *all_)
        g8 = net.gate(OR, a1, b1, high)
        g9 = net.gate(NAND, a1, b1, any_, high)
        out = net.gate(XNOR, *low, a1, b1, any_, *all_, g8, g9)
        outs += [net.gate(XNOR, *low, high, g8, g9, out), high]
        carry, i = out, i + 2
    return net.done(outs + [carry])


# ---- block lookahead ------------------------------------------------------------------------------------
# The carry into bit i is the XOR of mutually exclusive pieces, one per block of a partition of [0, i):
# "block B generates a carry and every bit between B and i passes it on" = AND(p above B, G_B).
#  - a single bit j:        G = a_j AND b_j, so the piece is AND(a_j, b_j, p_{j+1..i-1})        (depth 2)
#  - a small 'cnf' block:   G_B is an AND of OR gates over raw inputs (its prime implicates), shared by every
#                           output above; the piece is AND(clauses, p above)                     (depth 2)
#  - a 'big' block [x, y]:  G_B = AND of depth-2 factors F0 = OR(g_x..g_y) and, for x < m <= y,
#                           F_m = OR(g_{m+1..y}, a_m, b_m) ("bit m is not a kill or something above it
#                           generates"); the factors are shared by every output above y         (depth 3)
# Sum bit i = XOR(p_i, pieces). Depth 3 with singles and cnf blocks, depth 4 with big blocks.

_CLAUSES = {}


def clauses_of(n, x, y):
    if (n, x, y) not in _CLAUSES:
        _CLAUSES[(n, x, y)] = _clauses_of(n, x, y)
    return _CLAUSES[(n, x, y)]


def _clauses_of(n, x, y):
    def rec(top):
        a, b = top, n + top
        if top == x:
            return [(a,), (b,)]
        sub = rec(top - 1)
        return [(a, b)] + [(a,) + c for c in sub] + [(b,) + c for c in sub]
    return [tuple(sorted(c)) for c in rec(y)]


def partition(n, blocks):
    ends = {}
    for (x, y) in blocks:
        ends.setdefault(y + 1, []).append(x)
    best, choice = [0] * (n + 1), [0] * (n + 1)
    for i in range(1, n + 1):
        best[i], choice[i] = best[i - 1] + 1, i - 1
        for x in sorted(ends.get(i, [])):
            if best[x] + 1 < best[i]:
                best[i], choice[i] = best[x] + 1, x
    segs = [[]]
    for i in range(1, n + 1):
        s, e = [], i
        while e > 0:
            s.append((choice[e], e - 1))
            e = choice[e]
        segs.append(s)
    return segs


def choices(n, blocks):
    ends = {}
    for (x, y) in blocks:
        ends.setdefault(y + 1, []).append(x)
    best, choice = [0] * (n + 1), [0] * (n + 1)
    for i in range(1, n + 1):
        best[i], choice[i] = best[i - 1] + 1, i - 1
        for x in sorted(ends.get(i, [])):
            if best[x] + 1 < best[i]:
                best[i], choice[i] = best[x] + 1, x
    return choice


def plan_cost(n, blocks, bot=0):
    """Gate count of build_blocks(n, blocks, bot), without building it."""
    choice = choices(n, blocks)
    shared, pieces, used, need = set(), 0, set(), {0}
    if bot >= 2:
        pieces += (1 << bot) - 2        # the implicants of c_bot other than g_{bot-1}
        shared.add(('g', bot - 1))
    for i in range(1, n + 1):
        if i == bot and bot >= 2:
            continue
        e = i
        while e > 0:
            x, y = choice[e], e - 1
            if x == y and y == i - 1:
                shared.add(('g', x))
            else:
                pieces += 1
                if x != y:
                    used.add((x, y))
            e = x
        e = i
        while choice[e] > 0:
            e = choice[e]
        need.update(range(e, i))     # the bottom segment is [0, e-1]: p_e .. p_{i-1} are used
    for (x, y) in used:
        if blocks[(x, y)] == 'cnf':
            shared.update(('c', c) for c in clauses_of(n, x, y) if len(c) > 1)
        else:
            if x == 0 and bot >= 2 and y >= bot - 1:
                shared.add(('Fb', y))
                lo = bot
            else:
                shared.add(('F0', x, y))
                lo = x + 1
            for j in range(x, y + 1):
                shared.add(('g', j))
            for m in range(lo, y + 1):
                shared.add(('c', (y, n + y)) if m == y else ('F', m, y))
    last = 0
    e = n
    while e > 0:
        last += 1
        e = choice[e]
    outs = (n - 1) + (1 if last > 1 or bot == n else 0)
    return len(need) + len(shared) + pieces + outs


def cnf_family(n, bot=0, whole=False):
    """Consecutive small cnf blocks from the bottom: c4 blocks of 4 bits, then c3 of 3, then c2 of 2, then
    single bits up to the top. The counts are chosen by trying them all."""
    base = {(0, n - 1): 'big'} if whole else {}
    best = (plan_cost(n, base, bot), dict(base))
    for c4 in range(0, (n // 4 if n <= 24 else n // 8) + 1):
        for c3 in range(0, (n - 4 * c4) // 3 + 1):
            for c2 in range(0, (n - 4 * c4 - 3 * c3) // 2 + 1):
                bl, x = dict(base), 0
                for L, c in ((4, c4), (3, c3), (2, c2)):
                    for _ in range(c):
                        bl[(x, x + L - 1)] = 'cnf'
                        x += L
                v = plan_cost(n, bl, bot)
                if v < best[0]:
                    best = (v, bl)
    return best


def levels_plan(n, sizes, cumulative):
    blocks = {}

    def rec(s, e, lv):
        if lv >= len(sizes):
            return
        K, prev, b = sizes[lv], s, s + sizes[lv]
        while b <= e:
            if b - 1 > s:
                blocks[(s if cumulative else prev, b - 1)] = 'big'
            rec(prev, min(b, e), lv + 1)
            prev, b = b, b + K
        rec(prev, e, lv + 1)
    rec(0, n, 0)
    return blocks


def big_family(n, bot=0, top=4):
    """Nested levels of big blocks (sizes K1 > K2 > K3), cumulative from the segment start or not; the best
    two-level plans are then tried with a third level."""
    ks = list(range(2, n + 1)) if n <= 24 else sorted(set(list(range(2, 13)) + list(range(13, n + 1, max(1, n // 24)))))
    scored = []
    for K1 in ks:
        for K2 in [None] + [k for k in ks if k < K1]:
            for cum in (True, False):
                sizes = [k for k in (K1, K2) if k]
                bl = levels_plan(n, sizes, cum)
                scored.append((plan_cost(n, bl, bot), sizes, cum, bl))
    scored.sort(key=lambda r: r[0])
    out = [(plan_cost(n, {}, bot), {})] + [(c, bl) for c, sizes, cum, bl in scored[:top]]
    for c, sizes, cum, bl in scored[:6]:
        if len(sizes) < 2:
            continue
        for K3 in [k for k in ks if k < sizes[1]]:
            bl3 = levels_plan(n, sizes + [K3], cum)
            out.append((plan_cost(n, bl3, bot), bl3))
    out.sort(key=lambda r: r[0])
    return [bl for c, bl in out[:top]]


class CNet(Net):
    def __init__(self, inputs):
        Net.__init__(self, inputs)
        self.cache = {}

    def gate(self, mode, *signals):
        key = (mode, tuple(sorted(signals)))
        if key not in self.cache:
            self.cache[key] = Net.gate(self, mode, *signals)
        return self.cache[key]


def bottom_terms(n, k):
    """The carry into bit k as its prime implicants: g_j and, for each bit above j, a or b. Their OR is c_k,
    and since the implicants come in the exclusive-or pattern of the lookahead sum, their XOR is c_k too."""
    terms = []
    for j in range(k):
        for pick in range(1 << (k - 1 - j)):
            lits = [j, n + j] + [m + (n if pick >> (m - j - 1) & 1 else 0) for m in range(j + 1, k)]
            terms.append(lits)
    return terms


def build_blocks(n, blocks, bot=0):
    """bot = k >= 2: bits 0..k-1 are one 'super bit' whose generate is c_k written as its implicants (depth 1).
    Big blocks [0, y] then need one factor for the whole super bit, and s_k is the XOR of the implicants."""
    net = CNet(2 * n)
    segs = partition(n, blocks)
    p = [None] * n

    def P(j):
        if p[j] is None:
            p[j] = net.gate(XOR, j, n + j)
        return p[j]

    gg = lambda j: net.gate(AND, j, n + j)
    D = [net.gate(AND, *t) for t in bottom_terms(n, bot)] if bot >= 2 else []

    def piece(x, y, i):
        above = [P(j) for j in range(y + 1, i)]
        if x == y:
            return net.gate(AND, x, n + x, *above)
        if blocks[(x, y)] == 'cnf':
            cl = [c[0] if len(c) == 1 else net.gate(OR, *c) for c in clauses_of(n, x, y)]
            return net.gate(AND, *cl, *above)
        if x == 0 and bot >= 2 and y >= bot - 1:
            fs = [net.gate(OR, *[gg(j) for j in range(bot, y + 1)], *D)]
            lo = bot
        else:
            fs = [net.gate(OR, *[gg(j) for j in range(x, y + 1)])]
            lo = x + 1
        fs += [net.gate(OR, *[gg(j) for j in range(m + 1, y + 1)], m, n + m) for m in range(lo, y + 1)]
        return net.gate(AND, *fs, *above) if len(fs) + len(above) > 1 else fs[0]

    terms = [None]
    for i in range(1, n + 1):
        if i == bot and bot >= 2:
            terms.append(D)
        else:
            terms.append([piece(x, y, i) for x, y in segs[i]])
    outs = [P(0)]
    for i in range(1, n + 1):
        if i < n:
            outs.append(net.gate(XOR, *([p[i]] if p[i] is not None else [i, n + i]), *terms[i]))
        else:
            outs.append(terms[i][0] if len(terms[i]) == 1 else net.gate(XOR, *terms[i]))
    return fix_fanout(net.done(outs))


def fix_fanout(nl, limit=255):
    """Duplicate any gate that feeds more than `limit` gates."""
    while True:
        N, gates = nl["inputs"], nl["gates"]
        uses = {}
        for gi, (m, ins) in enumerate(gates):
            for s in ins:
                uses.setdefault(s, []).append(gi)
        bad = [s for s, u in uses.items() if len(u) > limit and s >= N]
        if not bad:
            return nl
        s = bad[0]
        g = s - N
        copies = (len(uses[s]) - 1) // limit
        # new numbering: insert copies right after gate g
        def ren(x):
            return x if x <= s else x + copies
        newg = []
        for gi, (m, ins) in enumerate(gates):
            newg.append([m, [ren(x) for x in ins]])
            if gi == g:
                for c in range(copies):
                    newg.append([m, [ren(x) for x in ins]])
        for k, gi in enumerate(uses[s]):
            c = k // limit
            if c:
                pos = gi + (copies if gi > g else 0)
                newg[pos][1] = [s + c if x == s else x for x in newg[pos][1]]
        outs = [ren(x) for x in nl["outputs"]]
        nl = {"inputs": N, "gates": newg, "outputs": outs}


def depth(nl):
    d = [0] * nl["inputs"]
    for m, ins in nl["gates"]:
        d.append(1 + max(d[x] for x in ins))
    return max(d[x] for x in nl["outputs"])


def fast(n, max_ticks):
    """Search plans (with and without the bottom super bit), build the most promising and keep the smallest
    that fits the limit. At 3 ticks the carry out may still be one big block (a single AND of factors)."""
    bots = [b for b in (0, 2, 3) if b <= n - 1] if n <= 32 else [2]
    cands = []
    for bot in bots:
        if max_ticks >= 4:
            cands += [(bl, bot) for bl in big_family(n, bot)]
        else:
            for whole in ((False, True) if n <= 24 else (False,)):
                cands.append((cnf_family(n, bot, whole)[1], bot))
    best = None
    for bl, bot in cands:
        nl = build_blocks(n, bl, bot)
        if depth(nl) <= max_ticks and (best is None or len(nl["gates"]) < len(best["gates"])):
            best = nl
    return best


def build(n, max_ticks):
    """The smallest adder this code knows that is at most max_ticks deep (None: no limit)."""
    if max_ticks is None or max_ticks >= 2 * n + 1:
        return small(n)
    return fast(n, max_ticks)
# EVOLVE-BLOCK-END


def run_experiment(n, max_ticks):
    """Called by the evaluator, once per case."""
    return {"n": n, "max_ticks": max_ticks, "netlist": build(n, max_ticks)}
