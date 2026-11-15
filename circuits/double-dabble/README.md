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
  which is input bit 0, in every build but the 12bit `fast` and `efficient`.
- A digit bit that can never be 1 has no gate at all. At 12 bits that is bit 3 of the thousands digit (x = 15),
  since the highest value is 4095.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`. That is what the verifier
reads; the colours are for players.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 0 | 0 | chain-7 | exhaustive | |
| 2bit | small | v1.0 | 0 | 0 | chain-7 | exhaustive | |
| 2bit | efficient | v1.0 | 0 | 0 | chain-7 | exhaustive | |
| 4bit | fast | v1.0 | 7 | 2 | chain-8 | exhaustive | |
| 4bit | small | v1.0 | 6 | 5 | chain-7 | exhaustive | |
| 4bit | efficient | v1.0 | 7 | 2 | chain-8 | exhaustive | |
| 8bit | fast | v1.0 | 54 | 10 | chain-8 | exhaustive | |
| 8bit | small | v1.0 | 47 | 29 | chain-7 | exhaustive | |
| 8bit | efficient | v1.0 | 54 | 10 | chain-8 | exhaustive | |
| 12bit | fast | v1.0 | 315 | 3 | search | exhaustive | |
| 12bit | small | v1.0 | 89 | 51 | search | exhaustive | |
| 12bit | efficient | v1.0 | 315 | 3 | search | exhaustive | |
| 16bit | fast | v1.0 | 261 | 26 | chain-8 | exhaustive | |
| 16bit | small | v1.0 | 228 | 77 | chain-7 | exhaustive | |
| 16bit | efficient | v1.0 | 261 | 26 | chain-8 | exhaustive | |
| 32bit | fast | v1.0 | 1145 | 58 | chain-8 | exhaustive | |
| 32bit | small | v1.0 | 1001 | 173 | chain-7 | exhaustive | |
| 32bit | efficient | v1.0 | 1145 | 58 | chain-8 | exhaustive | |
| 64bit | fast | v1.0 | 4747 | 122 | chain-8 | sampled | |
| 64bit | small | v1.0 | 4152 | 365 | chain-7 | sampled | |
| 64bit | efficient | v1.0 | 4747 | 122 | chain-8 | sampled | |

Gates and ticks are counted as in the main README: the input gates are not counted, and ticks is the number of
gates on the longest path after the input gate, the output gate included. Up to 32 bit every input was tried (4.3 billion at 32 bit); 64 bit was
checked on about a million inputs. None has been tested in the game yet.

- The 2bit build is just the two input gates, so it counts as 0 gates and 0 ticks: a number below 4 is its own
  ones digit.
- **12bit** holds one-off search results (`source: search`) that beat the generator there, found with the
  tools in `optimizers/fable`. `fast` is a lookup table: input gates, a layer of re-coded inputs, a layer of
  terms, one XOR per output. `small` came from rewriting the plain chain gate by gate. Counted with the input
  gates, the way the repo counted before, they were 327 gates at 4 ticks and 101 gates.
- The 12bit `small` build never uses its longest path. Over 100 000 random input changes the slowest took 28
  ticks, but 51 is the number that is guaranteed.
- niknal has reported a 12bit small build of 79 gates. It is not in the repo yet, and
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
