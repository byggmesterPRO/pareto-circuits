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
| 2bit | fast | v1.0 | 9 | 2 | normal-form | exhaustive | |
| 2bit | small | v1.0 | 6 | 4 | ripple | exhaustive | |
| 2bit | efficient | v1.0 | 9 | 2 | normal-form | exhaustive | |
| 4bit | fast | v1.0 | 81 | 2 | normal-form | exhaustive | |
| 4bit | small | v1.1 | 12 | 8 | sum-carry | exhaustive | |
| 4bit | efficient | v1.1 | 20 | 3 | clauses | exhaustive | |
| 8bit | fast | v1.1 | 58 | 3 | within-3 | exhaustive | |
| 8bit | small | v1.1 | 24 | 16 | sum-carry | exhaustive | |
| 8bit | efficient | v1.1 | 58 | 3 | within-3 | exhaustive | |
| 16bit | fast | v1.1 | 182 | 3 | within-3 | exhaustive | |
| 16bit | small | v1.1 | 48 | 32 | sum-carry | exhaustive | |
| 16bit | efficient | v2.0 | 104 | 5 | within-5 | exhaustive | |
| 32bit | fast | v1.1 | 623 | 3 | within-3 | sampled | |
| 32bit | small | v1.1 | 96 | 64 | sum-carry | sampled | |
| 32bit | efficient | v1.1 | 250 | 5 | factors | sampled | |
| 64bit | fast | v1.1 | 2406 | 3 | within-3 | sampled | |
| 64bit | small | v1.1 | 192 | 128 | sum-carry | sampled | |
| 64bit | efficient | v1.1 | 580 | 5 | factors | sampled | |

Gates and ticks do not count the input gates (see the main README). Up to 16 bit every input was tried. At 32 and 64 bit there are too many, so they were checked on about a
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
| `lookahead` | Every bit sees its borrow in one step | 3 at any size | about n x n / 2 |
| `blocks-k` | Lookahead inside blocks of k bits, the borrow walking from block to block | 2 per block | in between |
| `two-level-k` | Blocks of k bits, and a second lookahead across the blocks | 5 at any size | far fewer than `lookahead` |
| `normal-form` | Every output is one XOR gate over AND gates of plain inputs (up to 4bit) | 2 | many |
| `sum-carry` | The borrow out of a bit is read back from its difference bit | 2 per bit | 3 per bit |
| `clauses`, `factors` | The adder's fast designs, turned into subtractors | 4 and 5 | far fewer than `lookahead` |
| `within-T` | Planned for a limit of T ticks: lookahead where the limit needs it, `sum-carry` steps elsewhere | T | in between |

The last three rows come from the adder's search (`tools/evolve/tasks/adder`). a - b is the inverse of
"not a" + b, so an adder becomes a subtractor by reading a inverted and giving its sum bits inverted; the code
is `minus.rs` in the adder generator. That costs about one gate per bit and sometimes one tick, which is why
the subtractor's `clauses` takes 4 ticks where the adder's takes 3.

A borrow works like the adder's carry: it reaches a bit from exactly one place, so a difference bit is a single
gate over the terms for "a borrow arrives from bit j". The full subtractor in `ripple` is 4 gates, which exact
search showed is the fewest for one bit taken alone.
