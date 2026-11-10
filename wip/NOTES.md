# wip

Things tried for double dabble that are not part of a working system.

- `evolve.rs`: blind evolution of a netlist (mutate wires and gate types, keep the child if it is still correct
  and no bigger). From a 118-gate start it found nothing in several minutes on 18 cores. The targeted rewrites in
  `optimizers/fable/resub` got the same start to 89.
- A 2-layer lookup table (terms straight off the input gates) cannot respect the 255-connection limit at 12 bits:
  the ones-digit outputs each need about 1000 terms on one gate.
- Buffer trees fix a fan-out above 255 but cost a tick per level. Copying the driving gate instead keeps the ticks.
