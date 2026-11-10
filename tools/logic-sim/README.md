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
| `count <circuit>` | Gates and ticks |
| `delay <circuit> [jumps]` | Worst output delay seen over random input changes |
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
- **With memory, timers or loops** (RAM, counters, clocks): `run` with a test script. One step per line:
  `set in0 1`, `release in0`, `tick 3`, `settle`, `check out2 0`. A name is `in<k>` or `out<k>` for a pin, or
  `#<controller id>` for any part. `//` starts a comment.

A family in `src/families.rs` is only needed for `verify` on build slots, and for sizes where a table would be
too big. There so far: `adder`, `barrel-shifter`, `comparator`, `decoder`, `demultiplexer`, `double-dabble`,
`multiplexer`, `multiplier`, `seven-segment` and `subtractor`. With memory: `counter`, `ram` and `register`; for those `verify`
runs random operations against a model and reports the measured delays.

Inputs and outputs can be up to 128 bits wide each, which is what a 64bit adder needs.

## From Python, or any other language

```python
from logic_sim import LogicSim      # tools/logic-sim/logic_sim.py

sim = LogicSim()
sim.verify("circuits/double-dabble/builds/12bit/fast")
sim.check(netlist, table=[f(x) for x in range(1 << bits)])
sim.check(netlist, reference=other_netlist)
sim.run(blueprint, "set #3 1\ntick 2\ncheck #7 1")
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

`cargo test` has 15 tests here: a half adder, a NOR latch, a timer, independent lanes, tables, cases, circuit
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
- A timer is counted as one block when checking overlaps and touching; in the game it takes two.
- Test scripts run one copy of the circuit. Checking a memory circuit on many inputs at once needs the
  library (`sim.rs`), not the command line.
- The optimizers in `optimizers/fable` do not use it yet.

## For generators

`src/build.rs` has what a generator needs: a `Builder` that folds constants away and shares equal gates, a
`Netlist`, and the blueprint writer. See `tools/pareto/README.md`.
