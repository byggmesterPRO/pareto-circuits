# Adder

Adds two binary numbers. Size is the number of bits of each number: `8bit` adds two 8-bit numbers and gives an
8-bit sum plus a carry.

There is no carry input.

## Pins

- **Inputs:** the yellow gates in the first row. x = 0 .. n-1 is the first number, x = n .. 2n-1 the second,
  least significant bit first in each.
- **Outputs:** the green gates in the second row. x = 0 .. n-1 is the sum, least significant bit first, and
  x = n is the carry out.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 10 | 4 | ripple | exhaustive | |
| 2bit | small | v1.0 | 10 | 4 | ripple | exhaustive | |
| 2bit | balanced | v1.0 | 10 | 4 | ripple | exhaustive | |
| 4bit | fast | v1.0 | 26 | 4 | lookahead | exhaustive | |
| 4bit | small | v1.0 | 22 | 8 | ripple | exhaustive | |
| 4bit | balanced | v1.0 | 26 | 4 | lookahead | exhaustive | |
| 8bit | fast | v1.0 | 68 | 4 | lookahead | exhaustive | |
| 8bit | small | v1.0 | 46 | 16 | ripple | exhaustive | |
| 8bit | balanced | v1.0 | 68 | 4 | lookahead | exhaustive | |
| 16bit | fast | v1.0 | 200 | 4 | lookahead | exhaustive | |
| 16bit | small | v1.0 | 94 | 32 | ripple | exhaustive | |
| 16bit | balanced | v1.0 | 200 | 4 | lookahead | exhaustive | |
| 32bit | fast | v1.0 | 659 | 4 | lookahead | sampled | |
| 32bit | small | v1.0 | 190 | 64 | ripple | sampled | |
| 32bit | balanced | v1.0 | 327 | 6 | two-level-8 | sampled | |
| 64bit | fast | v1.0 | 2474 | 4 | lookahead | sampled | |
| 64bit | small | v1.0 | 382 | 128 | ripple | sampled | |
| 64bit | balanced | v1.0 | 783 | 6 | two-level-8 | sampled | |

Gates include the input gates. Up to 16 bit every input was tried (4.3 billion at 16 bit). At 32 and 64 bit
there are too many, so they were checked on about a million inputs: random ones, edge cases, and every carry
chain from any bit to any higher bit. None has been tested in the game yet.

## Generator

```
cargo build --release
target/release/pareto designs adder 16bit       # every design, measured
target/release/pareto build-all adder --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`. It offers several designs per size, and the build tool gives each
profile to whichever design wins when measured.

| Design | Idea | Ticks | Gates |
|---|---|---|---|
| `ripple` | One 4-gate full adder per bit, the carry walking up | 2 per bit | about 4 per bit |
| `lookahead` | Every sum bit sees its carry in one step | 4 at any size | about n x n / 2 |
| `blocks-k` | Lookahead inside blocks of k bits, the carry walking from block to block | 2 per block | in between |
| `two-level-k` | Blocks of k bits, and a second lookahead across the blocks | 6 at any size | far fewer than `lookahead` |

The trick behind all but `ripple`: a carry reaches a bit from exactly one place, the nearest lower bit that
starts one. So the terms for "a carry arrives from bit j" are never on two at a time, their OR is the same as
their XOR, and a sum bit is a single XOR gate over those terms. No gate is spent on the carry itself.

The full adder in `ripple` is 4 gates, which exact search showed is the fewest for one bit taken alone.
Where a gate would drive more than 255 others (the 32 and 64 bit `lookahead`), it is doubled.
