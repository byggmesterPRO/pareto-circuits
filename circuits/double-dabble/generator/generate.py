"""Double dabble generator: binary to BCD for any number of bits.

    python circuits/double-dabble/generator/generate.py <bits> <out netlist.json>

Only one profile so far: the classic shift-and-add-3 chain, built from the smallest known add-3 cell. It is
small but slow (the delay grows with the number of bits). The fast and small 12bit builds in this repo are
one-off search results that beat it; see the family README.

How it works: feed the bits in from the most significant end. Before each new bit is shifted in, every BCD
digit that is 5 or more gets 3 added. A digit only gets a cell at the steps where it can reach 5 at all.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "tools" / "bygg-logic-sim"))
import netlist

# Add-3 cell: digit bits 0..3 (value 0..9) -> the digit, plus 3 if it was 5 or more. 7 gates, which exact
# synthesis (SAT) showed is the minimum for a cell taken alone. Signals 0..3 are the digit bits, 4.. the gates.
CELL = [(4, [0, 2, 3]), (1, [0, 1, 4]), (5, [3, 5]), (0, [3, 5, 6]), (2, [2, 3, 6, 7]), (2, [0, 8]),
        (5, [0, 1, 2, 3, 4, 8])]
CELL_OUT = [9, 10, 6, 8]


class Builder:
    """Gate list with constant folding. A signal is an int, False (always 0) or True (always 1)."""

    def __init__(self, inputs):
        self.n = inputs
        self.gates = []
        self.memo = {}

    def gate(self, mode, ins):
        key = (mode, tuple(sorted(ins)))
        if key not in self.memo:
            self.gates.append([mode, list(key[1])])
            self.memo[key] = self.n + len(self.gates) - 1
        return self.memo[key]

    def add(self, mode, ins):
        base, invert = mode % 3, mode >= 3
        if base == 2:                                   # xor: constants flip the result, pairs cancel
            invert ^= sum(1 for x in ins if x is True) % 2 == 1
            real = [x for x in ins if x is not True and x is not False]
            real = sorted(x for x in set(real) if real.count(x) % 2)
            if not real:
                return invert
            if len(real) == 1 and not invert:
                return real[0]
            return self.gate(5 if invert else 2, real)
        absorbing = base == 1                           # and: a 0 decides it; or: a 1 decides it
        if any(x is absorbing for x in ins):
            return absorbing != invert
        real = sorted(set(x for x in ins if x is not (not absorbing)))
        if not real:
            return (not absorbing) != invert
        if len(real) == 1 and not invert:
            return real[0]
        return self.gate(base + 3 if invert else base, real)


def generate(bits):
    digits = len(str((1 << bits) - 1))
    b = Builder(bits)
    reg = [[False] * 4 for _ in range(digits)]
    for k in range(bits):                               # k bits are in the register, the next one is bit bits-1-k
        for d in range(digits):
            if (1 << k) - 1 >= 5 * 10 ** d:             # this digit can be 5 or more now
                sig = list(reg[d])
                for mode, ins in CELL:
                    sig.append(b.add(mode, [sig[i] for i in ins]))
                reg[d] = [sig[o] for o in CELL_OUT]
        carry = bits - 1 - k
        for d in range(digits):
            reg[d], carry = [carry] + reg[d][:3], reg[d][3]
    outputs = [None if s is False else s for d in range(digits) for s in reg[d]]
    if any(s is True for s in outputs):
        raise ValueError("an output folded to constant 1")
    return {"inputs": bits, "gates": b.gates, "outputs": outputs}


if __name__ == "__main__":
    netlist.save(sys.argv[2], generate(int(sys.argv[1])))
