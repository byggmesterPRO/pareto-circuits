# Seven segment

Turns one hexadecimal digit into the seven segments of a display. There is one size, `4bit`.

```
 aaa
f   b
 ggg
e   c
 ddd
```

0 to 9 are the usual shapes (6 and 9 with their tails, 7 without the top left bar), then A, b, C, d, E, F.

## Pins

- **Inputs:** the yellow gates in the first row, the digit, least significant bit first.
- **Outputs:** the green gates in the second row. x = 0 .. 6 are segments a, b, c, d, e, f, g.

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 4bit | fast | v1.0 | 22 | 3 | normal-form | exhaustive | |
| 4bit | small | v1.0 | 18 | 8 | searched-small | exhaustive | |
| 4bit | balanced | v1.0 | 22 | 3 | normal-form | exhaustive | |

Gates include the input gates. All 16 digits were tried. None has been tested in the game yet.

## Generator

```
cargo build --release
target/release/pareto designs seven-segment 4bit       # every design, measured
target/release/pareto build-all seven-segment --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`.

| Design | Idea | Ticks |
|---|---|---|
| `table` | One line per digit, and every segment is an OR (or NOR) over digits | 4 |
| `normal-form` | Every segment is one XOR gate over AND gates of plain inputs | 3 |
| `searched-*` | Found by `optimizers/fable/resub` starting from the two above | |

With one size there is nothing to generate, so the searched circuits are stored in the generator as they
are.
