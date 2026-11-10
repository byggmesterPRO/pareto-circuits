"""Half adder: build, simulate every input combination, print results."""
from itertools import product

from circuitlib import (
    CheckOutput, Circuit, GateType, IVec3, SetInput, TestCase, Tick, ValidationConfig, run_test,
)

c = Circuit()
s = c.add_gate(IVec3(0, 0, 0), GateType.XOR)
k = c.add_gate(IVec3(1, 0, 0), GateType.AND)
for name in ("a", "b"):
    c.add_input(name)
    c.connect_input(name, s)
    c.connect_input(name, k)
c.add_output("sum", s)
c.add_output("carry", k)

print(c.validate(ValidationConfig(
    max_inputs=255, max_outputs=255, max_outputs_per_circuit_input=255,
    allow_selfwiring=False, allow_twoloop=False,
)))

for a, b in product([False, True], repeat=2):
    case = TestCase([
        SetInput("a", a), SetInput("b", b), Tick(1),
        CheckOutput("sum", a ^ b), CheckOutput("carry", a and b),
    ])
    print(a, b, run_test(c, case))
