# Divider

Divides one binary number by another (unsigned). Size is the number of bits of each number. Dividing by 0
gives a quotient of all ones and a as the remainder.

## Pins

- **Inputs:** the yellow gates in the first row. x = 0 .. n-1 is a, x = n .. 2n-1 is b, least significant bit
  first in each.
- **Outputs:** the green gates in the second row. x = 0 .. n-1 is the quotient, x = n .. 2n-1 the remainder.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 15 | 3 | normal-form | exhaustive | |
| 2bit | small | v1.0 | 15 | 3 | normal-form | exhaustive | |
| 2bit | efficient | v1.0 | 15 | 3 | normal-form | exhaustive | |
| 4bit | fast | v1.0 | 157 | 3 | normal-form | exhaustive | |
| 4bit | small | v1.0 | 64 | 27 | ripple | exhaustive | |
| 4bit | efficient | v1.0 | 157 | 3 | normal-form | exhaustive | |
| 8bit | fast | v1.0 | 315 | 48 | lookahead | exhaustive | |
| 8bit | small | v1.0 | 224 | 83 | ripple | exhaustive | |
| 8bit | efficient | v1.0 | 315 | 48 | lookahead | exhaustive | |
| 16bit | fast | v1.0 | 1527 | 96 | lookahead | exhaustive | |
| 16bit | small | v1.0 | 832 | 291 | ripple | exhaustive | |
| 16bit | efficient | v1.0 | 1197 | 108 | two-level-4 | exhaustive | |
| 32bit | fast | v1.0 | 8717 | 192 | lookahead | sampled | |
| 32bit | small | v1.0 | 3200 | 1091 | ripple | sampled | |
| 32bit | efficient | v1.0 | 5369 | 220 | two-level-4 | sampled | |

Gates include the input gates. Up to 16 bit every input was tried. 32 bit was checked on about a million inputs: random ones, divisors of
every length, and numbers just below, at and just above a multiple of the divisor. None has been tested in the game yet.

## Generator

```
cargo build --release
target/release/pareto designs divider 8bit       # every design, measured
target/release/pareto build-all divider --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`.

Long division, one row per quotient bit from the top: bring down the next bit of a, and if what is left is at
least b, take b away. What is left after row k fits in k + 1 bits, so the rows start narrow.

A row is 6 gates per bit with a walking borrow: 3 for the borrow (the fewest, by exact search), one for "this
bit changes if we subtract", one AND with "we do subtract", and one XOR.

| Design | How a row finds out whether it can subtract |
|---|---|
| `ripple` | The borrow walks along the row. Fewest gates, and the ticks grow with n x n. |
| `lookahead`, `blocks-k`, `two-level-k` | One of the adder family's carry networks: 6 ticks per row or a little more |
| `normal-form` | No rows: every output is one XOR gate over AND gates of plain inputs (up to 4bit) |
