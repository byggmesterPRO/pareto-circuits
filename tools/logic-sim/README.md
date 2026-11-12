# logic-sim

Simulator and verifier for Scrap Mechanic logic, in Rust. Put together from `bygg-logic-sim` and
`niknal-logic-sim`, and meant for any circuit, from any language.

It is a candidate for the shared verifier, not the judge yet: both of us should look it over first, and the
tick model still has to be checked against the game.

## Build

```
cargo build --release
```

Run it in the repo. It builds every Rust tool here, for the CPU of your machine, into `target/release/`.

## Commands

A circuit is a build slot folder, a `blueprint.json` or a `netlist.json`. Every command prints one line of
JSON and exits 1 if there is a problem. `logic-sim help` lists them.

| Command | What it does |
|---|---|
| `verify <slot>` | Checks a build slot against its `meta.json`: correct, gates, ticks, buildable |
| `verify <circuit> <family> <size>` | Checks against a known family, for example `double-dabble 12bit` |
| `equal <circuit> <reference>` | Do two circuits give the same outputs for every input |
| `check <circuit> --table <file>` | Checks against a list with one output value per input |
| `check <circuit> --cases <file>` | Checks chosen inputs: `[[input, output], ...]` |
| `count <circuit>` | Gates, timers and ticks, and whether it is balanced |
| `delay <circuit> [jumps]` | Worst output delay seen over random input changes |
| `stream <circuit> [steps] [every]` | New inputs every tick (or every `every` ticks): are the outputs right, `ticks` ticks later |
| `balance <circuit> <out.json> [ticks]` | Pads with timers until every path is the same; writes a netlist |
| `validate <circuit>` | Can it be built in the game |
| `run <circuit> <script>` | Runs a test script on the tick simulator |
| `serve` | Stays running: one JSON request per line in, one reply per line out |

A bare `blueprint.json` does not say which gates are the inputs and outputs. Give them with
`--pins <file>`: `{"inputs": [controller id, ...], "outputs": [controller id or null, ...]}`. A build slot has
them in its `meta.json`, and a netlist has them built in.

## Any circuit

- **Without memory** (adders, decoders, converters): `verify`, `equal` and `check`. You do not need a family
  in Rust. Hand over the right answers as a table or as cases, or compare against a circuit you trust, such as
  a generator's output. Up to 32 input bits every input is tried; above that, edge cases plus a million random
  inputs. With a table or cases, exactly what you give is tried.
- **With memory or loops** (RAM, counters, clocks): `run` with a test script. One step per line:
  `set in0 1`, `release in0`, `tick 3`, `settle`, `check out2 0`. A name is `in<k>` or `out<k>` for a pin, or
  `#<controller id>` for any part. `//` starts a comment.

A family in `src/families.rs` is only needed for `verify` on build slots, and for sizes where a table would be
too big. `ROADMAP.md` lists the ones that are there. With memory: `counter`, `ram` and `register`; for those `verify`
runs random operations against a model and reports the measured delays.

Inputs and outputs can be up to 128 bits wide each, which is what a 64bit adder needs.

## Balanced circuits

Balanced is the game's word for a circuit where every path from an input to an output is the same number of
ticks. Its outputs always belong to the inputs of one tick, so the inputs can change every tick and a result
comes out every tick, `ticks` ticks later. It is not a build profile: the profile for lowest gates x ticks is called `efficient`.

- `count` says whether a circuit is balanced, from the wiring: `balanced`, and `shortest_path` next to `ticks`
  (the longest). They are equal when it is.
- `stream` is the test of it. It runs the circuit on the tick simulator with new random inputs on every tick,
  256 streams side by side, and checks every output on every tick against the inputs of `ticks` ticks before.
- `balance` makes a circuit balanced without touching its gates: it adds timers on the wires that are too
  short. A timer set to `d` is `d + 1` gates in a row (`cargo test` checks that on inputs that change every
  tick), so any delay up to 2400 ticks is one part. Readers of a signal that need the same delay share the
  timer. Give `ticks` to balance at more than the longest path.

A circuit that is not balanced still keeps up with inputs that come slowly enough. `count` gives that as
`period`: the longest path minus the shortest, plus 1. New inputs every `period` ticks, and the outputs are
right `ticks` ticks after each. `stream <circuit> <steps> <every>` tests it. Balanced is a period of 1.

How many timers it takes depends on where the delays go: a gate off the longest path can sit early with the
delay behind it, or late with the delay in front. `balance` searches for the placing with the fewest timers
(`src/balance.rs` says how). The count it gets is a best-known, not a proven minimum.

A netlist holds timers as `"timers": [[signal it reads, setting], ...]`. They are numbered after the gates:
signal `inputs + gates + t` is timer `t`. In a blueprint a timer takes one cell, like a gate, and stands two
blocks tall in it.

## With and without memory

A family is one of two kinds, and `src/families.rs` gives either as a `Spec`: a function of the inputs, or a
model with a state. `verify` (`src/verify.rs`) takes a circuit and a `Spec` and is the one place where the
two are told apart: without memory the circuit is tried on every input, with memory it is run through random
operations on the tick simulator. The reply has the same fields for both.

What differs is the timing, because one delay is not enough for a circuit with memory:

| Field | Without memory | With memory |
|---|---|---|
| `read` | the longest path (the same as `ticks`) | from a data or address input to the outputs |
| `write` | null | from a pulse to the outputs |
| `setup` | null | how long the inputs take to settle inside, before a pulse |
| `period` | longest path - shortest + 1 | the shortest gap between operations that works |
| `pulse_at` | null | where in that gap the pulse goes |
| `ticks` | `read` | `read` or `write`, whichever the family README names |

`period` is the answer to "how often" for both kinds. With memory, an operation is `period` ticks long: the
data and address change at its start, the pulse (if there is one) comes `pulse_at` ticks later, and at its
end the outputs have to be what the model says. `verify --pace` (`"pace": true` in a request) tries periods
from 1 up and gives the first that works on 3000 random operations in a row, over 64 copies. It is what was
seen to work, not a proof. Without `--pace`, `period` and `pulse_at` are null for a circuit with memory.

Whether a circuit has memory is never declared: it has, if a part reads itself around a loop
(`Circuit::feed_forward`). Timers do not make memory. A circuit of gates and timers without a loop is still
a function of its inputs, only later.

## From Python, or any other language

```python
from logic_sim import LogicSim      # tools/logic-sim/logic_sim.py

sim = LogicSim()
sim.verify("circuits/double-dabble/builds/12bit/fast")
sim.check(netlist, table=[f(x) for x in range(1 << bits)])
sim.check(netlist, reference=other_netlist)
sim.run(blueprint, "set #3 1\ntick 2\ncheck #7 1")
sim.stream(netlist)                 # does it keep up with inputs that change every tick
sim.balance(netlist)["netlist"]     # the same circuit with timers added until it does
```

The client keeps one `logic-sim serve` running, so a call costs no start-up, and circuits can be sent as
dicts without writing files. Any language that can start a program and read and write lines can do the same.
The request format is at the top of `src/api.rs`.

## What came from where

| Part | From | Why |
|---|---|---|
| One tick per part, reading the previous tick's state | both | The two agreed already |
| Only re-evaluate parts whose inputs changed | niknal | Most of a circuit is idle on most ticks |
| Timers take one input | niknal | That is how the game's timer works |
| Buildability rules: wire limits, self-wiring, cross-wiring, overlaps, parts touching | niknal | bygg had only the 255 wire limit |
| Test scripts (set, tick, check) | niknal | The way to test circuits with memory |
| Many test cases at once, one bit each in every signal | bygg | Makes trying every input cheap |
| Gates and ticks as the main README counts them | bygg | niknal's library does not count |
| Reading blueprint.json, build slots and netlists; pins from meta.json | bygg | niknal's circuits only exist in Python |
| Switches, buttons and lamps in a blueprint | bygg | |
| Worst delay actually seen | bygg | |

New here: the tick simulator itself runs many copies of the circuit at once, so the bit-parallel trick also
works for circuits with memory and timers. Checking against a table, cases or another circuit is new too.

## Do the three agree

`python tools/logic-sim/compare.py` runs all three. They agree on:

- the 12bit fast and small builds and the generator at 4, 8, 12 and 16 bits: correct, same gates, same ticks
- 30 random circuits with feedback loops, self-wired gates and timers, 200 ticks each with random inputs:
  the same state for every part on every tick

`cargo test` has 23 tests here: a half adder, a NOR latch, timers (one, and against rows of gates), balancing,
independent lanes, tables, cases, circuit
against circuit, parts in any order, more than 64 output bits, 128 input bits, the double dabble reference,
doubling a gate that drives too many, and writing a blueprint and reading it back.

## Speed

Measured on one machine with 20 threads, on the double dabble generator's circuits, trying every input.
Times are whole program runs.

| Size | Inputs tried | Gates | logic-sim | bygg (Python) |
|---|---|---|---|---|
| 12bit | 4096 | 135 | 0.02 s | 0.16 s |
| 16bit | 65 536 | 244 | 0.02 s | 5.5 s |
| 24bit | 16.8 million | 572 | 0.07 s | |
| 28bit | 268 million | 785 | 0.66 s | |
| 32bit | 4.3 billion | 1033 | 15 s | |

About 0.015 s of each run is starting the program and reading the file. Through `serve` that is gone: one
check of the 12bit circuit takes 0.9 ms, against 57 ms for the Python verifier inside a running Python.

The first version took 2.9 s on the 28bit run and 50 s on the 32bit run. Three changes together brought that
to 0.66 s and 15 s: each gate writes its result straight into place, a block of test inputs is kept small
enough for the CPU cache (512 KB measured best), and the double dabble reference counts in decimal instead of
dividing for every input. They were not timed one by one. Building for the newest CPU instructions was timed,
and made no difference.

## Open points

- **The tick model is not checked against the game.** All three simulators share it, so their agreeing does not
  prove it. A few small circuits built in game (a timer, a latch, a clock) would settle it.
- **Do parts have to touch?** niknal's validator requires the parts of a circuit to form one connected piece.
  The three 12bit builds have their input row, output row and other gates two blocks apart, so `validate`
  reports 3 separate pieces. For now that is a warning. If the game needs them to touch, the layout in
  `bygg-logic-sim/netlist.py` has to change and the builds get a new version.
- Self-wired and cross-wired parts are reported as warnings, since some builds want them (memory cells).
- A timer is counted as one block when checking overlaps and touching; in the game it takes two. The
  blueprint writer stands a timer on end, so that its second block is above the layer the gates are in. The
  timer's size (1 x 2 x 1) is from the game's files; that this way of turning it puts the long side up is
  worked out from the axes, and has not been looked at in the game.
- **Balanced circuits lean on the tick model more than anything else here.** One tick off in how the game
  counts a timer and a balanced build is not balanced. A balanced adder built in the game, fed from a counter,
  would settle it.
- Test scripts run one copy of the circuit. Checking a memory circuit on many inputs at once needs the
  library (`sim.rs`), not the command line.
- The optimizers in `optimizers/fable` do not use it yet.

## For generators

`src/build.rs` has what a generator needs: a `Builder` that folds constants away and shares equal gates, a
`Netlist`, and the blueprint writer. See `tools/pareto/README.md`.
