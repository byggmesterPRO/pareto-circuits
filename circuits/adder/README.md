# Adder

Adds two binary numbers. Size is the number of bits of each number: `8bit` adds two 8-bit numbers and gives an
8-bit sum plus a carry.

There is no carry input.

## Pins

- **Inputs:** the yellow gates in the first row. x = 0 .. n-1 is the first number, x = n .. 2n-1 the second,
  least significant bit first in each.
- **Outputs:** the green gates in the second row. x = 0 .. n-1 is the sum, least significant bit first, and
  x = n is the carry out.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 6 | 3 | ripple | exhaustive | |
| 2bit | small | v1.0 | 6 | 3 | ripple | exhaustive | |
| 2bit | efficient | v1.0 | 6 | 3 | ripple | exhaustive | |
| 4bit | fast | v1.0 | 18 | 3 | lookahead | exhaustive | |
| 4bit | small | v3.0 | 12 | 8 | sum-carry | exhaustive | |
| 4bit | efficient | v1.0 | 18 | 3 | lookahead | exhaustive | |
| 8bit | fast | v1.1 | 49 | 3 | clauses | exhaustive | |
| 8bit | small | v3.0 | 24 | 16 | sum-carry | exhaustive | |
| 8bit | efficient | v1.1 | 49 | 3 | clauses | exhaustive | |
| 16bit | fast | v1.1 | 133 | 3 | clauses | exhaustive | |
| 16bit | small | v3.0 | 48 | 32 | sum-carry | exhaustive | |
| 16bit | efficient | v2.0 | 93 | 4 | factors | exhaustive | |
| 32bit | fast | v1.1 | 365 | 3 | clauses | sampled | |
| 32bit | small | v3.0 | 96 | 64 | sum-carry | sampled | |
| 32bit | efficient | v2.0 | 220 | 4 | factors | sampled | |
| 64bit | fast | v1.1 | 1041 | 3 | clauses | sampled | |
| 64bit | small | v3.0 | 192 | 128 | sum-carry | sampled | |
| 64bit | efficient | v2.0 | 518 | 4 | factors | sampled | |

Gates and ticks do not count the input gates (see the main README). Up to 16 bit every input was tried (4.3 billion at 16 bit). At 32 and 64 bit
there are too many, so they were checked on about a million inputs: random ones, edge cases, and every carry
chain from any bit to any higher bit. None has been tested in the game yet.

## Generator

```
cargo build --release
target/release/pareto designs adder 16bit       # every design, measured
target/release/pareto build-all adder --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`, with the designs from the search in `sum_carry.rs` and `factors.rs`
next to it. It offers several designs per size, and the build tool gives each profile to whichever design wins
when measured.

| Design | Idea | Ticks | Gates |
|---|---|---|---|
| `ripple` | One 4-gate full adder per bit, the carry walking up | 2 per bit | about 4 per bit |
| `pairs` | Two bits at a time in a 7-gate block (exact search: 6 is impossible), the carry walking from pair to pair | 2 per bit, plus 1 | about 3.5 per bit |
| `lookahead` | Every sum bit sees its carry in one step | 3 at any size | about n x n / 2 |
| `blocks-k` | Lookahead inside blocks of k bits, the carry walking from block to block | 2 per block | in between |
| `two-level-k` | Blocks of k bits, and a second lookahead across the blocks | 5 at any size | far fewer than `lookahead` |
| `sum-carry` | The carry is read back from the sum, so a bit is 3 gates | 2 per bit | 3 per bit |
| `clauses` | Lookahead where small blocks say "a carry starts here" with OR gates over the inputs | 3 at any size | well under `lookahead`: 365 at 32 bit, where it has 595 |
| `factors` | Lookahead where a block's carry is an AND of shared OR factors, in up to 3 levels | 4 at any size | 6 to 8 per bit |
| `within-T` | A planner picks, per bit, the cheapest way to have its carry in time, for a limit of T ticks | at most T | between `factors` and `sum-carry` |

The trick behind all but `ripple` and `sum-carry`: a carry reaches a bit from exactly one place, the nearest lower bit that
starts one. So the terms for "a carry arrives from bit j" are never on two at a time, their OR is the same as
their XOR, and a sum bit is a single XOR gate over those terms. No gate is spent on the carry itself.

The carry from the sum, behind `sum-carry` and much of `within-T`: with s the sum of a bit, its carry out is
s xor or(a, b, s) xor and(a, b, s). So a bit needs the sum, the OR and the AND, and no gate for the carry: the
next sum is one wide XOR over its a and b and those three signals. That is 3 gates a bit where a full adder
takes 4, because the XOR that would make the carry is folded into the next sum. These designs came from a
search, in `tools/evolve/tasks/adder/runs`.

The full adder in `ripple` is 4 gates, which exact search showed is the fewest for one bit taken alone.
Where a gate would drive more than 255 others (the 32 and 64 bit `lookahead`), it is doubled.
