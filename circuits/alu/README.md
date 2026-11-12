# ALU

An arithmetic and logic unit: one of eight operations on two binary numbers, picked by a 3-bit code. Size is
the number of bits of each number.

| Code | Result | Flag |
|---|---|---|
| 0 | a + b | carry |
| 1 | a - b | borrow (a < b) |
| 2 | a and b | |
| 3 | a or b | |
| 4 | a xor b | |
| 5 | a shifted up one place | the bit that fell off the top |
| 6 | a shifted down one place | the bit that fell off the bottom |
| 7 | not a | |

## Pins

- **Inputs:** the yellow gates in the first row. x = 0 .. n-1 is a, x = n .. 2n-1 is b, least significant bit
  first in each, and x = 2n .. 2n+2 is the code.
- **Outputs:** the green gates in the second row. x = 0 .. n-1 is the result, x = n is the flag.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 54 | 7 | lookahead | exhaustive | |
| 2bit | small | v1.0 | 52 | 9 | ripple | exhaustive | |
| 2bit | efficient | v1.0 | 54 | 7 | lookahead | exhaustive | |
| 4bit | fast | v1.0 | 95 | 7 | lookahead | exhaustive | |
| 4bit | small | v1.0 | 88 | 13 | ripple | exhaustive | |
| 4bit | efficient | v1.0 | 95 | 7 | lookahead | exhaustive | |
| 8bit | fast | v1.0 | 189 | 7 | lookahead | exhaustive | |
| 8bit | small | v1.0 | 160 | 21 | ripple | exhaustive | |
| 8bit | efficient | v1.0 | 189 | 7 | lookahead | exhaustive | |
| 16bit | fast | v1.0 | 425 | 7 | lookahead | sampled | |
| 16bit | small | v1.0 | 304 | 37 | ripple | sampled | |
| 16bit | efficient | v1.0 | 347 | 8 | two-level-4 | sampled | |
| 32bit | fast | v1.0 | 1097 | 7 | lookahead | sampled | |
| 32bit | small | v1.0 | 592 | 69 | ripple | sampled | |
| 32bit | efficient | v1.0 | 731 | 8 | two-level-8 | sampled | |

Gates include the input gates. Up to 8 bit every input was tried. 16 and 32 bit were checked on about a million inputs, with carry and
borrow chains and 2000 random pairs for every operation among them. None has been tested in the game yet.

## Generator

```
cargo build --release
target/release/pareto designs alu 8bit       # every design, measured
target/release/pareto build-all alu --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`.

Add and subtract share one adder-subtractor, with the lowest code bit as its mode. Every operation has a line
that is on when it is picked, and each result bit is an OR over one AND gate per operation.

`ripple` lets the carry walk from bit to bit, `lookahead` gives every bit its carry in one step, `blocks-k`
does that inside blocks of k bits, and `two-level-k` adds a second lookahead across the blocks. They are the
adder family's carry networks; see `circuits/adder/README.md`.
