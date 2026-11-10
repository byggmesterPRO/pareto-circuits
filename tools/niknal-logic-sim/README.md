# niknal-logic-sim

niknal's Scrap Mechanic logic simulator, copied from `sm-logic-stuff/src/circuitlib`. It is plain Python with no dependencies.

This is one of the candidate simulators. We compare it with `bygg-logic-sim`, then build the shared `tools/` simulator from the best parts of each (probably in Rust).

## What it does

- **Circuit model** (`Circuit`): gates (`AND OR XOR NAND NOR XNOR`) and timers (0–2400 tick delay), each at a grid position (`IVec3`). Inputs and outputs are named. A circuit input is a name wired into gates, not a gate itself.
- **Validation** (`Circuit.validate`): enforces per-element input and output limits, a limit on how many gates one circuit input may feed, no overlapping positions, optional bans on self-wiring and two-gate loops, and requires the parts to be physically connected.
- **Simulation** (`Simulation`): advances one tick at a time and re-evaluates only the elements whose inputs changed. Each gate adds one tick. Gates count how many of their inputs are on, which matches how Scrap Mechanic evaluates them. A timer with one input outputs its input `delay` ticks later.
- **Tests** (`TestCase`, `run_test`): a list of `SetInput` / `Tick` / `CheckOutput` actions. `run_test` returns `(correct, total)`.

## Run

```
cd tools/niknal-logic-sim
PYTHONPATH=. python3 example.py   # half adder, all four input combinations
```

Python 3.10 or newer. You can also install it with `pip install -e tools/niknal-logic-sim` and then `import circuitlib`.

## Known gaps

- No gate or tick counter. Ticks are measured separately by each search task's evaluator.
- No blueprint import or export.
- Timer timing, `delay == 0` in particular, hasn't been checked in game.
