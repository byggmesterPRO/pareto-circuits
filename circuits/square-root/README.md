# Square root

The square root of a binary number, rounded down. Size is the number of bits of the number, which is even;
the root has half as many.

## Pins

- **Inputs:** the yellow gates in the first row, x = 0 .. n-1, least significant bit first.
- **Outputs:** the green gates in the second row, x = 0 .. n/2 - 1.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 2 | 2 | normal-form | exhaustive | |
| 2bit | small | v1.0 | 2 | 2 | normal-form | exhaustive | |
| 2bit | efficient | v1.0 | 2 | 2 | normal-form | exhaustive | |
| 4bit | fast | v1.0 | 7 | 2 | normal-form | exhaustive | |
| 4bit | small | v1.0 | 7 | 2 | normal-form | exhaustive | |
| 4bit | efficient | v1.0 | 7 | 2 | normal-form | exhaustive | |
| 8bit | fast | v1.0 | 124 | 2 | normal-form | exhaustive | |
| 8bit | small | v2.0 | 46 | 24 | ripple | exhaustive | |
| 8bit | efficient | v1.0 | 124 | 2 | normal-form | exhaustive | |
| 16bit | fast | v2.0 | 228 | 27 | lookahead | exhaustive | |
| 16bit | small | v2.0 | 158 | 80 | ripple | exhaustive | |
| 16bit | efficient | v2.0 | 228 | 27 | lookahead | exhaustive | |
| 32bit | fast | v2.0 | 1224 | 58 | lookahead | exhaustive | |
| 32bit | small | v2.0 | 574 | 288 | ripple | exhaustive | |
| 32bit | efficient | v2.0 | 1224 | 58 | lookahead | exhaustive | |
| 64bit | fast | v2.0 | 7580 | 120 | lookahead | sampled | |
| 64bit | small | v2.0 | 2174 | 1088 | ripple | sampled | |
| 64bit | efficient | v2.0 | 4703 | 164 | two-level-8 | sampled | |

Gates and ticks do not count the input gates (see the main README). Up to 32 bit every input was tried. 64 bit was checked on about a million inputs: random ones, perfect
squares, and the numbers just below one and just below the next. None has been tested in the game yet.

## Generator

```
cargo build --release
target/release/pareto designs square-root 8bit       # every design, measured
target/release/pareto build-all square-root --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`.

The pen and paper method in binary, one row per root bit from the top: bring down the next two bits, and if
what is left is at least (the root so far, then 01), take that away and the root bit is 1. A row is the same
"subtract if you can" row as in the divider, and the designs have the same names.
