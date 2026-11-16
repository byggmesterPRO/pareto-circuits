"""Score a candidate circuit, or a candidate generator, for a search such as ShinkaEvolve.

A candidate generator is one Python function per family:

    def build(size: str, max_ticks: int | None, balanced: bool) -> dict:
        ...return a netlist...

    netlist = {"inputs": n, "gates": [[mode, [signal, ...]], ...], "outputs": [signal or None, ...]}
    optional: "timers": [[signal it reads, setting], ...], numbered after the gates

Signal i < n is input bit i, signal n + g is gate g. Modes: 0 and, 1 or, 2 xor, 3 nand, 4 nor, 5 xnor.
What each family's inputs and outputs mean is in circuits/<family>/README.md and tools/logic-sim/src/families.rs.

Scoring is correctness first, size second:

- A candidate that is wrong, breaks a game limit (255 wires per part), is too slow for `max_ticks`, or is not
  balanced when it has to be, fails. Its score is PENALTY plus how wrong it is (0 to 1), so that a search can
  still tell a nearly right candidate from a hopeless one.
- A candidate that passes scores its size: gates, plus timers when balanced. Lower is better.

From the command line:

    python tools/evolve/evaluate.py adder 8bit candidate.json [--max-ticks 6] [--balanced]
    python tools/evolve/evaluate.py adder generator.py --sizes 4bit,8bit,16bit [--max-ticks 6] [--balanced]

From Python:

    from evaluate import Scorer
    with Scorer() as s:
        s.circuit("adder", "8bit", netlist, max_ticks=6)             # one circuit
        s.generator("adder", build, ["4bit", "8bit", "16bit"])       # one function, several sizes

Scoring a generator over several sizes at once is the point: it rewards a construction that works at every
size, where a single size rewards tricks that only fit that size.

Sizes up to 32 input bits are checked on every input. Larger ones are sampled (about a million inputs, with the
family's own hard cases), so a pass there is strong evidence and not a proof.

Build the verifier first: cargo build --release, in the repo.
"""
import importlib.util
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "logic-sim"))
from logic_sim import LogicSim  # noqa: E402

PENALTY = 1_000_000


class Scorer:
    def __init__(self):
        self.sim = LogicSim()

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.sim.close()

    def circuit(self, family, size, netlist, max_ticks=None, balanced=False):
        """{"ok", "score", "reason", "gates", "ticks", "timers"} for one netlist."""
        fail = lambda reason, wrong=1.0, **more: {"ok": False, "score": PENALTY + wrong, "reason": reason, **more}
        try:
            r = self.sim.verify(netlist, family=family, size=size)
        except Exception as e:      # a netlist the verifier cannot even read
            return fail(f"not a circuit: {e}")
        if "error" in r:
            return fail(r["error"])
        tested, wrong = r.get("tested") or 0, r.get("wrong") or 0
        share = wrong / tested if tested else 1.0
        if r.get("problems"):
            return fail("; ".join(r["problems"]), share if wrong else 1.0)
        gates, ticks, timers = r["gates"], r["ticks"], r.get("timers") or 0
        info = {"gates": gates, "ticks": ticks, "timers": timers, "verified": r.get("verified")}
        if max_ticks is not None and ticks > max_ticks:
            return fail(f"{ticks} ticks, the limit is {max_ticks}", 0.0, **info)
        if balanced:
            if r.get("balanced") is not True:
                return fail("not balanced: not every path from an input to an output is the same number of ticks", 0.0, **info)
            s = self.sim.stream(netlist)
            if s.get("problems") or s.get("wrong"):
                return fail("balanced on paper, but wrong when the inputs change every tick", 0.0, **info)
        return {"ok": True, "score": gates + (timers if balanced else 0), "reason": "", **info}

    def generator(self, family, build, sizes, max_ticks=None, balanced=False):
        """Run build(size, max_ticks, balanced) for every size. The score is the sum; every size has to pass."""
        results = {}
        for size in sizes:
            try:
                net = build(size, max_ticks, balanced)
            except Exception as e:
                results[size] = {"ok": False, "score": PENALTY + 1.0, "reason": f"the generator raised {type(e).__name__}: {e}"}
                continue
            results[size] = self.circuit(family, size, net, max_ticks, balanced)
        return {"ok": all(r["ok"] for r in results.values()), "score": sum(r["score"] for r in results.values()), "sizes": results}


def main():
    args = sys.argv[1:]

    def value(name):
        if name in args:
            i = args.index(name)
            v = args[i + 1]
            del args[i:i + 2]
            return v
        return None

    balanced = "--balanced" in args
    if balanced:
        args.remove("--balanced")
    max_ticks, sizes = value("--max-ticks"), value("--sizes")
    max_ticks = int(max_ticks) if max_ticks else None
    with Scorer() as s:
        if sizes:
            family, path = args
            spec = importlib.util.spec_from_file_location("candidate", path)
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            out = s.generator(family, module.build, sizes.split(","), max_ticks, balanced)
        else:
            family, size, path = args
            out = s.circuit(family, size, json.loads(Path(path).read_text()), max_ticks, balanced)
    print(json.dumps(out, indent=2))
    sys.exit(0 if out["ok"] else 1)


if __name__ == "__main__":
    main()
