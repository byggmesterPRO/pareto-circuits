# Barrel shifter

Moves a binary number up by a chosen number of places (a logical shift left). Size is the width of the number:
`8bit` shifts an 8-bit number by 0 to 7 places. Bits that move past the top are lost, zeros come in at the
bottom.

## Pins

- **Inputs:** the yellow gates in the first row. x = 0 .. n-1 is the number, then the shift amount (log2 n
  bits), least significant bit first in each.
- **Outputs:** the green gates in the second row, x = 0 .. n-1, least significant bit first.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 7 | 3 | normal-form | exhaustive | |
| 2bit | small | v1.0 | 7 | 3 | normal-form | exhaustive | |
| 2bit | balanced | v1.0 | 7 | 3 | normal-form | exhaustive | |
| 4bit | fast | v1.0 | 22 | 3 | normal-form | exhaustive | |
| 4bit | small | v1.0 | 21 | 4 | flat | exhaustive | |
| 4bit | balanced | v1.0 | 22 | 3 | normal-form | exhaustive | |
| 8bit | fast | v1.0 | 75 | 3 | normal-form | exhaustive | |
| 8bit | small | v1.0 | 57 | 4 | flat | exhaustive | |
| 8bit | balanced | v1.0 | 75 | 3 | normal-form | exhaustive | |
| 16bit | fast | v1.0 | 175 | 4 | flat | exhaustive | |
| 16bit | small | v1.0 | 149 | 6 | stages-2 | exhaustive | |
| 16bit | balanced | v1.0 | 175 | 4 | flat | exhaustive | |
| 32bit | fast | v1.0 | 608 | 4 | flat | sampled | |
| 32bit | small | v1.0 | 391 | 8 | stages-2 | sampled | |
| 32bit | balanced | v1.0 | 405 | 6 | stages-3 | sampled | |
| 64bit | fast | v1.0 | 2270 | 4 | flat | sampled | |
| 64bit | small | v1.0 | 889 | 8 | stages-2 | sampled | |
| 64bit | balanced | v1.0 | 967 | 6 | stages-3 | sampled | |

Gates include the input gates. Up to 16 bit every input was tried. 32 and 64 bit were checked on about a million inputs: random ones, and
for every amount a single bit in every place, all bits, and all bits but one. None has been tested in the game yet.

## Generator

```
cargo build --release
target/release/pareto designs barrel-shifter 8bit       # every design, measured
target/release/pareto build-all barrel-shifter --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`.

| Design | Idea | Ticks | Gates |
|---|---|---|---|
| `flat` | Every output bit is an OR over one AND gate per shift amount | 4 at any size | about n x n / 2 |
| `stages-r` | The amount is taken r bits at a time, each stage shifting by what those bits say | 2 per stage + 2 | fewer at large sizes |
| `normal-form` | Every output is one XOR gate over AND gates of plain inputs (up to 8bit) | 3 | more |

`stages-1` is the classic chain of two-way choices. Taking 2 or 3 bits per stage is both smaller and faster
here, because an OR gate over 4 or 8 terms costs the same as one over 2.

From 32bit on an amount input drives more than 255 gates in `flat`, so it goes through buffer gates. They sit
beside the inverters, so they cost no tick.
