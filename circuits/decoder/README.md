# Decoder

Turns a binary number into one line per value. Size is the number of input bits: `4bit` has 16 output lines,
`8bit` has 256. Exactly one line is on at a time, the one whose number the inputs spell.

## Pins

- **Inputs:** the yellow gates in the first row, least significant bit first.
- **Outputs:** the green gates. Line k is at x = k in the second row for 2bit. From 4bit on the lines form a
  square behind the inputs: the low half of the bits picks x, the high half picks the row (y = 1 and up).

The exact gate behind every pin is listed under `pins` in each build's `meta.json`.

## Records

| Size | Profile | Version | Gates | Ticks | Design | Verified | Workshop |
|---|---|---|---|---|---|---|---|
| 2bit | fast | v1.0 | 8 | 3 | flat | exhaustive | |
| 2bit | small | v1.0 | 8 | 3 | flat | exhaustive | |
| 2bit | efficient | v1.0 | 8 | 3 | flat | exhaustive | |
| 4bit | fast | v1.0 | 24 | 3 | flat | exhaustive | |
| 4bit | small | v1.0 | 24 | 3 | flat | exhaustive | |
| 4bit | efficient | v1.0 | 24 | 3 | flat | exhaustive | |
| 8bit | fast | v1.0 | 272 | 3 | flat | exhaustive | |
| 8bit | small | v1.0 | 272 | 3 | flat | exhaustive | |
| 8bit | efficient | v1.0 | 272 | 3 | flat | exhaustive | |

Gates include the input gates. Every input was tried at every size. None has been tested in the game yet.

## Generator

```
cargo build --release
target/release/pareto designs decoder 8bit       # every design, measured
target/release/pareto build-all decoder --write   # rebuild the slots that a design now wins
```

The code is in `generator/src/lib.rs`.

One design, `flat`: an inverter per input, and one AND gate per line over the inputs or their inverses. Gates
take any number of inputs, so decoding in stages would only add gates and ticks.
