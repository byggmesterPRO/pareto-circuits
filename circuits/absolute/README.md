# Absolute value

Drops the sign of a two's complement number. Size is the number of bits; the top bit is the sign. The result
is read as an unsigned number, so the most negative number comes out right.

## Pins

- **Inputs:** the yellow gates in the first row, x = 0 .. n-1, least significant bit first.
- **Outputs:** the green gates in the second row, x = 0 .. n-1.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 4 | 3 | normal-form | exhaustive | |
| 2bit | small | v1.0 | 4 | 3 | normal-form | exhaustive | |
| 2bit | efficient | v1.0 | 4 | 3 | normal-form | exhaustive | |
| 4bit | fast | v1.0 | 14 | 3 | normal-form | exhaustive | |
| 4bit | small | v1.0 | 13 | 4 | flat | exhaustive | |
| 4bit | efficient | v1.0 | 14 | 3 | normal-form | exhaustive | |
| 8bit | fast | v1.0 | 142 | 3 | normal-form | exhaustive | |
| 8bit | small | v1.0 | 29 | 4 | flat | exhaustive | |
| 8bit | efficient | v1.0 | 29 | 4 | flat | exhaustive | |
| 16bit | fast | v1.0 | 61 | 4 | flat | exhaustive | |
| 16bit | small | v1.0 | 61 | 4 | flat | exhaustive | |
| 16bit | efficient | v1.0 | 61 | 4 | flat | exhaustive | |
| 32bit | fast | v1.0 | 125 | 4 | flat | exhaustive | |
| 32bit | small | v1.0 | 125 | 4 | flat | exhaustive | |
| 32bit | efficient | v1.0 | 125 | 4 | flat | exhaustive | |
| 64bit | fast | v1.0 | 253 | 4 | flat | sampled | |
| 64bit | small | v1.0 | 253 | 4 | flat | sampled | |
| 64bit | efficient | v1.0 | 253 | 4 | flat | sampled | |

Gates include the input gates. Up to 32 bit every input was tried. 64 bit was checked on about a million inputs. None has been tested in the game yet.

## Generator

```
cargo build --release
target/release/pareto designs absolute 8bit       # every design, measured
target/release/pareto build-all absolute --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`.

| Design | Idea | Ticks |
|---|---|---|
| `flat` | A bit flips when the sign is on and any lower bit is 1: an OR, an AND and an XOR gate per bit | 4 |
| `normal-form` | Every output is one XOR gate over AND gates of plain inputs (up to 8bit) | 3 |
