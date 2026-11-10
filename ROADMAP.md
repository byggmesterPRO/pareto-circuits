# Roadmap

Which circuit families exist, and which are asked for. A family is done when it has a generator, verified
builds at its sizes, and a README with pins and records.

## Done

`adder`, `barrel-shifter`, `comparator`, `counter`, `decoder`, `demultiplexer`, `double-dabble`,
`multiplexer`, `multiplier`, `ram`, `register`, `seven-segment`, `subtractor`.

`counter`, `ram` and `register` have memory. They take one-tick pulses, and are checked by random operations
against a model on the tick simulator (`tools/logic-sim/src/memory.rs`). Each family README says what its
`ticks` means.

## Asked for

Arithmetic, no memory:

- adder-subtractor in one, with a mode pin
- incrementer and decrementer
- divider (quotient and remainder)
- negate and absolute value (two's complement)
- square, and square root
- ALU: add, subtract, and, or, xor, shift, picked by an opcode

Converting and encoding:

- BCD to binary (the reverse of double dabble)
- BCD adder
- priority encoder
- binary to Gray code and back
- popcount
- parity
- lookup table / ROM from any list of values, up to 12 input bits

Comparing and picking:

- min and max of two numbers
- equality check against a fixed value
- leading-zero counter
- sorting network for a handful of numbers

With memory (checked by test script, like RAM and counter):

- shift register (serial in parallel out, and the other way)
- FIFO queue and stack
- clock and clock divider
- edge detector and pulse stretcher
- up/down counter, and a decimal (BCD) counter
- program counter with jump
- random number generator (LFSR)
- timer-based memory

Display and input:

- multi-digit display driver with leading zeros blanked
- keypad encoder with debounce
