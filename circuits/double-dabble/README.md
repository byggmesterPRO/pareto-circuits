# Double dabble

Binary to BCD: turns a binary number into decimal digits, four bits per digit. Use it in front of a
seven-segment display or anything else that shows a number in decimal.

Size is the number of input bits. `12bit` takes 0..4095 and gives four digits.

## Pins

- **Inputs:** the yellow gates in the first row, least significant bit at x = 0. Wire your number into these.
- **Outputs:** the green gates in the row two blocks further back. x = 0..3 is the ones digit, x = 4..7 the
  tens, and so on, least significant bit first in each digit.
- A digit bit that passes straight through has no green gate: read it from the input gate. In the `small` build
  that is ones bit 0, which is input bit 0.
- A digit bit that can never be 1 has no gate at all. At 12 bits that is bit 3 of the thousands digit (x = 15),
  since the highest value is 4095.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`. That is what the verifier
reads; the colours are for players.

## Records

| Size | Profile | Version | Gates | Ticks | Workshop |
|---|---|---|---|---|---|
| 12bit | fast | v1.0 | 327 | 4 | |
| 12bit | small | v1.0 | 101 | 52 | |
| 12bit | balanced | v1.0 | 327 | 4 | |

Gates and ticks are counted as in the main README: input gates are included, and ticks is the number of gates on
the longest path, both ends included. In the older way of counting (without the input gates, ticks after them)
the fast build is 315 gates at 3 ticks and the small build is 89 gates.

- `fast` is a lookup table: input gates, a layer of re-coded inputs, a layer of terms, one XOR per output.
  Three layers of gates after the inputs is the least that fits the game's limit of 255 connections per part.
- `small` never uses its longest path. Over 100 000 random input changes the slowest took 29 ticks
  (`tools/bygg-logic-sim/delay.py`), but 52 is the number that is guaranteed.
- `balanced` is the same circuit as `fast` for now. Nothing found yet has a lower gates x ticks.
- niknal has reported a small build of 79 gates without the input gates. It is not in the repo yet, and it would
  take the `small` slot.
- All three are checked on every input by `tools/bygg-logic-sim/verify.py`. None has been tested in the game yet.

## Generator

```
python circuits/double-dabble/generator/generate.py <bits> <out netlist.json>
```

Works for any number of bits. It builds the classic shift-and-add-3 chain from a 7-gate add-3 cell, which is
the fewest gates such a cell can have on its own. Verified on every input at 4, 8, 12 and 16 bits and by sampling
at 32 bits.

| Bits | Gates | Ticks |
|---|---|---|
| 4 | 10 | 6 |
| 8 | 55 | 30 |
| 12 | 135 | 54 |
| 16 | 244 | 78 |
| 32 | 1033 | 174 |

It only has this one profile so far. The 12bit builds above are one-off search results (`source: search`)
that beat it, found with the tools in `optimizers/fable`. To turn a netlist into a build slot:

```
python tools/bygg-logic-sim/make_build.py <netlist.json> double-dabble <size> <profile> --source generator --authors <name>
```
