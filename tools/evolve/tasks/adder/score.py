"""Score an adder program on the task's cases. Lower is better; every case has to pass.

    python tools/evolve/tasks/adder/score.py tools/evolve/tasks/adder/initial.py
    python tools/evolve/tasks/adder/score.py my_candidate.py --json

A program is a Python file with build(n, max_ticks) that returns a netlist (see initial.py).
A case is (n, max_ticks): add two n-bit numbers in at most max_ticks ticks (None: no limit). The score of a
case is its gate count, and the total is the sum over the cases. Sizes are small enough that every input is
tried, so a pass is exact.

Set LOGIC_SIM to a logic-sim executable to use that one (for example a copy, while the repo is being rebuilt).
"""
import importlib.util
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from evaluate import PENALTY, Scorer  # noqa: E402

SIZES = [4, 8, 12]
# no limit, the fastest known, and the in-between delays nobody has good circuits for yet
LIMITS = [None, 3, 4, 5, 6, 8, 12]
CASES = [(n, t) for n in SIZES for t in LIMITS if t is None or t < 2 * n + 1]


def load(path):
    spec = importlib.util.spec_from_file_location("candidate", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.build


def score(path):
    rows, total, ok = [], 0, True
    try:
        build = load(path)
    except Exception as e:
        return {"ok": False, "total": PENALTY * len(CASES), "error": f"the program does not load: {type(e).__name__}: {e}", "cases": []}
    with Scorer() as s:
        for n, limit in CASES:
            try:
                r = s.circuit("adder", f"{n}bit", build(n, limit), max_ticks=limit)
            except Exception as e:
                r = {"ok": False, "score": PENALTY + 1.0, "reason": f"build raised {type(e).__name__}: {e}"}
            rows.append({"n": n, "max_ticks": limit, "ok": r["ok"], "gates": r.get("gates"), "ticks": r.get("ticks"), "reason": r["reason"]})
            total += r["score"]
            ok &= r["ok"]
    return {"ok": ok, "total": total, "cases": rows}


def main():
    out = score(sys.argv[1])
    if "--json" in sys.argv:
        print(json.dumps(out))
    else:
        if out.get("error"):
            print(out["error"])
        print(f"{'n':>3} {'limit':>5} {'gates':>6} {'ticks':>6}  result")
        for c in out["cases"]:
            limit = "-" if c["max_ticks"] is None else c["max_ticks"]
            print(f"{c['n']:>3} {limit:>5} {c['gates'] if c['gates'] is not None else '-':>6} {c['ticks'] if c['ticks'] is not None else '-':>6}  {'ok' if c['ok'] else 'FAIL: ' + c['reason'][:90]}")
        print("total", out["total"], "(all pass)" if out["ok"] else "(FAILED)")
    sys.exit(0 if out["ok"] else 1)


if __name__ == "__main__":
    main()
