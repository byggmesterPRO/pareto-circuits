# Incrementer

Adds 1 to a binary number. Size is the number of bits.

## Pins

- **Inputs:** the yellow gates in the first row, x = 0 .. n-1, least significant bit first.
- **Outputs:** the green gates in the second row. x = 0 .. n-1 is the number plus 1, x = n is the carry (on
  when every input bit was 1).

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 3 | 1 | flat | exhaustive | |
| 2bit | small | v1.0 | 3 | 1 | flat | exhaustive | |
| 2bit | efficient | v1.0 | 3 | 1 | flat | exhaustive | |
| 4bit | fast | v1.0 | 7 | 2 | flat | exhaustive | |
| 4bit | small | v1.0 | 7 | 2 | flat | exhaustive | |
| 4bit | efficient | v1.0 | 7 | 2 | flat | exhaustive | |
| 8bit | fast | v1.0 | 15 | 2 | flat | exhaustive | |
| 8bit | small | v1.0 | 15 | 2 | flat | exhaustive | |
| 8bit | efficient | v1.0 | 15 | 2 | flat | exhaustive | |
| 16bit | fast | v1.0 | 31 | 2 | flat | exhaustive | |
| 16bit | small | v1.0 | 31 | 2 | flat | exhaustive | |
| 16bit | efficient | v1.0 | 31 | 2 | flat | exhaustive | |
| 32bit | fast | v1.0 | 63 | 2 | flat | exhaustive | |
| 32bit | small | v1.0 | 63 | 2 | flat | exhaustive | |
| 32bit | efficient | v1.0 | 63 | 2 | flat | exhaustive | |
| 64bit | fast | v1.0 | 127 | 2 | flat | sampled | |
| 64bit | small | v1.0 | 127 | 2 | flat | sampled | |
| 64bit | efficient | v1.0 | 127 | 2 | flat | sampled | |

Gates and ticks do not count the input gates (see the main README). Up to 32 bit every input was tried. 64 bit was checked on about a million inputs, with runs of ones of
every length at the bottom among them. None has been tested in the game yet.

## Generator

```
cargo build --release
target/release/pareto designs incrementer 8bit       # every design, measured
target/release/pareto build-all incrementer --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`.

One design, `flat`: bit i flips when every bit below it is 1, which is one AND gate over the lower bits and one
XOR gate. All bits at once, so 2 ticks at any size and about 2 gates per bit.
