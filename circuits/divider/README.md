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
| 2bit | fast | v1.0 | 11 | 2 | normal-form | exhaustive | |
| 2bit | small | v1.0 | 11 | 2 | normal-form | exhaustive | |
| 2bit | efficient | v1.0 | 11 | 2 | normal-form | exhaustive | |
| 4bit | fast | v1.0 | 149 | 2 | normal-form | exhaustive | |
| 4bit | small | v2.0 | 48 | 27 | ripple | exhaustive | |
| 4bit | efficient | v1.0 | 149 | 2 | normal-form | exhaustive | |
| 8bit | fast | v2.0 | 265 | 29 | lookahead | exhaustive | |
| 8bit | small | v2.0 | 160 | 83 | ripple | exhaustive | |
| 8bit | efficient | v3.0 | 265 | 29 | lookahead | exhaustive | |
| 16bit | fast | v2.0 | 1361 | 60 | lookahead | exhaustive | |
| 16bit | small | v2.0 | 576 | 291 | ripple | exhaustive | |
| 16bit | efficient | v2.0 | 1117 | 70 | blocks-8 | exhaustive | |
| 32bit | fast | v2.0 | 8166 | 123 | lookahead | sampled | |
| 32bit | small | v2.0 | 2176 | 1091 | ripple | sampled | |
| 32bit | efficient | v2.0 | 4951 | 163 | two-level-8 | sampled | |

Gates and ticks do not count the input gates (see the main README). Up to 16 bit every input was tried. 32 bit was checked on about a million inputs: random ones, divisors of
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

Taking b away flips some bits of what is left. So a bit of what is left is never joined into one signal: it
is handed on as the list of signals whose XOR it is (the bit of a, and one "this bit flips" per row so far),
and the XOR gates of the next row read the whole list. Only the remainder outputs join them.

A row is 4 gates per bit with a walking borrow: "this bit changes if we subtract" (b xor the borrow in),
"what is left equals b here", the AND of those two, and the flip (an AND with "we do subtract"). The borrow
out is b xor that AND, and is handed on as those two signals.

| Design | How a row finds out whether it can subtract |
|---|---|
| `ripple` | The borrow walks along the row. Fewest gates, and the ticks grow with n x n. |
| `lookahead`, `blocks-k`, `two-level-k` | One of the adder family's carry networks. The flip is one NOR gate over the borrow's terms, so a row is 4 ticks or a little more |
| `normal-form` | No rows: every output is one XOR gate over AND gates of plain inputs (up to 4bit) |
