# Subtractor

Takes one binary number from another. Size is the number of bits of each number: `8bit` gives the 8-bit
difference a - b plus a borrow.

If b is larger than a the difference wraps around (as two's complement) and the borrow is on. There is no
borrow input.

## Pins

- **Inputs:** the yellow gates in the first row. x = 0 .. n-1 is a, x = n .. 2n-1 is b, least significant bit
  first in each.
- **Outputs:** the green gates in the second row. x = 0 .. n-1 is the difference, least significant bit first,
  and x = n is the borrow (on when a < b).

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 13 | 3 | normal-form | exhaustive | |
| 2bit | small | v1.0 | 10 | 5 | ripple | exhaustive | |
| 2bit | balanced | v1.0 | 13 | 3 | normal-form | exhaustive | |
| 4bit | fast | v1.0 | 89 | 3 | normal-form | exhaustive | |
| 4bit | small | v1.0 | 22 | 9 | ripple | exhaustive | |
| 4bit | balanced | v1.0 | 31 | 4 | lookahead | exhaustive | |
| 8bit | fast | v1.0 | 77 | 4 | lookahead | exhaustive | |
| 8bit | small | v1.0 | 46 | 17 | ripple | exhaustive | |
| 8bit | balanced | v1.0 | 77 | 4 | lookahead | exhaustive | |
| 16bit | fast | v1.0 | 217 | 4 | lookahead | exhaustive | |
| 16bit | small | v1.0 | 94 | 33 | ripple | exhaustive | |
| 16bit | balanced | v1.0 | 217 | 4 | lookahead | exhaustive | |
| 32bit | fast | v1.0 | 692 | 4 | lookahead | sampled | |
| 32bit | small | v1.0 | 190 | 65 | ripple | sampled | |
| 32bit | balanced | v1.0 | 360 | 6 | two-level-8 | sampled | |
| 64bit | fast | v1.0 | 2539 | 4 | lookahead | sampled | |
| 64bit | small | v1.0 | 382 | 129 | ripple | sampled | |
| 64bit | balanced | v1.0 | 848 | 6 | two-level-8 | sampled | |

Gates include the input gates. Up to 16 bit every input was tried. At 32 and 64 bit there are too many, so they were checked on about a
million inputs: random ones, edge cases, and borrow chains from any bit to any higher bit. None has been tested in the game yet.

## Generator

```
cargo build --release
target/release/pareto designs subtractor 8bit       # every design, measured
target/release/pareto build-all subtractor --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`.

| Design | Idea | Ticks | Gates |
|---|---|---|---|
| `ripple` | One 4-gate full subtractor per bit, the borrow walking up | 2 per bit | about 4 per bit |
| `lookahead` | Every bit sees its borrow in one step | 4 at any size | about n x n / 2 |
| `blocks-k` | Lookahead inside blocks of k bits, the borrow walking from block to block | 2 per block | in between |
| `two-level-k` | Blocks of k bits, and a second lookahead across the blocks | 6 at any size | far fewer than `lookahead` |
| `normal-form` | Every output is one XOR gate over AND gates of plain inputs (up to 4bit) | 3 | many |

A borrow works like the adder's carry: it reaches a bit from exactly one place, so a difference bit is a single
gate over the terms for "a borrow arrives from bit j". The full subtractor in `ripple` is 4 gates, which exact
search showed is the fewest for one bit taken alone.
