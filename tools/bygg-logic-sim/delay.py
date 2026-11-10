"""Worst output delay actually seen when the inputs change, for a circuit without memory.

    python tools/bygg-logic-sim/delay.py <build slot or netlist.json> [transitions]

The official `ticks` (verify.py) is the longest path through the circuit, which no input change can exceed.
Many circuits never use their longest path, so the delay that really happens can be lower. This simulates
random jumps from one input value to another, tick by tick, and reports the longest time until the outputs
stop changing. It counts like `ticks`: the input gate is tick 1. A lower number here is a measurement over
the jumps tried, not a guarantee.

All jumps run at once: every signal is one big integer with a bit per jump.
"""
import json
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import netlist


def worst_seen(net, transitions=200_000, seed=1):
    n = net["inputs"]
    rng = random.Random(seed)
    top = (1 << n) - 1
    a = [rng.randrange(top + 1) for _ in range(transitions)]
    b = [rng.randrange(top + 1) for _ in range(transitions)]
    mask = (1 << transitions) - 1
    pack = lambda vals, i: sum(((v >> i) & 1) << k for k, v in enumerate(vals))
    state = netlist.evaluate(net, [pack(a, i) for i in range(n)], mask)      # settled on a
    new_inputs = [pack(b, i) for i in range(n)]
    state[:n] = new_inputs                                                    # tick 1: the input gates show b
    outs = [o for o in net["outputs"] if o is not None]
    worst = 1
    for t in range(2, max(netlist.depths(net)) + 2):
        new = list(new_inputs)
        for mode, ins in net["gates"]:
            new.append(netlist.gate_value(mode, [state[i] for i in ins], mask))
        if any(new[o] != state[o] for o in outs):
            worst = t
        state = new
    return worst


if __name__ == "__main__":
    target = Path(sys.argv[1])
    if target.is_dir():
        meta = json.loads((target / "meta.json").read_text(encoding="utf-8"))
        bp = json.loads((target / "blueprint" / "blueprint.json").read_text(encoding="utf-8"))
        net = netlist.from_blueprint(bp, meta["pins"])
    else:
        net = netlist.load(target)
    k = int(sys.argv[2]) if len(sys.argv) > 2 else 200_000
    gates, ticks = netlist.counts(net)
    print(json.dumps({"gates": gates, "ticks": ticks, "worst_delay_seen": worst_seen(net, k), "transitions": k}))
