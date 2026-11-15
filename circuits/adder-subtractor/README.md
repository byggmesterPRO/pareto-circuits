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
| 2bit | small | v1.0 | 11 | 6 | ripple | exhaustive | |
| 2bit | efficient | v1.0 | 13 | 4 | lookahead | exhaustive | |
| 4bit | fast | v1.0 | 28 | 4 | lookahead | exhaustive | |
| 4bit | small | v1.0 | 21 | 10 | ripple | exhaustive | |
| 4bit | efficient | v1.0 | 28 | 4 | lookahead | exhaustive | |
| 8bit | fast | v1.0 | 70 | 4 | lookahead | exhaustive | |
| 8bit | small | v1.0 | 41 | 18 | ripple | exhaustive | |
| 8bit | efficient | v1.0 | 54 | 5 | two-level-4 | exhaustive | |
| 16bit | fast | v1.0 | 202 | 4 | lookahead | sampled | |
| 16bit | small | v1.0 | 81 | 34 | ripple | sampled | |
| 16bit | efficient | v1.0 | 124 | 5 | two-level-4 | sampled | |
| 32bit | fast | v1.0 | 666 | 4 | lookahead | sampled | |
| 32bit | small | v1.0 | 161 | 66 | ripple | sampled | |
| 32bit | efficient | v1.0 | 300 | 5 | two-level-8 | sampled | |

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

`ripple` lets the carry walk from bit to bit, `lookahead` gives every bit its carry in one step, `blocks-k`
does that inside blocks of k bits, and `two-level-k` adds a second lookahead across the blocks. They are the
adder family's carry networks; see `circuits/adder/README.md`.
