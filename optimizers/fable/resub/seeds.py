"""Seed designs for 12-bit binary -> BCD built from long-division chains with different divisors.

A chain divides a number (bits, most significant first) by a constant M one bit at a time: keep a remainder r,
shift the next bit in, subtract M when the result is M or more; the "did subtract" bits are the quotient.
Dividing x >> k by M / 2**k with M = 10, 100 or 1000 peels decimal digits off in different orders:

  a5     x>>1 by 5, then that quotient >>1 by 5, then again          (the classic double dabble, 18 cells)
  a25    x>>2 by 25 -> x div 100 and x mod 100, each then by 5       (13 cells)
  a125   x>>3 by 125 -> thousands; rest >>2 by 25 -> hundreds; rest >>1 by 5 -> tens, ones   (11 cells)
  a5_25  x>>1 by 5 -> ones; quotient >>2 by 25 -> thousands; rest >>1 by 5 -> hundreds, tens

These are plain ripple logic, meant as starting points for resub.py. 12bit only.

    python seeds.py <folder>        writes <folder>/a5.json, a25.json, a125.json, a5_25.json
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "tools" / "bygg-logic-sim"))
import netlist
import verify


class B:
    """Gate list builder with constant folding. Signals: int ids, False = constant 0, True = constant 1."""

    def __init__(self):
        self.gates = []
        self.memo = {}

    def gate(self, mode, ins):
        key = (mode, tuple(sorted(ins)))
        if key not in self.memo:
            self.gates.append((mode, list(key[1])))
            self.memo[key] = 11 + len(self.gates)
        return self.memo[key]

    def and_(self, *xs):
        if any(x is False for x in xs):
            return False
        xs = sorted(set(x for x in xs if x is not True))
        return True if not xs else xs[0] if len(xs) == 1 else self.gate(0, xs)

    def or_(self, *xs):
        if any(x is True for x in xs):
            return True
        xs = sorted(set(x for x in xs if x is not False))
        return False if not xs else xs[0] if len(xs) == 1 else self.gate(1, xs)

    def xor_(self, *xs):
        inv = sum(1 for x in xs if x is True) % 2
        real = [x for x in xs if x is not True and x is not False]
        odd = sorted(x for x in set(real) if real.count(x) % 2)
        if not odd:
            return bool(inv)
        if len(odd) == 1 and not inv:
            return odd[0]
        return self.gate(5 if inv else 2, odd)

    def reduce(self, t, M, tmax):
        """t: bits LSB first, value <= tmax < 2*M. Returns (q, r bits LSB first, r max)."""
        if tmax < M:
            return False, t, tmax
        n = len(t)
        D = M - 1
        terms = []
        for i in range(n):          # t > D: some bit where D has 0 and t has 1, with t covering D's 1s above it
            if not D >> i & 1:
                terms.append(self.and_(t[i], *[t[j] for j in range(i + 1, n) if D >> j & 1]))
        q = self.or_(*terms)
        K = (1 << n) - M            # subtracting M is adding K and dropping the carry out
        r, c = [], False
        for i in range(n):
            if K >> i & 1:
                r.append(self.xor_(t[i], q, c))
                c = self.or_(self.and_(t[i], q), c)
            else:
                r.append(self.xor_(t[i], c))
                c = self.and_(t[i], c)
        nb = (M - 1).bit_length()
        return q, r[:nb], M - 1

    def divide(self, bits, M, vmax):
        """bits: most significant first, value <= vmax. Returns (quotient bits MSB first, remainder bits LSB first)."""
        q, r, rmax = [], [], 0
        for i, b in enumerate(bits):
            left = len(bits) - 1 - i
            t = [b] + r
            tmax = min(2 * rmax + 1, vmax >> left)
            qi, r, rmax = self.reduce(t, M, tmax)
            while len(r) > 1 and r[-1] is False:
                r = r[:-1]
            if q or qi is not False:
                q.append(qi)
        return q, r


def pad(bits, n):
    return bits + [False] * (n - len(bits))


def build(name):
    b = B()
    x = list(range(12))
    hi = lambda k: [x[i] for i in range(11, k - 1, -1)]         # x11 .. xk
    out = [False] * 16
    out[0] = x[0]
    if name == "a5":
        q0, r0 = b.divide(hi(1), 5, 2047)                        # x div 10
        out[1:4] = pad(r0, 3)
        out[4] = q0[-1]
        q1, r1 = b.divide(q0[:-1], 5, 204)                       # x div 100
        out[5:8] = pad(r1, 3)
        out[8] = q1[-1]
        q2, r2 = b.divide(q1[:-1], 5, 20)                        # x div 1000
        out[9:12] = pad(r2, 3)
        th = q2
    elif name == "a25":
        h, r25 = b.divide(hi(2), 25, 1023)                       # h = x div 100 (<= 40)
        m = list(reversed(pad(r25, 5))) + [x[1]]                 # (x mod 100) >> 1, <= 49
        t, r5 = b.divide(m, 5, 49)
        out[1:4] = pad(r5, 3)
        out[4:8] = pad(list(reversed(t)), 4)
        out[8] = h[-1]
        th, r = b.divide(h[:-1], 5, 20)
        out[9:12] = pad(r, 3)
    elif name == "a125":
        th, r125 = b.divide(hi(3), 125, 511)                     # thousands
        m = list(reversed(pad(r125, 7))) + [x[2]]                # (x mod 1000) >> 2, <= 249
        h, r25 = b.divide(m, 25, 249)
        out[8:12] = pad(list(reversed(h)), 4)
        m2 = list(reversed(pad(r25, 5))) + [x[1]]
        t, r5 = b.divide(m2, 5, 49)
        out[1:4] = pad(r5, 3)
        out[4:8] = pad(list(reversed(t)), 4)
    elif name == "a5_25":
        q0, r0 = b.divide(hi(1), 5, 2047)                        # q0 = x div 10 (<= 409)
        out[1:4] = pad(r0, 3)
        out[4] = q0[-1]
        th, r25 = b.divide(q0[:-2], 25, 102)                     # (x div 10) >> 2 by 25 -> thousands
        m = list(reversed(pad(r25, 5))) + [q0[-2]]               # ((x div 10) mod 100) >> 1
        h, r5 = b.divide(m, 5, 49)
        out[5:8] = pad(r5, 3)
        out[8:12] = pad(list(reversed(h)), 4)
    out[12:12 + len(th)] = list(reversed(th))
    return {"inputs": 12, "gates": [[m, i] for m, i in b.gates], "outputs": [None if s is False else s for s in out]}


if __name__ == "__main__":
    folder = Path(sys.argv[1])
    for name in ("a5", "a25", "a125", "a5_25"):
        net = build(name)
        res = verify.check(net, "double-dabble", "12bit")
        assert not res["problems"] and not res["wrong"], (name, res)
        netlist.save(folder / f"{name}.json", net)
        print(name, res["gates"], "gates,", res["ticks"], "ticks")
