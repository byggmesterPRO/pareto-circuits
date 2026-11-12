# Square

Multiplies a binary number by itself. Size is the number of bits: `8bit` gives a 16-bit square.

## Pins

- **Inputs:** the yellow gates in the first row, x = 0 .. n-1, least significant bit first.
- **Outputs:** the green gates in the second row, x = 0 .. 2n-1.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 4 | 3 | normal-form | exhaustive | |
| 2bit | small | v1.0 | 4 | 3 | normal-form | exhaustive | |
| 2bit | efficient | v1.0 | 4 | 3 | normal-form | exhaustive | |
| 4bit | fast | v1.0 | 18 | 3 | normal-form | exhaustive | |
| 4bit | small | v1.0 | 18 | 3 | normal-form | exhaustive | |
| 4bit | efficient | v1.0 | 18 | 3 | normal-form | exhaustive | |
| 8bit | fast | v1.0 | 182 | 3 | normal-form | exhaustive | |
| 8bit | small | v1.0 | 134 | 24 | ripple | exhaustive | |
| 8bit | efficient | v1.0 | 182 | 3 | normal-form | exhaustive | |
| 16bit | fast | v1.0 | 1254 | 11 | wide-blocks-16 | exhaustive | |
| 16bit | small | v1.0 | 586 | 56 | ripple | exhaustive | |
| 16bit | efficient | v1.0 | 697 | 14 | tree-two-level-8 | exhaustive | |
| 32bit | fast | v1.0 | 7845 | 13 | wide-lookahead | exhaustive | |
| 32bit | small | v1.0 | 2450 | 120 | ripple | exhaustive | |
| 32bit | efficient | v1.0 | 2821 | 18 | tree-two-level-8 | exhaustive | |

Gates include the input gates. Every input was tried at every size (4.3 billion at 32 bit). None has been tested in the game yet.

## Generator

```
cargo build --release
target/release/pareto designs square 8bit       # every design, measured
target/release/pareto build-all square --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`.

A multiplier with half the work: bit i with bit j shows up twice in the grid of products, and twice a bit is
the same bit one column up. So there is one AND gate per pair, and the columns are added up the way the
multiplier does it. The designs have the multiplier's names; see `circuits/multiplier/README.md`.
