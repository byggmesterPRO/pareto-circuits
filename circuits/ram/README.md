# RAM

Words of memory, one of them picked by an address. Size is `<words>x<bits>bit`: `16x8bit` is 16 words of
8 bits. The word at the address is on the outputs all the time; a write pulse stores the data there.

## Pins

- **Inputs:** the yellow gates in the first row, in this order, least significant bit first in each: the
  address (4 bits for 16 words), the data to write, and write (a one-tick pulse).
- **Outputs:** the green gates in the second row: the word at the address.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Timing

All of these store a bit in an XOR gate wired to itself. It keeps its value, and flips when its other input
is on for one tick. So a pulse input has to be on for **exactly one tick**: a second tick flips the bits back.
A button or switch is too slow; send it through a one-tick pulse maker first.

The gates wired to themselves start out off, so the memory starts at 0.

`ticks` is the read delay: from a new address to its word on the outputs.

| Size | Read | Write pulse to outputs | Address and data steady before write |
|---|---|---|---|
| 16x4bit, 16x8bit | 4 | 5 | 5 |
| 64x8bit | 4 | 6 | 5 |
| 256x8bit | 5 | 7 | 6 |

Keep the address and data steady until 2 ticks after the pulse.

Reads and writes can follow each other without a gap, each taking this many ticks: the address and data
change at the start, the write pulse (if it is a write) comes some ticks later, and the word at the address
is on the outputs when the ticks are over.

| Size | Ticks per operation | Write pulse after the address and data |
|---|---|---|
| 16x4bit, 16x8bit | 9 | 4 |
| 64x8bit | 9 | 3 |
| 256x8bit | 11 | 4 |

A tick is counted from the moment an input is driven, so the input gate itself is the first tick.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 16x4bit | fast | v1.0 | 213 | 4 | direct | sampled | |
| 16x4bit | small | v1.0 | 213 | 4 | direct | sampled | |
| 16x4bit | efficient | v1.0 | 213 | 4 | direct | sampled | |
| 16x8bit | fast | v1.0 | 417 | 4 | direct | sampled | |
| 16x8bit | small | v1.0 | 417 | 4 | direct | sampled | |
| 16x8bit | efficient | v1.0 | 417 | 4 | direct | sampled | |
| 64x8bit | fast | v1.0 | 1606 | 4 | direct | sampled | |
| 64x8bit | small | v1.0 | 1606 | 4 | direct | sampled | |
| 64x8bit | efficient | v1.0 | 1606 | 4 | direct | sampled | |
| 256x8bit | fast | v1.0 | 6346 | 5 | direct | sampled | |
| 256x8bit | small | v1.0 | 6346 | 5 | direct | sampled | |
| 256x8bit | efficient | v1.0 | 6346 | 5 | direct | sampled | |

Gates include the input gates. The three profiles hold the same build where one design wins all of them.
Checked on the tick simulator: 3000 rounds of a random address and data, with and without a write, on 64 copies at once, against a model.
None has been tested in the game yet, and these lean on the simulator's tick model more than circuits without
memory do.

## Generator

```
cargo build --release
target/release/pareto designs ram 16x8bit       # every design, measured
target/release/pareto build-all ram --write   # rebuild the slots that a design now wins
target/release/logic-sim verify circuits/ram/builds/16x8bit/fast ram 16x8bit
```

The code is in `generator/src/lib.rs`.

| Design | Idea |
|---|---|
| `direct` | Every cell reads the address bits itself. 3 gates per stored bit plus one XOR gate per data bit. |
| `lines` | Each word has a "picked" line and a "write here" line. Fewer wires, 2 gates per word more, a tick slower. |

The word at the address is always on the outputs, so "which bits differ from the data" is one XOR gate per
data bit for the whole RAM. A cell is the stored bit, an AND gate that flips it (write, its address, and the
bit differs), and an AND gate that puts it on the output (its address, and the bit).
At 256 words the output OR is split in two, because a gate takes 255 inputs.
