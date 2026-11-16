# Square

Multiplies a binary number by itself. Size is the number of bits: `8bit` gives a 16-bit square.

## Pins

- **Inputs:** the yellow gates in the first row, x = 0 .. n-1, least significant bit first.
- **Outputs:** the green gates in the second row, x = 0 .. 2n-1.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 2 | 2 | normal-form | exhaustive | |
| 2bit | small | v1.0 | 2 | 2 | normal-form | exhaustive | |
| 2bit | efficient | v1.0 | 2 | 2 | normal-form | exhaustive | |
| 4bit | fast | v1.0 | 14 | 2 | normal-form | exhaustive | |
| 4bit | small | v1.0 | 14 | 2 | normal-form | exhaustive | |
| 4bit | efficient | v1.0 | 14 | 2 | normal-form | exhaustive | |
| 8bit | fast | v1.0 | 174 | 2 | normal-form | exhaustive | |
| 8bit | small | v2.0 | 108 | 26 | parts-ripple | exhaustive | |
| 8bit | efficient | v1.0 | 174 | 2 | normal-form | exhaustive | |
| 16bit | fast | v1.0 | 1238 | 10 | wide-blocks-16 | exhaustive | |
| 16bit | small | v2.0 | 472 | 60 | parts-ripple | exhaustive | |
| 16bit | efficient | v2.0 | 651 | 12 | tree+factors | exhaustive | |
| 32bit | fast | v1.1 | 6708 | 12 | wide+clauses | exhaustive | |
| 32bit | small | v2.0 | 1968 | 128 | parts-ripple | exhaustive | |
| 32bit | efficient | v2.0 | 2663 | 16 | tree+factors | exhaustive | |

Gates and ticks do not count the input gates (see the main README). Every input was tried at every size (4.3 billion at 32 bit). None has been tested in the game yet.

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
