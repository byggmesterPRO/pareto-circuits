# Negate

Turns a number into its negative, in two's complement. Size is the number of bits. 0 stays 0, and the most
negative number stays itself.

## Pins

- **Inputs:** the yellow gates in the first row, x = 0 .. n-1, least significant bit first.
- **Outputs:** the green gates in the second row, x = 0 .. n-1.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 4 | 2 | flat | exhaustive | |
| 2bit | small | v1.0 | 4 | 2 | flat | exhaustive | |
| 2bit | balanced | v1.0 | 4 | 2 | flat | exhaustive | |
| 4bit | fast | v1.0 | 10 | 3 | flat | exhaustive | |
| 4bit | small | v1.0 | 10 | 3 | flat | exhaustive | |
| 4bit | balanced | v1.0 | 10 | 3 | flat | exhaustive | |
| 8bit | fast | v1.0 | 22 | 3 | flat | exhaustive | |
| 8bit | small | v1.0 | 22 | 3 | flat | exhaustive | |
| 8bit | balanced | v1.0 | 22 | 3 | flat | exhaustive | |
| 16bit | fast | v1.0 | 46 | 3 | flat | exhaustive | |
| 16bit | small | v1.0 | 46 | 3 | flat | exhaustive | |
| 16bit | balanced | v1.0 | 46 | 3 | flat | exhaustive | |
| 32bit | fast | v1.0 | 94 | 3 | flat | exhaustive | |
| 32bit | small | v1.0 | 94 | 3 | flat | exhaustive | |
| 32bit | balanced | v1.0 | 94 | 3 | flat | exhaustive | |
| 64bit | fast | v1.0 | 190 | 3 | flat | sampled | |
| 64bit | small | v1.0 | 190 | 3 | flat | sampled | |
| 64bit | balanced | v1.0 | 190 | 3 | flat | sampled | |

Gates include the input gates. Up to 32 bit every input was tried. 64 bit was checked on about a million inputs. None has been tested in the game yet.

## Generator

```
cargo build --release
target/release/pareto designs negate 8bit       # every design, measured
target/release/pareto build-all negate --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`.

One design, `flat`: negating keeps every bit up to and including the lowest 1 and flips the ones above it. So
bit i flips when any bit below it is 1: one OR gate over the lower bits and one XOR gate. 3 ticks at any size.

Bit 0 never changes. It goes through a gate that only copies it, so that every output has a gate of its own.
