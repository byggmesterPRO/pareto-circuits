"""Do bygg-logic-sim, niknal-logic-sim and logic-sim agree, and how fast is each.

    cargo build --release --manifest-path tools/logic-sim/Cargo.toml
    python tools/logic-sim/compare.py

Part 1: the double dabble builds and generator outputs, checked by all three.
Part 2: random circuits with feedback loops and timers, driven with random inputs. All three simulators must
show the same state for every part on every tick.
"""
import json
import random
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "tools" / "bygg-logic-sim"))
sys.path.insert(0, str(REPO / "tools" / "niknal-logic-sim"))
sys.path.insert(0, str(REPO / "circuits" / "double-dabble" / "generator"))
import netlist
import verify
from sim import Sim as ByggSim
from circuitlib import Circuit, Direction, GateType, IVec3, Simulation
from families import FAMILIES
from generate import generate

EXE = REPO / "tools" / "logic-sim" / "target" / "release" / ("logic-sim.exe" if sys.platform == "win32" else "logic-sim")
TYPES = [GateType.AND, GateType.OR, GateType.XOR, GateType.NAND, GateType.NOR, GateType.XNOR]
TMP = Path(tempfile.mkdtemp())


def logic_sim(*args):
    t0 = time.perf_counter()
    out = subprocess.run([str(EXE), *map(str, args)], capture_output=True, text=True).stdout
    return json.loads(out), time.perf_counter() - t0


def niknal_circuit(net):
    """Our netlist in niknal's model: each input gate is an OR gate fed by a named input."""
    c = Circuit()
    cels = []
    for i in range(net["inputs"]):
        g = c.add_gate(IVec3(i, 0, 0), GateType.OR)
        c.add_input(f"in{i}")
        c.connect_input(f"in{i}", g)
        cels.append(g)
    for k, (mode, ins) in enumerate(net["gates"]):
        g = c.add_gate(IVec3(k, 1, 0), TYPES[mode])
        for i in ins:
            cels[i].connect_to(g)
        cels.append(g)
    for b, o in enumerate(net["outputs"]):
        if o is not None:
            c.add_output(f"out{b}", cels[o])
    return c


def part1():
    print("Part 1: correct circuits, every input")
    print(f"{'circuit':<22}{'gates':>6}{'ticks':>6}   {'bygg':>10}{'niknal':>16}{'logic-sim':>11}")
    targets = []
    for prof in ("fast", "small"):
        slot = REPO / "circuits" / "double-dabble" / "builds" / "12bit" / prof
        meta = json.loads((slot / "meta.json").read_text())
        bp = json.loads((slot / "blueprint" / "blueprint.json").read_text())
        targets.append((f"12bit {prof}", netlist.from_blueprint(bp, meta["pins"]), 12))
    for bits in (4, 8, 12, 16):
        targets.append((f"generator {bits}bit", generate(bits), bits))
    ok = True
    for name, net, bits in targets:
        size = f"{bits}bit"
        f = FAMILIES["double-dabble"](size)[2]
        t0 = time.perf_counter()
        res = verify.check(net, "double-dabble", size)
        t_bygg = time.perf_counter() - t0
        # niknal: settle every input (a sample above 12 bits, it is one input at a time)
        xs = range(1 << bits) if bits <= 12 else random.Random(0).sample(range(1 << bits), 2000)
        sim = Simulation(niknal_circuit(net))
        t0 = time.perf_counter()
        wrong = 0
        for x in xs:
            for i in range(bits):
                sim.set_input(f"in{i}", bool(x >> i & 1))
            for _ in range(res["ticks"]):
                sim.tick()
            got = sum(int(sim.read_output(f"out{b}")) << b for b, o in enumerate(net["outputs"]) if o is not None)
            wrong += got != f(x)
        t_niknal = time.perf_counter() - t0
        path = TMP / f"{name.replace(' ', '_')}.json"
        netlist.save(path, net)
        rust, t_rust = logic_sim("verify", path, "double-dabble", size)
        agree = res["wrong"] == wrong == rust["wrong"] == 0 and (res["gates"], res["ticks"]) == (rust["gates"], rust["ticks"])
        ok &= agree
        note = "" if bits <= 12 else f" ({len(xs)} inputs)"
        print(f"{name:<22}{res['gates']:>6}{res['ticks']:>6}   {t_bygg:>9.3f}s{t_niknal:>9.2f}s{note:<14}{t_rust:>9.3f}s  {'agree' if agree else 'DISAGREE'}")
    return ok


def random_circuit(rng, n_in, n_gates, n_timers):
    """Parts 0..n_in-1 are input gates. Gates read any part, later ones and themselves included."""
    n = n_in + n_gates + n_timers
    kinds = ["in"] * n_in + ["gate"] * n_gates + ["timer"] * n_timers
    order = list(range(n_in, n))
    rng.shuffle(order)
    kinds[n_in:] = [kinds[i] for i in order]
    parts = []
    for e in range(n):
        if kinds[e] == "in":
            parts.append(("gate", 1, []))
        elif kinds[e] == "gate":
            parts.append(("gate", rng.randrange(6), rng.sample(range(n), rng.randint(1, 4))))
        else:
            parts.append(("timer", rng.randrange(7), [rng.randrange(n)]))
    return parts


def part2(circuits=30, ticks=200):
    print(f"\nPart 2: {circuits} random circuits with loops and timers, {ticks} ticks each, every part compared on every tick")
    rng = random.Random(7)
    ok = True
    for k in range(circuits):
        n_in = rng.randint(2, 5)
        parts = random_circuit(rng, n_in, rng.randint(5, 30), rng.randint(0, 4))
        n = len(parts)
        # niknal
        c = Circuit()
        cels = [c.add_gate(IVec3(e, 0, 0), TYPES[p[1]]) if p[0] == "gate" else c.add_timer(IVec3(e, 2, 0), Direction.PosY, p[1])
                for e, p in enumerate(parts)]
        for e, p in enumerate(parts):
            for i in p[2]:
                cels[i].connect_to(cels[e])
        for i in range(n_in):
            c.add_input(f"in{i}")
            c.connect_input(f"in{i}", cels[i])
        nik = Simulation(c)
        # bygg, from a blueprint
        readers = [[] for _ in range(n)]
        for e, p in enumerate(parts):
            for i in p[2]:
                readers[i].append(e)
        childs = []
        for e, p in enumerate(parts):
            ctrl = {"id": e, "controllers": [{"id": r} for r in readers[e]] or None}
            ctrl.update({"mode": p[1]} if p[0] == "gate" else {"seconds": 0, "ticks": p[1]})
            childs.append({"color": "222222", "pos": {"x": e, "y": 0, "z": 0}, "controller": ctrl,
                           "shapeId": netlist.LOGIC_GATE if p[0] == "gate" else "8f7fd0e7-c46e-4944-a414-7ce2437bb30f"})
        bp = {"bodies": [{"childs": childs}], "version": 4}
        bygg = ByggSim(bp)
        script, same = [], True
        for t in range(ticks):
            for i in range(n_in):
                if rng.random() < 0.3:
                    v = rng.random() < 0.5
                    nik.set_input(f"in{i}", v)
                    bygg.force(i, v)
                    script.append(f"set #{i} {int(v)}")
            nik.tick()
            bygg.tick()
            script.append("tick 1")
            state = [nik.values[nik.index[cels[e]]] for e in range(n)]
            same &= state == [bygg.get(e) for e in range(n)]
            script += [f"check #{e} {int(v)}" for e, v in enumerate(state)]
        (TMP / "bp.json").write_text(json.dumps(bp))
        (TMP / "script.txt").write_text("\n".join(script))
        rust, _ = logic_sim("run", TMP / "bp.json", TMP / "script.txt")
        good = same and rust.get("correct") == rust.get("total") == ticks * n
        ok &= good
        if not good:
            print(f"  circuit {k}: bygg == niknal: {same}, logic-sim: {rust.get('correct')} of {rust.get('total')}")
    print("  all three agree on every part and tick" if ok else "  DISAGREEMENT, see above")
    return ok


if __name__ == "__main__":
    a = part1()
    b = part2()
    sys.exit(0 if a and b else 1)
