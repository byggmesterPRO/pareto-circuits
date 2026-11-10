"""Exact search for the smallest divide-by-5 cell of a double dabble, with the state encoding left free.

A double dabble digit is a chain of cells. Each takes the remainder r (0..4) and one new bit b, and gives the
quotient bit (2r + b >= 5) and the new remainder (2r + b) mod 5. In plain binary the cell needs 7 gates. Here
the solver also chooses how r is coded on w wires, the same code going in and coming out.

    python cell_search.py <wires> <gates> [conflict budget]

Prints "found" with the code and the cell, "impossible", or "budget" if it gave up. Needs python-sat.
What it showed: 3 wires need 6 gates (5 is impossible); with 4 or 5 wires 4 gates is impossible and 5 was
not settled.
"""
import json
import sys
import time

from synth2 import CNF

ROWS = [(r, b) for r in range(5) for b in (0, 1)]


def search(w, k, budget):
    cnf = CNF()
    code = [[cnf.var() for _ in range(w)] for _ in range(5)]
    cols = [[code[r][j] for r, b in ROWS] for j in range(w)] + [[bool(b) for r, b in ROWS]]
    cell = cnf.circuit(cols, k)
    cnf.output(cell, [2 * r + b >= 5 for r, b in ROWS])
    for j in range(w):
        cnf.output(cell, [code[(2 * r + b) % 5][j] for r, b in ROWS])
    model = cnf.solve(budget=budget)
    if model == "budget":
        return {"result": "budget"}
    if model is None:
        return {"result": "impossible"}
    gates, picks = CNF.decode(cell, model)
    return {"result": "found", "code": [[int(code[v][j] in model) for j in range(w)] for v in range(5)],
            "gates": gates, "quotient": picks[0], "next_state": picks[1:]}


if __name__ == "__main__":
    w, k = int(sys.argv[1]), int(sys.argv[2])
    budget = int(sys.argv[3]) if len(sys.argv) > 3 else 3_000_000
    t0 = time.time()
    out = search(w, k, budget)
    out.update(wires=w, gates_allowed=k, seconds=round(time.time() - t0, 1))
    print(json.dumps(out))
