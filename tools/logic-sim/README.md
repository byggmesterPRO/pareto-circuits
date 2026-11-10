# logic-sim

The simulator and verifier in Rust, put together from `bygg-logic-sim` and `niknal-logic-sim`.

It is a candidate for the shared verifier, not the judge yet: both of us should look it over first, and the
tick model still has to be checked against the game.

```
cargo build --release --manifest-path tools/logic-sim/Cargo.toml
tools/logic-sim/target/release/logic-sim verify circuits/double-dabble/builds/12bit/fast
```

Every command prints one line of JSON and exits 1 if there is a problem.

| Command | What it does |
|---|---|
| `verify <slot>` | Checks a build slot against its `meta.json`: correct, gates, ticks, buildable |
| `verify <netlist.json> <family> <size>` | The same for a netlist |
| `count <slot or netlist>` | Gates and ticks |
| `delay <slot or netlist> [jumps]` | Worst output delay seen over random input changes |
| `validate <slot or blueprint.json>` | Can it be built in the game |
| `run <slot or blueprint.json> <script>` | Runs a test script on the tick simulator |

A test script is one step per line: `set in0 1`, `release in0`, `tick 3`, `settle`, `check out2 0`. A name is
`in<k>` or `out<k>` for a pin, or `#<controller id>` for any part. `//` starts a comment.

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
works for circuits with memory and timers. In both Python simulators it was one input at a time.

## Do the three agree

`python tools/logic-sim/compare.py` runs all three. They agree on:

- the 12bit fast and small builds and the generator at 4, 8, 12 and 16 bits: correct, same gates, same ticks
- 30 random circuits with feedback loops, self-wired gates and timers, 200 ticks each with random inputs:
  the same state for every part on every tick

`cargo test` covers a half adder, a NOR latch, a timer, independent lanes and the double dabble function.

## Speed

Measured on one machine with 20 threads. logic-sim's times are whole program runs, of which about 0.03 s is
starting up and reading the file. The Python times are the checking step alone, inside a Python that is already
running; as a whole program the bygg verifier takes 0.35 s on the first row.

| Check | bygg | niknal | logic-sim |
|---|---|---|---|
| 12bit fast build, every input | 0.05 s | 0.65 s | 0.03 s |
| generator 16bit, every input | 5.5 s | 8.5 s for 2000 of the 65536 inputs | 0.05 s |
| generator 24bit, every input | | | 0.45 s |
| generator 28bit, every input | | | 2.9 s |
| generator 32bit, all 4.3 billion inputs | | | 50 s |

So logic-sim tries every input up to 32 bits, where the Python verifier stops at 20 and samples above that.
Above 32 bits it tests edge cases plus a million random inputs.

## Open points

- **The tick model is not checked against the game.** All three simulators share it, so their agreeing does not
  prove it. A few small circuits built in game (a timer, a latch, a clock) would settle it.
- **Do parts have to touch?** niknal's validator requires the parts of a circuit to form one connected piece.
  The three 12bit builds have their input row, output row and other gates two blocks apart, so `validate`
  reports 3 separate pieces. For now that is a warning. If the game needs them to touch, the layout in
  `bygg-logic-sim/netlist.py` has to change and the builds get a new version.
- Self-wired and cross-wired parts are reported as warnings, since some builds want them (memory cells).
- A timer is counted as one block when checking overlaps and touching; in the game it takes two.
- Only `double-dabble` is known as a family so far (`src/families.rs`).
- The optimizers still use the Python verifier inside their search loops. There are no Python bindings yet.
