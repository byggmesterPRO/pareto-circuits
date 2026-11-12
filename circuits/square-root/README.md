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
| 2bit | fast | v1.0 | 4 | 3 | normal-form | exhaustive | |
| 2bit | small | v1.0 | 4 | 3 | normal-form | exhaustive | |
| 2bit | efficient | v1.0 | 4 | 3 | normal-form | exhaustive | |
| 4bit | fast | v1.0 | 11 | 3 | normal-form | exhaustive | |
| 4bit | small | v1.0 | 11 | 3 | normal-form | exhaustive | |
| 4bit | efficient | v1.0 | 11 | 3 | normal-form | exhaustive | |
| 8bit | fast | v1.0 | 132 | 3 | normal-form | exhaustive | |
| 8bit | small | v1.0 | 66 | 29 | ripple | exhaustive | |
| 8bit | efficient | v1.0 | 132 | 3 | normal-form | exhaustive | |
| 16bit | fast | v1.0 | 286 | 46 | lookahead | exhaustive | |
| 16bit | small | v1.0 | 230 | 89 | ripple | exhaustive | |
| 16bit | efficient | v1.0 | 286 | 46 | lookahead | exhaustive | |
| 32bit | fast | v1.0 | 1406 | 94 | lookahead | exhaustive | |
| 32bit | small | v1.0 | 846 | 305 | ripple | exhaustive | |
| 32bit | efficient | v1.0 | 1212 | 104 | two-level-8 | exhaustive | |
| 64bit | fast | v1.0 | 8201 | 190 | lookahead | sampled | |
| 64bit | small | v1.0 | 3230 | 1121 | ripple | sampled | |
| 64bit | efficient | v1.0 | 5294 | 216 | two-level-8 | sampled | |

Gates include the input gates. Up to 32 bit every input was tried. 64 bit was checked on about a million inputs: random ones, perfect
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
