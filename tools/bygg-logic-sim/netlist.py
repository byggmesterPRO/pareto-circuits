"""Netlists and Scrap Mechanic blueprints, both ways. No dependencies.

A netlist is JSON:

    {"inputs": 12, "gates": [[mode, [signal, ...]], ...], "outputs": [signal or null, ...]}

Signal i < inputs is input bit i, least significant first. Signal inputs + g is gate g. Gates are listed in
feed-forward order: a gate only reads signals before it. mode: 0 and, 1 or, 2 xor, 3 nand, 4 nor, 5 xnor.
An output is the signal that carries that output bit, least significant first. null is a bit that is always 0
and has no gate. An output may be an input signal (the bit passes straight through).

In a blueprint every input bit is one OR gate with nothing wired into it: the player wires into those.
"""
import json
from pathlib import Path

LOGIC_GATE = "9f0f56e8-2c31-4d83-996c-d00a9b296c3f"
MODES = ["and", "or", "xor", "nand", "nor", "xnor"]
AND, OR, XOR, NAND, NOR, XNOR = range(6)
INPUT_COLOR = "E8C547"
OUTPUT_COLOR = "19E753"
GATE_COLOR = "222222"
MAX_CONNECTIONS = 255       # the game's limit on wires into or out of one part
ROW = 16


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def save(path, net):
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_text(json.dumps(net, separators=(",", ":")) + "\n", encoding="utf-8")


def gate_value(mode, values, mask):
    """Value of one gate as a bit vector, one bit per test input. A gate with no inputs is off."""
    if not values:
        return 0
    v = values[0]
    if mode in (AND, NAND):
        for u in values[1:]:
            v &= u
    elif mode in (OR, NOR):
        for u in values[1:]:
            v |= u
    else:
        for u in values[1:]:
            v ^= u
    return v ^ mask if mode >= 3 else v


def evaluate(net, input_vectors, mask):
    """input_vectors[i] has bit r set when input i is on in test r. Returns one bit vector per signal."""
    val = list(input_vectors)
    for mode, ins in net["gates"]:
        val.append(gate_value(mode, [val[i] for i in ins], mask))
    return val


def depths(net):
    """Gates on the longest path from an input up to and including each signal (an input gate counts as 1)."""
    d = [1] * net["inputs"]
    for mode, ins in net["gates"]:
        d.append(1 + max((d[i] for i in ins), default=0))
    return d


def counts(net):
    """(gates, ticks) the way the repo README counts them. Gates include the input gates. Ticks is the
    number of gates on the longest path from an input to an output, both ends included."""
    d = depths(net)
    outs = [o for o in net["outputs"] if o is not None]
    return net["inputs"] + len(net["gates"]), max(d[o] for o in outs)


def to_blueprint(net):
    """Returns (blueprint dict, pins). pins = {"inputs": [controller id], "outputs": [controller id or None]}.
    Layout: inputs on row y=0 at x=bit, output gates on row y=2 at x=bit, the rest from row y=4."""
    n = net["inputs"]
    fan_out = [0] * (n + len(net["gates"]))
    for mode, ins in net["gates"]:
        if len(ins) > MAX_CONNECTIONS:
            raise ValueError(f"a gate has {len(ins)} inputs, the game allows {MAX_CONNECTIONS}")
        if len(set(ins)) != len(ins):
            raise ValueError("a gate reads the same signal twice")
        for i in ins:
            fan_out[i] += 1
    if max(fan_out) > MAX_CONNECTIONS:
        raise ValueError(f"a signal drives {max(fan_out)} gates, the game allows {MAX_CONNECTIONS}")
    out_bit = {}
    for b, o in enumerate(net["outputs"]):
        if o is not None and o >= n:
            out_bit.setdefault(o, b)
    readers = [[] for _ in fan_out]
    for g, (mode, ins) in enumerate(net["gates"]):
        for i in ins:
            readers[i].append(n + g)

    def part(sig, mode, color, pos):
        return {"color": color,
                "controller": {"active": False, "controllers": [{"id": r} for r in readers[sig]] or None,
                               "id": sig, "joints": None, "mode": mode},
                "joints": None, "pos": {"x": pos[0], "y": pos[1], "z": pos[2]},
                "shapeId": LOGIC_GATE, "xaxis": 1, "zaxis": -2}

    childs = [part(i, OR, INPUT_COLOR, (i, 0, 0)) for i in range(n)]
    k = 0
    for g, (mode, ins) in enumerate(net["gates"]):
        sig = n + g
        if sig in out_bit:
            childs.append(part(sig, mode, OUTPUT_COLOR, (out_bit[sig], 2, 0)))
        else:
            childs.append(part(sig, mode, GATE_COLOR, (k % ROW, 4 + k // ROW, 0)))
            k += 1
    pins = {"inputs": list(range(n)), "outputs": list(net["outputs"])}
    return {"bodies": [{"childs": childs}], "joints": None, "version": 4}, pins


def from_blueprint(bp, pins):
    """Netlist of every logic gate in a blueprint, with the given pins. Gates nothing reads are kept, since
    they still count as gates."""
    mode, ins = {}, {}
    for body in bp["bodies"]:
        for p in body["childs"]:
            if p.get("shapeId") != LOGIC_GATE:
                continue
            c = p["controller"]
            mode[c["id"]] = c["mode"]
            ins.setdefault(c["id"], [])
    for body in bp["bodies"]:
        for p in body["childs"]:
            c = p.get("controller") or {}
            if c.get("id") in mode:
                for t in c.get("controllers") or []:
                    if t["id"] in ins:
                        ins[t["id"]].append(c["id"])
    sig = {}
    for i, cid in enumerate(pins["inputs"]):
        if ins[cid]:
            raise ValueError(f"input pin {i} has wires going into it")
        sig[cid] = i
    n = len(sig)
    gates, todo = [], [g for g in mode if g not in sig]
    while todo:
        rest = []
        for g in todo:
            if all(i in sig for i in ins[g]):
                sig[g] = n + len(gates)
                gates.append([mode[g], [sig[i] for i in ins[g]]])
            else:
                rest.append(g)
        if len(rest) == len(todo):
            raise ValueError("the blueprint has a feedback loop")
        todo = rest
    return {"inputs": n, "gates": gates, "outputs": [None if o is None else sig[o] for o in pins["outputs"]]}
