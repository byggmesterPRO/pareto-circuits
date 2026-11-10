# Demultiplexer

Sends one data input to one of n output lines. Size is the number of lines: `16bit` has 16 lines and a 4-bit
line number. The other lines stay off.

## Pins

- **Inputs:** the yellow gates in the first row. x = 0 is the data, x = 1 and up is the line number, least
  significant bit first.
- **Outputs:** the green gates, laid out as a matrix behind the inputs: the low half of the line number picks
  x, the high half picks the row (y = 1 and up). `2bit` has its two lines side by side.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 5 | 3 | flat | exhaustive | |
| 2bit | small | v1.0 | 5 | 3 | flat | exhaustive | |
| 2bit | balanced | v1.0 | 5 | 3 | flat | exhaustive | |
| 4bit | fast | v1.0 | 9 | 3 | flat | exhaustive | |
| 4bit | small | v1.0 | 9 | 3 | flat | exhaustive | |
| 4bit | balanced | v1.0 | 9 | 3 | flat | exhaustive | |
| 8bit | fast | v1.0 | 15 | 3 | flat | exhaustive | |
| 8bit | small | v1.0 | 15 | 3 | flat | exhaustive | |
| 8bit | balanced | v1.0 | 15 | 3 | flat | exhaustive | |
| 16bit | fast | v1.0 | 25 | 3 | flat | exhaustive | |
| 16bit | small | v1.0 | 25 | 3 | flat | exhaustive | |
| 16bit | balanced | v1.0 | 25 | 3 | flat | exhaustive | |
| 32bit | fast | v1.0 | 43 | 3 | flat | exhaustive | |
| 32bit | small | v1.0 | 43 | 3 | flat | exhaustive | |
| 32bit | balanced | v1.0 | 43 | 3 | flat | exhaustive | |
| 64bit | fast | v1.0 | 77 | 3 | flat | exhaustive | |
| 64bit | small | v1.0 | 77 | 3 | flat | exhaustive | |
| 64bit | balanced | v1.0 | 77 | 3 | flat | exhaustive | |

Gates include the input gates. Every input was tried at every size. None has been tested in the game yet.

## Generator

```
cargo build --release
target/release/pareto designs demultiplexer 8bit       # every design, measured
target/release/pareto build-all demultiplexer --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`.

| Design | Idea | Ticks | Gates |
|---|---|---|---|
| `flat` | One AND gate per line over the data and the line number bits or their inverses | 3 | n + log n |
| `matrix` | The high half of the line number is decoded into rows (which carry the data), the low half into columns, and each output is the AND of its row and its column | 4 | more |

`matrix` is how it is built from two-input parts, and what the layout looks like. With gates that take any
number of inputs it only adds the row and column gates and a tick, so `flat` holds every slot. Both are laid
out as the same matrix.
