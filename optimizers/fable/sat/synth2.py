"""Exact synthesis of several small gate circuits that share unknown signal encodings (SAT).

A circuit has input columns (one literal per row), k gates and output columns. Gates are Scrap Mechanic logic
gates (and, or, xor, nand, nor, xnor) with any number of inputs. Columns may be constants or SAT variables, so
the encoding of a value passed between two circuits can be left for the solver to choose.

    cnf = CNF()
    code = [[cnf.var() for _ in range(3)] for _ in range(5)]      # an unknown 3-wire code for 5 values
    c = cnf.circuit(columns, k)
    cnf.output(c, column)
    model = cnf.solve()
    gates, picks = cnf.decode(c, model)     # gates: [(mode, [signal])], picks: [signal] per output
"""
from pysat.solvers import Glucose4

AND, OR, XOR, NAND, NOR, XNOR = range(6)


class CNF:
    def __init__(self):
        self.n = 0
        self.clauses = []
        self.T = self.var()
        self.clauses.append([self.T])

    def var(self):
        self.n += 1
        return self.n

    def lit(self, x):
        if isinstance(x, bool):
            return self.T if x else -self.T
        return x

    def circuit(self, inputs, k):
        add = self.clauses.append
        cols = [[self.lit(x) for x in col] for col in inputs]
        n, R = len(cols), len(cols[0])
        sel, mode = [], []
        for g in range(k):
            sg = n + g
            s = [self.var() for _ in range(sg)]
            sel.append(s)
            add(list(s))
            m = [self.var() for _ in range(6)]
            mode.append(m)
            add(list(m))
            for a in range(6):
                for b in range(a + 1, 6):
                    add([-m[a], -m[b]])
            out = [self.var() for _ in range(R)]
            for r in range(R):
                A, O, X = self.var(), self.var(), self.var()
                us, ws = [], []
                for i in range(sg):
                    v = cols[i][r]
                    add([-A, -s[i], v])
                    u = self.var()
                    add([-u, s[i]])
                    add([-u, -v])
                    w = self.var()
                    add([-w, s[i]])
                    add([-w, v])
                    add([w, -s[i], -v])
                    add([-w, O])
                    us.append(u)
                    ws.append(w)
                add([A] + us)
                add([-O] + ws)
                P = ws[0]
                for w in ws[1:]:
                    p = self.var()
                    self.clauses += [[-p, P, w], [-p, -P, -w], [p, -P, w], [p, P, -w]]
                    P = p
                self.clauses += [[-X, P], [X, -P]]
                for md, base, inv in ((AND, A, 0), (NAND, A, 1), (OR, O, 0), (NOR, O, 1), (XOR, X, 0), (XNOR, X, 1)):
                    o = -base if inv else base
                    add([-m[md], -out[r], o])
                    add([-m[md], out[r], -o])
            cols.append(out)
        # symmetry breaking: two neighbouring gates that don't depend on each other come in a fixed order
        for g in range(k - 1):
            sg = n + g
            dep = sel[g + 1][sg]
            eq = self.T
            for i in range(sg - 1, -1, -1):
                a, b = sel[g][i], sel[g + 1][i]
                add([dep, -eq, -a, b])
                e = self.var()
                add([-eq, -a, -b, e])
                add([-eq, a, b, e])
                eq = e
        c = {"n": n, "k": k, "cols": cols, "sel": sel, "mode": mode, "picks": []}
        self._circuits = getattr(self, "_circuits", []) + [c]
        return c

    def output(self, c, column):
        col = [self.lit(x) for x in column]
        pick = [self.var() for _ in range(c["n"] + c["k"])]
        self.clauses.append(list(pick))
        for i, p in enumerate(pick):
            for r, t in enumerate(col):
                v = c["cols"][i][r]
                self.clauses.append([-p, -v, t])
                self.clauses.append([-p, v, -t])
        c["picks"].append(pick)

    def solve(self, budget=None):
        """Returns a set of true variables, None if impossible, or "budget" if it gave up."""
        for c in self._circuits:      # every gate feeds a later gate or an output
            for g in range(c["k"]):
                sg = c["n"] + g
                self.clauses.append([p[sg] for p in c["picks"]] + [c["sel"][h][sg] for h in range(g + 1, c["k"])])
        solver = Glucose4(bootstrap_with=self.clauses)
        if budget:
            solver.conf_budget(budget)
            res = solver.solve_limited()
        else:
            res = solver.solve()
        model = set(x for x in solver.get_model() if x > 0) if res else None
        solver.delete()
        return "budget" if res is None else model

    @staticmethod
    def decode(c, model):
        gates = []
        for g in range(c["k"]):
            md = next(a for a in range(6) if c["mode"][g][a] in model)
            gates.append((md, [i for i in range(c["n"] + g) if c["sel"][g][i] in model]))
        picks = [next(i for i in range(c["n"] + c["k"]) if p[i] in model) for p in c["picks"]]
        return gates, picks
