"""The shared verifier: is a circuit correct, and what are its gates and ticks.

    python tools/bygg-logic-sim/verify.py circuits/double-dabble/builds/12bit/fast       a build slot
    python tools/bygg-logic-sim/verify.py net.json double-dabble 12bit                   a netlist

A slot is checked from its blueprint.json, with the pins listed in meta.json, and its meta.json numbers must
match. Up to 20 input bits every input is tried ("exhaustive"); above that, edge cases and 200 000 random
inputs ("sampled"). Exits 1 if anything is wrong.
"""
import json
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import netlist
from families import FAMILIES

EXHAUSTIVE_BITS = 20
SAMPLES = 200_000


def test_inputs(bits):
    if bits <= EXHAUSTIVE_BITS:
        return "exhaustive", range(1 << bits)
    top = (1 << bits) - 1
    xs = {0, 1, top, top - 1}
    for i in range(bits):
        xs.update({1 << i, (1 << i) - 1, top ^ (1 << i)})
    for d in range(1, len(str(top)) + 1):
        xs.update(v for v in (10 ** d - 1, 10 ** d, 5 * 10 ** (d - 1)) if v <= top)
    rng = random.Random(0)
    xs.update(rng.randrange(top + 1) for _ in range(SAMPLES))
    return "sampled", sorted(xs)


def check(net, family, size):
    """Returns a dict: gates, ticks, verified ("exhaustive"/"sampled"), wrong (inputs with a wrong output)."""
    bits, out_bits, f = FAMILIES[family](size)
    problems = []
    if net["inputs"] != bits:
        problems.append(f"{net['inputs']} inputs, {family} {size} has {bits}")
    if len(net["outputs"]) != out_bits:
        problems.append(f"{len(net['outputs'])} outputs, {family} {size} has {out_bits}")
    if problems:
        return {"problems": problems}
    how, xs = test_inputs(bits)
    xs = list(xs)
    mask = (1 << len(xs)) - 1
    vectors = [sum(((x >> i) & 1) << r for r, x in enumerate(xs)) for i in range(bits)]
    val = netlist.evaluate(net, vectors, mask)
    want = [f(x) for x in xs]
    wrong = 0
    for b, o in enumerate(net["outputs"]):
        target = sum(((w >> b) & 1) << r for r, w in enumerate(want))
        wrong |= (0 if o is None else val[o]) ^ target
    gates, ticks = netlist.counts(net)
    return {"gates": gates, "ticks": ticks, "verified": how, "wrong": bin(wrong).count("1"), "problems": []}


def check_slot(slot):
    slot = Path(slot)
    meta = json.loads((slot / "meta.json").read_text(encoding="utf-8"))
    bp = json.loads((slot / "blueprint" / "blueprint.json").read_text(encoding="utf-8"))
    net = netlist.from_blueprint(bp, meta["pins"])
    res = check(net, meta["family"], meta["size"])
    if res["problems"]:
        return res
    for key in ("gates", "ticks", "verified"):
        if meta.get(key) != res[key]:
            res["problems"].append(f"meta.json says {key} = {meta.get(key)}, the verifier gets {res[key]}")
    if slot.resolve().parts[-2:] != (meta["size"], meta["profile"]):
        res["problems"].append("meta.json size/profile do not match the folder")
    wires_in, wires_out = {}, {}
    for body in bp["bodies"]:
        for p in body["childs"]:
            c = p["controller"]
            wires_out[c["id"]] = len(c.get("controllers") or [])
            for t in c.get("controllers") or []:
                wires_in[t["id"]] = wires_in.get(t["id"], 0) + 1
    worst = max(list(wires_in.values()) + list(wires_out.values()))
    if worst > netlist.MAX_CONNECTIONS:
        res["problems"].append(f"a part has {worst} connections, the game allows {netlist.MAX_CONNECTIONS}")
    return res


if __name__ == "__main__":
    if len(sys.argv) == 2:
        res = check_slot(sys.argv[1])
    else:
        res = check(netlist.load(sys.argv[1]), sys.argv[2], sys.argv[3])
    if res.get("wrong"):
        res["problems"].append(f"{res['wrong']} test inputs give a wrong output")
    print(json.dumps(res))
    sys.exit(1 if res["problems"] else 0)
