"""Adder for Scrap Mechanic logic, as one function that a search can rewrite.

The task: build(n, max_ticks) returns a circuit that adds two n-bit numbers, is at most max_ticks deep, and
uses as few gates as possible.

Rules of the game's logic:
- Gate modes: AND, OR, XOR, NAND, NOR, XNOR. A gate takes ANY number of inputs (up to 255) and costs the same
  whatever the number. XOR is on when an odd number of its inputs are on. A wide gate is as cheap as a narrow
  one, so the best circuits here do not look like textbook two-input circuits.
- A signal may feed at most 255 gates.
- Ticks: every gate on a path costs one tick. The depth of the circuit is the longest path, counted in gates.
  The inputs themselves cost nothing.
- Size: the number of gates. The inputs are not counted.

Pins: a is input bits 0 .. n-1, b is input bits n .. 2n-1, lowest bit first. Outputs: the n sum bits, lowest
first, then the carry out.

Known so far: 4 gates per bit with a full adder per bit; 3.5 per bit with the 7-gate block for two bits below
(exact search: 6 gates for two bits with carry in and out is impossible); 3 ticks at any size with carry
lookahead, at about n*n/2 gates. Nothing good is known in between: a circuit that must finish in, say, n ticks
is built here as the full lookahead, which is far bigger than it needs to be.
"""
AND, OR, XOR, NAND, NOR, XNOR = range(6)


class Net:
    """A circuit under construction. gate() returns the new gate's signal number."""

    def __init__(self, inputs):
        self.inputs, self.gates = inputs, []

    def gate(self, mode, *signals):
        self.gates.append([mode, list(signals)])
        return self.inputs + len(self.gates) - 1

    def done(self, outputs):
        return {"inputs": self.inputs, "gates": self.gates, "outputs": list(outputs)}


# EVOLVE-BLOCK-START
def chain(n):
    """3 gates per bit. The carry between bits is not one wire but the parity of up to three wires (found by
    exact search with a free carry code): after bit i with sum s, the wires s, AND(a, b, s), OR(a, b, s) have
    odd parity exactly when the carry out is on. The next sum gate is one XOR over a, b and all three wires,
    so no gate is spent on forming the carry itself. Bit 0 passes AND(a0, b0) as the code; at the top the
    carry is turned into one wire with an XOR."""
    net, outs = Net(2 * n), []
    outs.append(net.gate(XOR, 0, n))
    wires = [net.gate(AND, 0, n)]
    for i in range(1, n):
        a, b = i, n + i
        s = net.gate(XOR, a, b, *wires)
        wires = [s, net.gate(AND, a, b, s), net.gate(OR, a, b, s)]
        outs.append(s)
    carry = wires[0] if len(wires) == 1 else net.gate(XOR, *wires)
    return net.done(outs + [carry])


def small(n):
    """Two bits at a time in 7 gates, the carry walking from pair to pair. About 2 ticks per bit."""
    net, outs, carry, i = Net(2 * n), [], None, 0
    while i < n:
        low = [i, n + i] + ([carry] if carry is not None else [])
        if i + 1 == n:      # an odd bit at the top: a full adder, or a half adder if there is no carry yet
            s = net.gate(XOR, *low)
            carry = net.gate(AND, *low) if carry is None else net.gate(XOR, s, net.gate(AND, *low), net.gate(OR, *low))
            outs.append(s)
            break
        a1, b1 = i + 1, n + i + 1
        any_ = net.gate(OR, *low)
        # "a, b and the carry are all on": never so in the first pair, which has no carry in
        all_ = [net.gate(AND, *low)] if carry is not None else []
        high = net.gate(XOR, *low, a1, b1, any_, *all_)
        g8 = net.gate(OR, a1, b1, high)
        g9 = net.gate(NAND, a1, b1, any_, high)
        out = net.gate(XNOR, *low, a1, b1, any_, *all_, g8, g9)
        outs += [net.gate(XNOR, *low, high, g8, g9, out), high]
        carry, i = out, i + 2
    return net.done(outs + [carry])


def fast(n):
    """Carry lookahead. A carry reaches bit i from exactly one lower bit j (the nearest that starts one, with
    every bit between passing it on), so the terms are never on two at a time and a sum bit is one XOR gate
    over them. 3 ticks at any size."""
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


def build(n, max_ticks):
    """The smallest adder this code knows that is at most max_ticks deep (None: no limit)."""
    if max_ticks is None or max_ticks >= 2 * n:     # the chain is 2n ticks deep
        return chain(n)
    if max_ticks >= 2 * n + 1:
        return small(n)
    return fast(n)
# EVOLVE-BLOCK-END


def run_experiment(n, max_ticks):
    """Called by the evaluator, once per case."""
    return {"n": n, "max_ticks": max_ticks, "netlist": build(n, max_ticks)}
