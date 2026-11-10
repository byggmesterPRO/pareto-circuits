# Double dabble

Binary to BCD: turns a binary number into decimal digits, four bits per digit. Use it in front of a
seven-segment display or anything else that shows a number in decimal.

Size is the number of input bits. `12bit` takes 0..4095 and gives four digits.

## Pins

- **Inputs:** the yellow gates in the first row, least significant bit at x = 0. Wire your number into these.
- **Outputs:** the green gates. x = 0..3 is the ones digit, x = 4..7 the tens, and so on, least significant
  bit first in each digit. They are in the second row, except in the three 12bit builds, where they sit one
  row further back.
- A digit bit that passes straight through has no green gate: read it from the input gate. That is ones bit 0,
  which is input bit 0, in every build but the 12bit `fast` and `balanced`.
- A digit bit that can never be 1 has no gate at all. At 12 bits that is bit 3 of the thousands digit (x = 15),
  since the highest value is 4095.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`. That is what the verifier
reads; the colours are for players.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 2 | 1 | chain-7 | exhaustive | |
| 2bit | small | v1.0 | 2 | 1 | chain-7 | exhaustive | |
| 2bit | balanced | v1.0 | 2 | 1 | chain-7 | exhaustive | |
| 4bit | fast | v1.0 | 11 | 3 | chain-8 | exhaustive | |
| 4bit | small | v1.0 | 10 | 6 | chain-7 | exhaustive | |
| 4bit | balanced | v1.0 | 11 | 3 | chain-8 | exhaustive | |
| 8bit | fast | v1.0 | 62 | 11 | chain-8 | exhaustive | |
| 8bit | small | v1.0 | 55 | 30 | chain-7 | exhaustive | |
| 8bit | balanced | v1.0 | 62 | 11 | chain-8 | exhaustive | |
| 12bit | fast | v1.0 | 327 | 4 | search | exhaustive | |
| 12bit | small | v1.0 | 101 | 52 | search | exhaustive | |
| 12bit | balanced | v1.0 | 327 | 4 | search | exhaustive | |
| 16bit | fast | v1.0 | 277 | 27 | chain-8 | exhaustive | |
| 16bit | small | v1.0 | 244 | 78 | chain-7 | exhaustive | |
| 16bit | balanced | v1.0 | 277 | 27 | chain-8 | exhaustive | |
| 32bit | fast | v1.0 | 1177 | 59 | chain-8 | exhaustive | |
| 32bit | small | v1.0 | 1033 | 174 | chain-7 | exhaustive | |
| 32bit | balanced | v1.0 | 1177 | 59 | chain-8 | exhaustive | |
| 64bit | fast | v1.0 | 4811 | 123 | chain-8 | sampled | |
| 64bit | small | v1.0 | 4216 | 366 | chain-7 | sampled | |
| 64bit | balanced | v1.0 | 4811 | 123 | chain-8 | sampled | |

Gates and ticks are counted as in the main README: input gates are included, and ticks is the number of gates on
the longest path, both ends included. Up to 32 bit every input was tried (4.3 billion at 32 bit); 64 bit was
checked on about a million inputs. None has been tested in the game yet.

- The 2bit build is just the two input gates: a number below 4 is its own ones digit.
- **12bit** holds one-off search results (`source: search`) that beat the generator there, found with the
  tools in `optimizers/fable`. `fast` is a lookup table: input gates, a layer of re-coded inputs, a layer of
  terms, one XOR per output. `small` came from rewriting the plain chain gate by gate. In the older way of
  counting (without the input gates, ticks after them) they are 315 gates at 3 ticks and 89 gates.
- The 12bit `small` build never uses its longest path. Over 100 000 random input changes the slowest took 29
  ticks, but 52 is the number that is guaranteed.
- niknal has reported a 12bit small build of 79 gates without the input gates. It is not in the repo yet, and
  it would take that slot.
- The search tools stop at 16 input bits (gate count) and 12 (lookup table), so the larger sizes come from the
  generator alone. Nothing here is fast in the way the 12bit lookup table is: both designs get slower with
  every bit. A design whose delay grows more slowly is still missing.

## Generator

```
cargo build --release
target/release/pareto designs double-dabble 16bit       # every design, measured
target/release/pareto build-all double-dabble --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`. Both designs are the classic shift-and-add-3 chain: feed the bits in
from the top, and before each new bit add 3 to every digit that is 5 or more. A digit only gets an add-3 cell
at the steps where it can reach 5 at all. They differ in the cell.

| Design | Cell | Result |
|---|---|---|
| `chain-7` | 7 gates, the fewest an add-3 cell can have on its own (exact search), up to 4 gates deep | smallest |
| `chain-8` | 8 gates in two layers: four AND terms, then one XOR per output bit | about a third of the ticks for 13 % more gates |
