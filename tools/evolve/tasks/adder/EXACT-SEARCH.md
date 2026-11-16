# Exact-search results for adder blocks

Tools: `synth.py` (synth2.py extended with inactive rows, a symbolic m-wire carry code, solver choice),
`run.py` (one block), `joint.py` (several blocks that share one code). Solver CaDiCaL 1.9.5 via pysat unless
noted. "Code cM" = the carry between blocks is M wires, and which wire patterns mean carry 0 / carry 1 is left
to the solver (unused patterns are don't-care rows). "same" = the block's carry out uses the same code as its
carry in (so the block can be repeated). first = no carry in; last = plain carry out; mid = code in and out.
Budgets are conflict budgets.

## Sanity (known results reproduced)
- 1 bit, plain carry in/out, 3 gates: impossible. 4 gates: found.
- 1 bit, no carry in, plain out, 2 gates: found (half adder).

## Plain carry
- 2 bits, no carry in, plain carry out, 5 gates: impossible (3.6s). 6 gates: found. => 6 is minimal.

## Coded carry, single blocks
- 1 bit mid, c2 same, 3 gates: impossible.
- 1 bit mid, c3 same, 3 gates: FOUND. Code = parity of the three wires. Gates s = XOR(a, b, w0, w1, w2),
  AND(a, b, s), OR(a, b, s, AND); out wires (s, AND, OR). => 3 gates per bit (vs 3.5 for the 7-gate pair).
- 1 bit mid, c3 same, 2 gates: impossible. c4 same, 2 gates: impossible (148s). => 3 per bit is minimal for a
  repeated one-bit block with up to 4 wires.
- 1 bit last (code c3 in, plain out), 3 gates: impossible; 4 gates: found (with its own code).
  c4 in, plain out, 3 gates: impossible.
- 1 bit first (no carry in, code c3 out), 1 gate: found (the code may use the input wires themselves).
- 2 bits first (no carry in, c3 out), 4 gates: found (free code).

## Joint (first, mid, last sharing one code)
- m=3 first:1:1 + mid:1:3: FOUND (code: wires (p, b, a) of the bit, mid decodes it).
- m=3 first:2:4 + mid:1:3: FOUND (no better than 1 + 3).
- m=3 mid:1:3 + last:1:3: impossible.
- m=2 first:1:1 + mid:1:3 + last:1:4: impossible. m=3: impossible. m=4: impossible (21s).
- m=3 first:1:1 + mid:1:3 + last:1:5: impossible (53s).
- m=3 first:1:1 + mid:1:3 + last:2:5: impossible (15s).
- m=3 first:1:2 + mid:1:3 + last:1:4: FOUND => n-bit adder in 2 + 3(n-2) + 4 = 3n gates.
  (Used in cand_01: the parity code; bit 0 passes AND(a0, b0), the top bit adds XOR of the three wires.)

## More joint results
- m=4 first:1:1 + mid:1:3 + last:1:5: FOUND (358s) => also 3n, no gain.
- m=4 first:2:3 + mid:1:3 + last:1:4: impossible (295s).
- m=3 first:2:3 + mid:1:3 + last:1:4: impossible (2s).
- m=3 first:1:1 + mid:1:3 + last:2:6: impossible (356s).

## Fixed codes (fixed.py / multi.py)
Code names: par3 = carry is the parity of 3 wires, all 8 patterns; chain4 = parity of (s, AND, OR) with only
the 4 patterns the chain block makes (000, 001, 101, 111 as s,AND,OR); abs = wires (a, b, s) of the bit
below, carry = maj(a, b, not s).
- 1 bit par3 -> par3, 3 gates: found. chain4 -> chain4, 3: found. par3 -> plain, 4: found.
- 1 bit abs -> abs 3: found; abs -> abs 2, abs -> chain4 2, abs -> par3 2, abs -> any 3-wire code 2: impossible.
- 1 bit abs -> chain4 3: impossible; abs -> par3 3: impossible; abs -> any 3-wire code 3: found (abs itself).
- 1 bit none -> abs 1 gate: found.
- 1 bit abs -> plain 4: impossible; 5: impossible (15s); 6: found (80s).
- 2 bits chain4 -> chain4 5: impossible (12s). chain4 -> par3 5: impossible (139s). par3 -> par3 5:
  impossible (120s). par3 -> par4 5: impossible (1087s).
- 2 bits chain4 -> any 3-wire code, 5 gates: FOUND (5s); the code it picks is abs (wires b1, s1, a1).
  So 2.5 gates/bit is possible for one step, but:
- 2 bits abs -> chain4 5, abs -> par3 5, abs -> abs 5, abs -> any 3-wire 5, abs -> any 4-wire 5: all
  impossible (5-30s each). 2 bits none -> chain4 4: impossible.
- multi 2:chain4:Y3:5 + 1:Y3:chain4:3 (some 3-wire code Y: 8 gates per 3 bits): impossible (1088s).
- 2 bits chain4 -> plain 6: impossible (788s).

- multi 2:chain4:Y3:5 + 2:Y3:chain4:5 (alternating 2-bit blocks, 2.5/bit): impossible (2066s).

## Stopped without an answer (gave up; each ran 1.6 to 2.1 hours of wall time on a loaded machine,
## within budgets of 200M to 2000M conflicts that were not reached)
- 2 bits, symbolic c3 same (repeatable 2-bit block, 3 wires), 5 gates.
- 2 bits, symbolic c4 same, 5 gates.
- 3 bits, symbolic c3 same, 8 gates; symbolic c4 same, 8 gates.
- 3 bits chain4 -> chain4, 8 gates; par3 -> par3, 8 gates; chain4 -> plain, 9 gates.
- multi 2:chain4:Y4:5 + 1:Y4:chain4:3 (8 per 3 bits with a 4-wire code Y).
- multi 2:par3:Y3:5 + 2:Y3:par3:5.
- joint m=3 first:1:2 + mid:1:3 + last:2:6 (would give 3n - 1).
- whole adders, plain: 3 bits no carry in, 8 gates (is 9 = 3n minimal at n=3?); 4 bits, 11 gates.
- Killed early as no longer relevant: 2 bits symbolic c2 same 6; 2 bits abs -> plain 7.

## Summary
- Proven: a repeatable 1-bit block needs 3 gates with any carry code of up to 4 wires (2 is impossible for
  m = 2, 3, 4), and with a plain carry it needs 4. A repeatable 2-bit block in 5 gates is impossible with a
  2-wire code, and with the parity codes par3 / chain4, and for the alternating chain4 <-> 3-wire-code pair.
  It is still open (timed out) for fully free 3- and 4-wire codes.
- Ends: the best found is first bit 2 + last bit 4 (or first 1 + last 5 at m = 4), both 3n in total. Every
  finished search for a version that saves one gate at the ends came back impossible: first 1 + last 4
  (m = 2, 3, 4), first 1 + last 5 (m = 3), first 2-bit 3 + last 4 (m = 3, 4), first 1 + last 2-bit 5 or 6
  (m = 3). first 2 + last 2-bit 6 (m = 3) timed out.

## Program results
- cand_01.py: no-limit adder = parity-code chain, 3n gates (12 / 24 / 36). Total 1086.
- cand_02.py: also use the chain for limited cases when the limit >= 2n (its depth): n=4 limit 8 -> 12 gates.
  Total 1080. Checked with no limit at n = 5, 7, 9, 16: all pass, 15 / 21 / 27 / 48 gates (3n).
