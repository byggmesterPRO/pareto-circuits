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
| 2bit | fast | v1.0 | 11 | 3 | normal-form | exhaustive | |
| 2bit | small | v1.0 | 11 | 3 | normal-form | exhaustive | |
| 2bit | efficient | v1.0 | 11 | 3 | normal-form | exhaustive | |
| 4bit | fast | v1.0 | 112 | 3 | normal-form | exhaustive | |
| 4bit | small | v1.0 | 64 | 12 | ripple | exhaustive | |
| 4bit | efficient | v1.0 | 112 | 3 | normal-form | exhaustive | |
| 8bit | fast | v1.0 | 534 | 10 | wide-blocks-8 | exhaustive | |
| 8bit | small | v1.0 | 288 | 28 | ripple | exhaustive | |
| 8bit | efficient | v1.0 | 326 | 12 | tree-blocks-8 | exhaustive | |
| 16bit | fast | v1.0 | 3355 | 12 | wide-lookahead | exhaustive | |
| 16bit | small | v1.0 | 1216 | 60 | ripple | exhaustive | |
| 16bit | efficient | v1.0 | 1414 | 16 | tree-blocks-16 | exhaustive | |
| 32bit | fast | v1.0 | 15903 | 13 | wide-lookahead | sampled | |
| 32bit | small | v1.0 | 4992 | 124 | ripple | sampled | |
| 32bit | efficient | v1.0 | 5372 | 20 | tree-two-level-8 | sampled | |

Gates include the input gates. Up to 16 bit every input was tried (4.3 billion at 16 bit). 32 bit was checked on about a million inputs:
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

`<adder>` is one of the adder family's carry networks: `lookahead`, `blocks-k` or `two-level-k`.

The counters rest on a fact about counting: the 2s bit of "how many of these are on" is the XOR of the AND of
every pair, and the 4s bit is the XOR of the AND of every four. In the first layer the pairs and fours are taken
straight from the input bits, which saves a tick.
