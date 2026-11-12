# Multiplexer

Sends one of n data inputs to the output. Size is the number of data lines: `16bit` has 16 lines and a 4-bit
line number.

## Pins

- **Inputs:** the yellow gates in the first row. x = 0 .. n-1 are the data lines, then the line number,
  least significant bit first.
- **Output:** the green gate at x = 0 in the second row.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 6 | 3 | no-inverters | exhaustive | |
| 2bit | small | v1.0 | 6 | 3 | no-inverters | exhaustive | |
| 2bit | efficient | v1.0 | 6 | 3 | no-inverters | exhaustive | |
| 4bit | fast | v1.0 | 15 | 3 | no-inverters | exhaustive | |
| 4bit | small | v1.0 | 13 | 4 | flat | exhaustive | |
| 4bit | efficient | v1.0 | 15 | 3 | no-inverters | exhaustive | |
| 8bit | fast | v1.0 | 38 | 3 | no-inverters | exhaustive | |
| 8bit | small | v1.0 | 23 | 4 | flat | exhaustive | |
| 8bit | efficient | v1.0 | 23 | 4 | flat | exhaustive | |
| 16bit | fast | v1.0 | 101 | 3 | no-inverters | exhaustive | |
| 16bit | small | v1.0 | 41 | 4 | flat | exhaustive | |
| 16bit | efficient | v1.0 | 41 | 4 | flat | exhaustive | |
| 32bit | fast | v1.0 | 280 | 3 | no-inverters | sampled | |
| 32bit | small | v1.0 | 75 | 4 | flat | sampled | |
| 32bit | efficient | v1.0 | 75 | 4 | flat | sampled | |
| 64bit | fast | v1.0 | 141 | 4 | flat | sampled | |
| 64bit | small | v1.0 | 141 | 4 | flat | sampled | |
| 64bit | efficient | v1.0 | 141 | 4 | flat | sampled | |

Gates include the input gates. Up to 16 lines every input was tried. 32 and 64 lines were checked on about a million inputs: random ones,
and for every line number all data off, all on, only that line on, and only that line off. None has been tested in the game yet.

## Generator

```
cargo build --release
target/release/pareto designs multiplexer 8bit       # every design, measured
target/release/pareto build-all multiplexer --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`.

| Design | Idea | Ticks | Gates |
|---|---|---|---|
| `flat` | One AND gate per data line over the line and the line number bits or their inverses, and an OR | 4 | n + log n + 1 |
| `no-inverters` | "d and not s" is d xor (d and s), so the whole thing is one XOR gate over AND gates of plain inputs | 3 | 3^(log n) |

`no-inverters` stops at 32 lines: at 64 the XOR gate would need 729 inputs, and a gate takes 255.
