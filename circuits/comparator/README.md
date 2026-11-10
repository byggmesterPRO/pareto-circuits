# Comparator

Compares two binary numbers (unsigned). Size is the number of bits of each number. Exactly one of the three
outputs is on.

## Pins

- **Inputs:** the yellow gates in the first row. x = 0 .. n-1 is a, x = n .. 2n-1 is b, least significant bit
  first in each.
- **Outputs:** the green gates in the second row. x = 0 is a < b, x = 1 is a = b, x = 2 is a > b.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 15 | 3 | normal-form | exhaustive | |
| 2bit | small | v1.0 | 13 | 5 | flat-nor | exhaustive | |
| 2bit | balanced | v1.0 | 15 | 3 | normal-form | exhaustive | |
| 4bit | fast | v1.0 | 123 | 3 | normal-form | exhaustive | |
| 4bit | small | v1.0 | 23 | 5 | flat-nor | exhaustive | |
| 4bit | balanced | v1.0 | 27 | 4 | flat | exhaustive | |
| 8bit | fast | v1.0 | 51 | 4 | flat | exhaustive | |
| 8bit | small | v1.0 | 43 | 5 | flat-nor | exhaustive | |
| 8bit | balanced | v1.0 | 51 | 4 | flat | exhaustive | |
| 16bit | fast | v1.0 | 99 | 4 | flat | exhaustive | |
| 16bit | small | v1.0 | 83 | 5 | flat-nor | exhaustive | |
| 16bit | balanced | v1.0 | 99 | 4 | flat | exhaustive | |
| 32bit | fast | v1.0 | 195 | 4 | flat | sampled | |
| 32bit | small | v1.0 | 163 | 5 | flat-nor | sampled | |
| 32bit | balanced | v1.0 | 195 | 4 | flat | sampled | |
| 64bit | fast | v1.0 | 387 | 4 | flat | sampled | |
| 64bit | small | v1.0 | 323 | 5 | flat-nor | sampled | |
| 64bit | balanced | v1.0 | 387 | 4 | flat | sampled | |

Gates include the input gates. Up to 16 bit every input was tried. At 32 and 64 bit they were checked on about a million inputs: random
ones, edge cases, equal numbers, and numbers that differ in a single bit. None has been tested in the game yet.

## Generator

```
cargo build --release
target/release/pareto designs comparator 8bit       # every design, measured
target/release/pareto build-all comparator --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`.

| Design | Idea | Ticks | Gates |
|---|---|---|---|
| `flat` | One AND gate per bit for "they differ here, b has the 1, all bits above are equal", and the same for a | 4 at any size | about 4 per bit |
| `flat-nor` | The same for below only; above is "neither below nor equal" | 5 at any size | about 3 per bit |
| `normal-form` | Every output is one XOR gate over AND gates of plain inputs (up to 4bit) | 3 | many |

Walking the answer up bit by bit (3 gates per bit is the fewest, by exact search) and the subtractor's borrow
networks were measured too. Both lose to `flat-nor` on gates and on ticks, so they are not in the generator.
