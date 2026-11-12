# pareto

Runs the generators, measures what they make, and writes build slots.

```
cargo build --release
target/release/pareto designs adder 16bit           # every design for this size, checked and counted
target/release/pareto netlist adder 16bit lookahead out.json
target/release/pareto build adder 16bit             # which design wins fast, small and balanced
target/release/pareto build-all adder --write --authors yourname
target/release/pareto frontier adder 64bit          # the designs nothing beats on both gates and ticks
target/release/pareto pick adder 13bit --balanced --max-ticks 8 --out my-adder
```

Run it from inside the repo.

## How a build is made

1. The family's generator (`circuits/<family>/generator/`) offers several designs for the size.
2. Each design is checked on every input by `logic-sim` (sampled above 32 input bits) and its gates and ticks
   are counted. A design that is wrong, or that the game could not build, stops the run.
3. Each profile goes to the design that wins by the main README's rule: `fast` is fewest ticks, `small` is
   fewest gates, `balanced` is lowest gates x ticks.
4. With `--write`, a winner is written into its slot only if it beats what is there. The version goes up
   (major if the ticks changed, minor otherwise), the `localId` is kept, and the written slot is read back and
   checked once more.

So a generator never has to say which design is the fast one. It offers what it has, and the numbers decide.
A one-off search result that sits in a slot stays there until a design beats it.

`build-all` does every size the family has slots for. Each generator lists its own in `SIZES`: mostly 2, 4,
8, 16, 32 and 64 bit.

## The frontier, and picking one design

The three profiles are three points. The frontier is all of them: every design that no other design beats on
both gates and ticks. `pareto frontier <family> <size>` lists it, fastest first, and each one after is slower
and smaller. Whatever is wanted, the answer is on it.

`pareto pick <family> <size>` gives the one design that fits a wish, at any size, not only the sizes with
slots:

| Wish | Meaning |
|---|---|
| `--max-ticks n` | at most this slow |
| `--max-gates n` | at most this big |
| `--balanced` | every path the same number of ticks, so the inputs can change every tick |
| `--max-period n` | new inputs, or operations, at least this often: every n ticks |
| `--max-read n` | with memory: at most this long from an input to the outputs |
| `--max-write n` | with memory: at most this long from a pulse to the outputs |
| `--max-setup n` | with memory: the inputs steady at most this long before a pulse |
| `--fastest` | the fastest that fits; without it, the smallest |
| `--out <folder>` | write it: `blueprint.json` and `description.json` (the folder can go into the game's Blueprints folder), `netlist.json`, `pins.json` |

If nothing fits it says so, and what the fastest and the smallest known are.

Balanced here is the game's meaning, not the profile's (see `tools/logic-sim/README.md`). Every design
without memory is also measured in its balanced form: timers are added until every path is the design's
`ticks`, and the result is checked to be the same gates, to have equal paths, and to give the right outputs
on inputs that change every tick. A timer counts as one part, so the size of a balanced design is its gates
plus its timers. `designs` shows that size in the `balanced` column, and `frontier` prints a second frontier
for it. The two frontiers are not the same designs: one that is small as it is can need many timers.

Two designs with the same gates and ticks are told apart by that balanced size: the one that needs fewer
parts balanced wins, in the profiles too.

A family with memory has more delays than one, so its frontier has more axes: read, write, setup, period and
gates (`tools/logic-sim/README.md` says what each is). A design is on it unless another is at least as good
on all five. `frontier` prints them all, and `pick` takes a wish for each. `--balanced` is not for these;
`--max-period` is the question that takes its place. The `ticks` in a build's `meta.json` stays the one
number the family README names.

Best means best of what the generators offer. Nothing here proves that no smaller circuit exists, and the
timer count of a balanced design is a search result too.

## Adding a family

1. Say what it computes: add it to `tools/logic-sim/src/families.rs` (inputs and outputs as numbers, the
   function, and inputs that are likely to catch mistakes at large sizes).
2. Make `circuits/<family>/generator/` a small Rust library with `SIZES` and `designs(size)`, built on
   `logic_sim::build::Builder`. The builder folds constants away, shares equal gates, and doubles a gate that
   would drive more than 255 others. `logic_sim::parts` has pieces to reuse: carry networks, the 4-gate full
   adder, decoders, and a 3-tick circuit for any small function.
3. Add the crate to the workspace `Cargo.toml`, to `tools/pareto/Cargo.toml`, and to `generator()` in
   `tools/pareto/src/main.rs`.
4. `cargo test --release -p pareto` then checks every design of every family on every input, at every size
   with 16 input bits or fewer.

Nothing in `pareto` asks for a family by name whether it has memory. A design is handed to
`logic_sim::verify` with the family's spec, and comes back measured either way.

A family with memory is a `Machine` in `families.rs` instead: which inputs are one-tick pulses, and a model
that says what the outputs should be after an operation. Its generator makes loops with `Builder::later` and
`Builder::wire`. It is checked on the tick simulator with random operations, and its ticks are measured there.

Families so far: `absolute`, `adder`, `adder-subtractor`, `alu`, `barrel-shifter`, `comparator`, `counter`, `decoder`,
`decrementer`, `demultiplexer`, `divider`, `double-dabble`, `incrementer`, `multiplexer`, `multiplier`,
`negate`, `ram`, `register`, `seven-segment`, `square`, `square-root`, `subtractor`.
