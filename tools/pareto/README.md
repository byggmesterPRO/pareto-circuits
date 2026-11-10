# pareto

Runs the generators, measures what they make, and writes build slots.

```
cargo build --release
target/release/pareto designs adder 16bit           # every design for this size, checked and counted
target/release/pareto netlist adder 16bit lookahead out.json
target/release/pareto build adder 16bit             # which design wins fast, small and balanced
target/release/pareto build-all adder --write --authors yourname
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

`build-all` does the standard sizes: 2, 4, 8, 16, 32 and 64 bit.

## Adding a family

1. Say what it computes: add it to `tools/logic-sim/src/families.rs` (inputs and outputs as numbers, the
   function, and inputs that are likely to catch mistakes at large sizes).
2. Make `circuits/<family>/generator/` a small Rust library with `designs(bits)`, built on
   `logic_sim::build::Builder`. The builder folds constants away, shares equal gates, and doubles a gate that
   would drive more than 255 others.
3. Add the crate to the workspace `Cargo.toml` and to `designs()` in `tools/pareto/src/main.rs`.
4. Give it a test like `circuits/adder/generator/tests/designs.rs`: every design, small sizes, every input.

Families so far: `adder`, `double-dabble`.
