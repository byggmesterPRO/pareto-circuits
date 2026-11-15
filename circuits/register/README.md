# Register

Keeps a number until the next write. Size is the number of bits.

## Pins

- **Inputs:** the yellow gates in the first row. x = 0 .. n-1 is the data, least significant bit first, and
  x = n is write (a one-tick pulse).
- **Outputs:** the green gates in the second row, x = 0 .. n-1: the stored number.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Timing

All of these store a bit in an XOR gate wired to itself. It keeps its value, and flips when its other input
is on for one tick. So a pulse input has to be on for **exactly one tick**: a second tick flips the bits back.
A button or switch is too slow; send it through a one-tick pulse maker first.

The gates wired to themselves start out off, so the memory starts at 0.

`ticks` is the time from the write pulse to the new number on the outputs: 2. The data has to be steady for
1 tick before the pulse and until the tick after it.

Writes can follow each other 4 ticks apart: new data, the write pulse one tick after it, and the number is
on the outputs when the 4 ticks are over.

Ticks are counted as in the main README: the tick of the input gate is not counted. Gaps between two inputs, such as how far apart pulses can come, are the same either way.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 6 | 2 | toggle | sampled | |
| 2bit | small | v1.0 | 6 | 2 | toggle | sampled | |
| 2bit | efficient | v1.0 | 6 | 2 | toggle | sampled | |
| 4bit | fast | v1.0 | 12 | 2 | toggle | sampled | |
| 4bit | small | v1.0 | 12 | 2 | toggle | sampled | |
| 4bit | efficient | v1.0 | 12 | 2 | toggle | sampled | |
| 8bit | fast | v1.0 | 24 | 2 | toggle | sampled | |
| 8bit | small | v1.0 | 24 | 2 | toggle | sampled | |
| 8bit | efficient | v1.0 | 24 | 2 | toggle | sampled | |
| 16bit | fast | v1.0 | 48 | 2 | toggle | sampled | |
| 16bit | small | v1.0 | 48 | 2 | toggle | sampled | |
| 16bit | efficient | v1.0 | 48 | 2 | toggle | sampled | |
| 32bit | fast | v1.0 | 96 | 2 | toggle | sampled | |
| 32bit | small | v1.0 | 96 | 2 | toggle | sampled | |
| 32bit | efficient | v1.0 | 96 | 2 | toggle | sampled | |
| 64bit | fast | v1.0 | 192 | 2 | toggle | sampled | |
| 64bit | small | v1.0 | 192 | 2 | toggle | sampled | |
| 64bit | efficient | v1.0 | 192 | 2 | toggle | sampled | |

Gates and ticks do not count the input gates (see the main README). The three profiles hold the same build where one design wins all of them.
Checked on the tick simulator: 3000 rounds of random data with and without a write, on 64 copies at once, against a model.
None has been tested in the game yet, and these lean on the simulator's tick model more than circuits without
memory do.

## Generator

```
cargo build --release
target/release/pareto designs register 8bit       # every design, measured
target/release/pareto build-all register --write   # rebuild the slots that a design now wins
target/release/logic-sim verify circuits/register/builds/8bit/fast register 8bit
```

The code is in `generator/src/lib.rs`.

One design, `toggle`, 3 gates per bit: the stored bit, an XOR gate for "the data differs from what is stored",
and an AND gate that flips the bit when that is so and write is on.
