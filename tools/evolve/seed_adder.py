"""A seed for a search: the adder as one function. It is the repo's `pairs` and `lookahead` designs, written
out in plain Python so that a search has something correct to start changing.

    python tools/evolve/evaluate.py adder tools/evolve/seed_adder.py --sizes 4bit,8bit,16bit
    python tools/evolve/evaluate.py adder tools/evolve/seed_adder.py --sizes 4bit,8bit,16bit --max-ticks 3

Inputs: a is bits 0 .. n-1, b is bits n .. 2n-1. Outputs: the sum in bits 0 .. n-1, then the carry.
"""
AND, OR, XOR, NAND, NOR, XNOR = range(6)


class Net:
    def __init__(self, inputs):
        self.inputs, self.gates = inputs, []

    def gate(self, mode, *signals):
        self.gates.append([mode, list(signals)])
        return self.inputs + len(self.gates) - 1

    def done(self, outputs):
        return {"inputs": self.inputs, "gates": self.gates, "outputs": outputs}


def small(n):
    """Two bits at a time in 7 gates (6 is impossible), the carry walking from pair to pair."""
    net, outs, carry, i = Net(2 * n), [], None, 0
    while i < n:
        low = [i, n + i] + ([carry] if carry is not None else [])
        if i + 1 == n:      # an odd bit at the top: a full adder, or a half adder if there is no carry yet
            s = net.gate(XOR, *low)
            carry = net.gate(AND, *low) if carry is None else net.gate(XOR, s, net.gate(AND, *low), net.gate(OR, *low))
            outs.append(s)
            break
        a1, b1 = i + 1, n + i + 1
        # "all three of a, b and the carry": always off in the first pair, which has no carry in
        any_ = net.gate(OR, *low)
        all_ = [net.gate(AND, *low)] if carry is not None else []
        high = net.gate(XOR, *low, a1, b1, any_, *all_)
        g8 = net.gate(OR, a1, b1, high)
        g9 = net.gate(NAND, a1, b1, any_, high)
        out = net.gate(XNOR, *low, a1, b1, any_, *all_, g8, g9)
        outs += [net.gate(XNOR, *low, high, g8, g9, out), high]
        carry, i = out, i + 2
    return net.done(outs + [carry])


def fast(n):
    """Carry lookahead: every sum bit is one XOR gate over "a carry arrives from bit j" terms. 3 ticks."""
    net = Net(2 * n)
    p = [net.gate(XOR, i, n + i) for i in range(n)]
    outs = []
    for i in range(n + 1):
        terms = [net.gate(AND, j, n + j, *p[j + 1:i]) for j in range(i)]
        if i < n:
            outs.append(net.gate(XOR, p[i], *terms) if terms else p[i])
        else:
            outs.append(net.gate(XOR, *terms) if len(terms) > 1 else terms[0])
    return net.done(outs)


def build(size, max_ticks, balanced):
    n = int(size.removesuffix("bit"))
    # the small one takes about 2 ticks per bit; anything tighter gets the lookahead
    return small(n) if max_ticks is None or max_ticks >= 2 * n + 2 else fast(n)
