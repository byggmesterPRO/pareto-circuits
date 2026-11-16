# Adder-subtractor

Adds two binary numbers, or subtracts them when the mode pin is on. Size is the number of bits of each number.

## Pins

- **Inputs:** the yellow gates in the first row. x = 0 .. n-1 is a, x = n .. 2n-1 is b, least significant bit
  first in each, and x = 2n is the mode: off adds, on gives a - b.
- **Outputs:** the green gates in the second row. x = 0 .. n-1 is the result, x = n is the carry out of an
  addition or the borrow of a subtraction (on when a < b).

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 13 | 4 | lookahead | exhaustive | |
| 2bit | small | v2.0 | 9 | 5 | sum-carry | exhaustive | |
| 2bit | efficient | v2.0 | 9 | 5 | sum-carry | exhaustive | |
| 4bit | fast | v2.0 | 27 | 3 | clauses | exhaustive | |
| 4bit | small | v2.0 | 17 | 9 | sum-carry | exhaustive | |
| 4bit | efficient | v2.0 | 27 | 3 | clauses | exhaustive | |
| 8bit | fast | v1.1 | 62 | 4 | clauses | exhaustive | |
| 8bit | small | v2.0 | 33 | 17 | sum-carry | exhaustive | |
| 8bit | efficient | v2.0 | 62 | 4 | clauses | exhaustive | |
| 16bit | fast | v1.1 | 155 | 4 | clauses | sampled | |
| 16bit | small | v2.0 | 65 | 33 | sum-carry | sampled | |
| 16bit | efficient | v1.1 | 113 | 5 | factors | sampled | |
| 32bit | fast | v1.1 | 408 | 4 | clauses | sampled | |
| 32bit | small | v2.0 | 129 | 65 | sum-carry | sampled | |
| 32bit | efficient | v1.1 | 261 | 5 | factors | sampled | |

Gates and ticks do not count the input gates (see the main README). Up to 8 bit every input was tried. From 16 bit on they were checked on about a million inputs, with the
adder's carry chains and the subtractor's borrow chains among them. None has been tested in the game yet.

## Generator

```
cargo build --release
target/release/pareto designs adder-subtractor 8bit       # every design, measured
target/release/pareto build-all adder-subtractor --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`.

Subtracting is adding the inverse of b plus one. So b goes through one XOR gate per bit with the mode, and the
mode is the carry into the lowest bit. That costs one gate per bit and one tick over a plain adder.

`sum-carry`, `clauses`, `factors` and `within-T` are the designs from the adder's search
(`tools/evolve/tasks/adder`) with the mode pin added, by `mode_designs` in the adder generator. The carry in
comes from an adder one bit longer whose lowest bit has the mode on both inputs.

`ripple` lets the carry walk from bit to bit, `lookahead` gives every bit its carry in one step, `blocks-k`
does that inside blocks of k bits, and `two-level-k` adds a second lookahead across the blocks. They are the
adder family's carry networks; see `circuits/adder/README.md`.
