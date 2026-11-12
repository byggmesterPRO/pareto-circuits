# Counter

Counts pulses. Size is the number of bits: `8bit` counts 0 to 255 and then starts again at 0.

## Pins

- **Inputs:** the yellow gates in the first row. x = 0 is count, x = 1 is reset. Both are one-tick pulses, and
  not both in the same tick.
- **Outputs:** the green gates in the second row, x = 0 .. n-1: the count, least significant bit first.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Timing

All of these store a bit in an XOR gate wired to itself. It keeps its value, and flips when its other input
is on for one tick. So a pulse input has to be on for **exactly one tick**: a second tick flips the bits back.
A button or switch is too slow; send it through a one-tick pulse maker first.

The gates wired to themselves start out off, so the memory starts at 0.

`ticks` is the time from a pulse to the new count on the outputs: 3 at any size (bit 0 is there after 2).
Pulses can follow each other 3 ticks apart.

A tick is counted from the moment an input is driven, so the input gate itself is the first tick.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 7 | 3 | toggle | sampled | |
| 2bit | small | v1.0 | 7 | 3 | toggle | sampled | |
| 2bit | efficient | v1.0 | 7 | 3 | toggle | sampled | |
| 4bit | fast | v1.0 | 13 | 3 | toggle | sampled | |
| 4bit | small | v1.0 | 13 | 3 | toggle | sampled | |
| 4bit | efficient | v1.0 | 13 | 3 | toggle | sampled | |
| 8bit | fast | v1.0 | 25 | 3 | toggle | sampled | |
| 8bit | small | v1.0 | 25 | 3 | toggle | sampled | |
| 8bit | efficient | v1.0 | 25 | 3 | toggle | sampled | |
| 16bit | fast | v1.0 | 49 | 3 | toggle | sampled | |
| 16bit | small | v1.0 | 49 | 3 | toggle | sampled | |
| 16bit | efficient | v1.0 | 49 | 3 | toggle | sampled | |
| 32bit | fast | v1.0 | 97 | 3 | toggle | sampled | |
| 32bit | small | v1.0 | 97 | 3 | toggle | sampled | |
| 32bit | efficient | v1.0 | 97 | 3 | toggle | sampled | |
| 64bit | fast | v1.0 | 193 | 3 | toggle | sampled | |
| 64bit | small | v1.0 | 193 | 3 | toggle | sampled | |
| 64bit | efficient | v1.0 | 193 | 3 | toggle | sampled | |

Gates include the input gates. The three profiles hold the same build where one design wins all of them.
Checked on the tick simulator: 3000 rounds of count, reset or nothing, on 64 copies at once, against a model. That reaches counts of about
1000, so above 10 bits the high bits were never seen to flip; they are the same three gates as the low ones.
None has been tested in the game yet, and these lean on the simulator's tick model more than circuits without
memory do.

## Generator

```
cargo build --release
target/release/pareto designs counter 8bit       # every design, measured
target/release/pareto build-all counter --write   # rebuild the slots that a design now wins
target/release/logic-sim verify circuits/counter/builds/8bit/fast counter 8bit
```

The code is in `generator/src/lib.rs`.

One design, `toggle`, 3 gates per bit: the stored bit, an AND gate over count and every lower bit (the bit
flips when all the bits below it are 1), and an AND gate over reset and the bit itself (a 1 flips back to 0).
Every bit sees the count pulse at the same time, so nothing walks from bit to bit.
