# bygg-logic-sim

byggmesterPRO's simulator and verifier, as used for the double dabble builds. It sits here next to
`niknal-logic-sim` until the best parts of both are merged into one `logic-sim` in Rust.

Plain Python 3.10+, no packages needed.

| File | Job |
|---|---|
| `netlist.py` | The netlist format, bit-parallel evaluation, gate and tick count, blueprint export and import |
| `families.py` | What each circuit family has to compute |
| `verify.py` | Checks a build slot or a netlist: correct on every input, gates, ticks, connection limit |
| `delay.py` | Worst output delay actually seen over random input changes |
| `sim.py` | Tick-by-tick simulator for any blueprint: gates, timers, switches, buttons, lamps, feedback loops |
| `make_build.py` | Netlist to build slot: blueprint, description.json, meta.json, version bump |

```
python tools/bygg-logic-sim/verify.py circuits/double-dabble/builds/12bit/fast
python tools/bygg-logic-sim/verify.py net.json double-dabble 12bit
python tools/bygg-logic-sim/delay.py circuits/double-dabble/builds/12bit/small
```

## How it works

**Netlist.** `{"inputs": n, "gates": [[mode, [signals]], ...], "outputs": [signal or null, ...]}`. Signal
i < n is input bit i, n + g is gate g, and a gate only reads earlier signals. Modes are the game's: 0 and,
1 or, 2 xor, 3 nand, 4 nor, 5 xnor. Optimizers and generators hand over netlists; `netlist.py` turns them into
blueprints and back.

**Bit-parallel evaluation.** For a circuit without memory, all test inputs run at once. Every signal is one
Python integer with one bit per test input, so a gate is a few AND/OR/XOR operations on big integers. Checking the
12bit fast build on all 4096 inputs takes well under a second. Up to 20 input bits every input is tried;
above that, edge cases plus 200 000 random inputs.

**Gates and ticks.** As the main README defines them: gates is every logic gate in the blueprint, ticks is the
number of gates on the longest path from an input to an output with both ends counted.

**Pins.** `meta.json` lists the controller id behind every input and output bit under `pins`. An output can be
an input gate (the bit passes through) or `null` (the bit is always 0 and has no gate).

**Tick simulator.** `sim.py` steps a blueprint one game tick at a time: every part reads the previous tick's
state. This is the one that handles memory, clocks and timers. It is slow (pure Python, one input at a time).

## What to carry into logic-sim

- The bit-parallel evaluation is the reason exhaustive checks are cheap. In Rust with 64-bit words it would be
  far faster still.
- `delay.py` runs the tick-accurate model bit-parallel over many input changes at once. The same trick would
  speed up a tick simulator for circuits with memory.
- The tick model is not checked against the game yet (see the top of `sim.py`). That should happen before
  either simulator is trusted as the judge.
