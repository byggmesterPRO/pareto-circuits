# Multiplier

Multiplies two binary numbers (unsigned). Size is the number of bits of each number: `8bit` gives a 16-bit
product.

There is no 64bit: it would be about 20 000 gates.

## Pins

- **Inputs:** the yellow gates in the first row. x = 0 .. n-1 is a, x = n .. 2n-1 is b, least significant bit
  first in each.
- **Outputs:** the green gates in the second row, x = 0 .. 2n-1, least significant bit first.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 7 | 2 | normal-form | exhaustive | |
| 2bit | small | v1.0 | 7 | 2 | normal-form | exhaustive | |
| 2bit | efficient | v1.0 | 7 | 2 | normal-form | exhaustive | |
| 4bit | fast | v1.0 | 104 | 2 | normal-form | exhaustive | |
| 4bit | small | v2.0 | 50 | 13 | parts-ripple | exhaustive | |
| 4bit | efficient | v1.0 | 104 | 2 | normal-form | exhaustive | |
| 8bit | fast | v1.0 | 518 | 9 | wide-blocks-8 | exhaustive | |
| 8bit | small | v2.0 | 230 | 31 | parts-ripple | exhaustive | |
| 8bit | efficient | v1.0 | 310 | 11 | tree-blocks-8 | exhaustive | |
| 16bit | fast | v1.1 | 3129 | 11 | wide+clauses | exhaustive | |
| 16bit | small | v2.0 | 974 | 67 | parts-ripple | exhaustive | |
| 16bit | efficient | v2.0 | 1271 | 16 | tree+factors | exhaustive | |
| 32bit | fast | v1.1 | 14626 | 12 | wide+clauses | sampled | |
| 32bit | small | v2.0 | 3998 | 139 | parts-ripple | sampled | |
| 32bit | efficient | v2.0 | 5177 | 18 | tree+factors | sampled | |

Gates and ticks do not count the input gates (see the main README). Up to 16 bit every input was tried (4.3 billion at 16 bit). 32 bit was checked on about a million inputs:
random ones, edge cases, single bits and runs of ones. None has been tested in the game yet.

## Generator

```
cargo build --release
target/release/pareto designs multiplier 8bit       # every design, measured
target/release/pareto build-all multiplier --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`.

Every design makes the n x n products of one bit of a and one bit of b, and then adds up the columns.

| Design | Idea |
|---|---|
| `ripple` | Full adders (4 gates) bring every column down to one bit, lowest column first. Fewest gates, and the carries walk. |
| `tree-<adder>` | Full adders in layers bring the columns down to two rows; a fast adder adds those. |
| `wide-<adder>` | The same with counters of up to 7 bits in and 3 out. A counter is 2 gates deep whatever its size, so there are far fewer layers, at several times the gates. |
| `normal-form` | Every output is one XOR gate over AND gates of plain inputs (up to 4bit) |
| `parts-ripple`, `parts-<adder>` | Like `ripple` and `tree-<adder>`, with full adders of 3 gates: the carry is handed on as three signals whose XOR it is, and the adder that reads it takes all three into its XOR gate. Fewer gates, and a tick or two more where a carry has to be joined after all. |
| `tree+<design>`, `parts+<design>`, `wide+<design>` | The last two rows are added by a design from the adder's search (`clauses`, `factors`, `within-T`, `sum-carry`; see `circuits/adder`). Only the ones nothing else beats are offered. |

`<adder>` is one of the adder family's carry networks: `lookahead`, `blocks-k` or `two-level-k`.

The counters rest on a fact about counting: the 2s bit of "how many of these are on" is the XOR of the AND of
every pair, and the 4s bit is the XOR of the AND of every four. In the first layer the pairs and fours are taken
straight from the input bits, which saves a tick.
