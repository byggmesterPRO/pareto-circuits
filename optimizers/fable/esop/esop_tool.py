"""Glue for the esop minimiser: a 4-tick lookup-table circuit for any family up to 12 input bits.

The circuit has four layers of gates: the input gates; XOR/XNOR re-codings of the inputs and inverters; AND/NOR
terms; and one XOR per output bit. `start` picks the re-coding and writes a first set of terms, esop (the Rust
program) shrinks the set of terms, `build` turns the result into a netlist.

    python esop_tool.py start <family> <size> <seed> <steps> <name>     writes runs/<name>.esop and .rows.json
    esop runs/<name>.esop runs/<name>.res <threads> <seconds>
    python esop_tool.py build <family> <size> runs/<name>.res runs/<name>.rows.json <out netlist.json>

Different seeds give different re-codings, and the end result depends a lot on the re-coding: try several.
steps 0 means no re-coding. Needs numpy. The start step follows xor_rom in sm_blueprint_lib.
"""
import json
import random
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "tools" / "bygg-logic-sim"))
import netlist
import verify
from families import FAMILIES

RUNS = Path(__file__).resolve().parent / "runs"


def pc(x):
    return bin(x).count("1")


def transform(table, n, rows):
    """Table re-indexed by y, where y_i = parity(x & rows[i])."""
    x = np.arange(1 << n)
    y = np.zeros_like(x)
    for i, r in enumerate(rows):
        par = x & r
        for s in (1, 2, 4, 8, 16):
            par ^= par >> s
        y |= (par & 1) << i
    out = np.empty(1 << n, dtype=np.int64)
    out[y] = np.asarray(table, dtype=np.int64)
    return out


def anf(bits):
    """Moebius transform: truth table of one output bit -> Reed-Muller coefficients."""
    a = bits.astype(np.uint8).copy()
    for i in range(a.size.bit_length() - 1):
        a = a.reshape(-1, 2 << i)
        a[:, 1 << i:] ^= a[:, :1 << i]
    return a.reshape(-1)


def reed_muller(table, n, m, pol):
    """Terms of the fixed polarity form: bit i of pol set = variable i inverted in every term."""
    vals = np.asarray(table, dtype=np.int64)[np.arange(1 << n) ^ pol]
    cubes = {}
    for b in range(m):
        for t in np.nonzero(anf((vals >> b) & 1))[0].tolist():
            key = (t & ~pol, t & pol)
            om = cubes.pop(key, 0) ^ (1 << b)
            if om:
                cubes[key] = om
    return cubes


def cost(cubes):
    """Term gates plus inverters, the number esop minimises."""
    terms = inv = 0
    for p, q in cubes:
        if pc(p | q) >= 2:
            terms += 1
            if p and q:
                inv |= q
    return terms + pc(inv)


def search_rows(table, n, m, steps, rng):
    """Hill-climb over invertible XOR re-codings of the inputs (row operations y_i ^= y_j)."""
    full = (1 << n) - 1

    def score(rows):
        tab = transform(table, n, rows)
        return min(cost(reed_muller(tab, n, m, p)) for p in (0, full)) + sum(pc(r) >= 2 for r in rows)

    rows = tuple(1 << i for i in range(n))
    cur = score(rows)
    best, best_cost = rows, cur
    for _ in range(steps):
        i, j = rng.sample(range(n), 2)
        trial = list(rows)
        trial[i] ^= rows[j]
        trial = tuple(trial)
        c = score(trial)
        if c <= cur or rng.random() < 0.02:
            rows, cur = trial, c
            if c < best_cost:
                best, best_cost = rows, c
    return best


def cmd_start(family, size, seed, steps, name):
    n, m, f = FAMILIES[family](size)
    if n > 12:
        sys.exit("a lookup table is only practical up to 12 input bits")
    table = [f(x) for x in range(1 << n)]
    rng = random.Random(seed)
    rows = search_rows(table, n, m, steps, rng)
    tab = transform(table, n, rows)
    full = (1 << n) - 1
    pols = [0, full] + [rng.randrange(1 << n) for _ in range(64)]
    cubes = reed_muller(tab, n, m, min(pols, key=lambda p: cost(reed_muller(tab, n, m, p))))
    base = m + sum(pc(r) >= 2 for r in rows)
    RUNS.mkdir(exist_ok=True)
    lines = [f"{n} {m} {base}"] + [f"{p} {q} {om}" for (p, q), om in cubes.items()]
    (RUNS / f"{name}.esop").write_text("\n".join(lines) + "\n")
    (RUNS / f"{name}.rows.json").write_text(json.dumps(rows))
    print(f"{name}: {base - m} inputs re-coded, start cost {cost(cubes) + base}")


def cmd_build(family, size, result, rows_file, out):
    n, m, f = FAMILIES[family](size)
    table = [f(x) for x in range(1 << n)]
    rows = json.loads(Path(rows_file).read_text())
    cubes = {}
    for line in Path(result).read_text().split("\n")[1:]:
        t = line.split()
        if len(t) == 3:
            cubes[(int(t[0]), int(t[1]))] = int(t[2])
    tab = transform(table, n, rows)
    y = np.arange(1 << n)
    got = np.zeros(1 << n, dtype=np.int64)
    for (p, q), om in cubes.items():
        got ^= np.where(((y & p) == p) & ((y & q) == 0), om, 0)
    if not (got == tab).all():
        sys.exit("the terms do not add up to the table")

    gates = []

    def gate(mode, ins):
        gates.append([mode, sorted(ins)])
        return n + len(gates) - 1

    bits = lambda mask: [i for i in range(n) if mask >> i & 1]
    # layer 2: re-coded inputs, and the inverted ones that some term needs next to plain ones
    yv = [bits(r)[0] if pc(r) == 1 else gate(netlist.XOR, bits(r)) for r in rows]
    inv_mask = 0
    for p, q in cubes:
        if p and q:
            inv_mask |= q
    inv = {i: gate(netlist.NOR if pc(rows[i]) == 1 else netlist.XNOR, bits(rows[i])) for i in bits(inv_mask)}
    # layer 3: one gate per term with two or more literals
    term = {}
    for p, q in sorted(c for c in cubes if pc(c[0] | c[1]) >= 2):
        if p and q:
            term[p, q] = gate(netlist.AND, [yv[i] for i in bits(p)] + [inv[i] for i in bits(q)])
        elif p:
            term[p, q] = gate(netlist.AND, [yv[i] for i in bits(p)])
        else:
            term[p, q] = gate(netlist.NOR, [yv[i] for i in bits(q)])
    # layer 4: one XOR per output bit
    outputs = []
    for b in range(m):
        feeds, invert = [], False
        for (p, q), om in cubes.items():
            if not om >> b & 1:
                continue
            if (p, q) in term:
                feeds.append(term[p, q])
            elif p or q:
                feeds.append(yv[(p | q).bit_length() - 1])
                invert ^= bool(q)       # a lone inverted variable: the plain one, output flipped
            else:
                invert = not invert     # the constant 1
        feeds = sorted(s for s in set(feeds) if feeds.count(s) % 2)     # a signal fed twice cancels out
        if not feeds:
            if invert:
                sys.exit(f"output bit {b} is always 1, which this builder does not handle")
            outputs.append(None)
        elif len(feeds) == 1 and not invert:
            outputs.append(feeds[0])
        else:
            outputs.append(gate(netlist.XNOR if invert else netlist.XOR, feeds))
    net = {"inputs": n, "gates": gates, "outputs": outputs}
    res = verify.check(net, family, size)
    if res["problems"] or res["wrong"]:
        sys.exit(f"built circuit is wrong: {res}")
    netlist.to_blueprint(net)       # raises if a part would need more than 255 connections
    netlist.save(out, net)
    print(f"{out}: {res['gates']} gates, {res['ticks']} ticks, {res['verified']}")


if __name__ == "__main__":
    if sys.argv[1] == "start":
        cmd_start(sys.argv[2], sys.argv[3], int(sys.argv[4]), int(sys.argv[5]), sys.argv[6])
    else:
        cmd_build(*sys.argv[2:7])
