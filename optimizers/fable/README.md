# fable

byggmesterPRO's search setup: Claude (Fable) writing and steering classic logic-synthesis searches. Three tools,
each good at one thing. All of them read and write the netlist format of `tools/bygg-logic-sim`.

| Folder | Good at | Needs |
|---|---|---|
| `resub/` | Fewest gates. Rewrites an existing circuit gate by gate. | Python |
| `esop/` | Fewest ticks. Lookup-table circuits, 4 ticks, up to 12 input bits. | Python, numpy, Rust |
| `sat/` | Proving what a small block can and cannot do. | Python, python-sat |

Run output goes in a `runs/` folder next to each tool (git-ignored).

## resub: fewest gates

```
python optimizers/fable/resub/seeds.py optimizers/fable/resub/runs/seeds
python optimizers/fable/resub/many.py double-dabble 12bit 200 optimizers/fable/resub/runs/r1 <start.json> ...
python optimizers/fable/resub/resub.py double-dabble 12bit <start.json> <out.json> [seed] [seconds]
```

Start from any correct netlist: the family generator's output, or the division layouts from `seeds.py`
(12bit only). For each gate it works out on which inputs the gate's value matters at all, then tries to rebuild
it from signals that already exist, using one to three new gates. Wide XORs are found with linear algebra.
A second move swaps a signal for a different one and rewrites the gates that read it. `many.py` runs it on
every core; feed the best results back in as the next starts until nothing improves.

It stops at 16 input bits because it tries every input. It ignores ticks, so results are slow.

Double dabble 12bit, gates without the input gates: 118 (chain of 7-gate cells) to 93 in two minutes, then 89
after restarting from the best results. More time on 18 cores did not get below 89.

## esop: fewest ticks

```
python optimizers/fable/esop/esop_tool.py start double-dabble 12bit <seed> 20000 <name>
rustc -C opt-level=3 -o optimizers/fable/esop/esop.exe optimizers/fable/esop/esop.rs
optimizers/fable/esop/esop.exe runs/<name>.esop runs/<name>.res <threads> <seconds>
python optimizers/fable/esop/esop_tool.py build double-dabble 12bit runs/<name>.res runs/<name>.rows.json <out.json>
```

(`runs/` here is `optimizers/fable/esop/runs/`.)

Every output bit is an XOR of terms, and each term is one AND or NOR gate over the inputs. `start` first
re-codes the inputs with a layer of XOR gates, which makes the table much smaller, and writes a first set of
terms. `esop` then anneals over rewrites of pairs of terms (exorlink), far more steps than a Python search can
take. `build` checks the result against the table and writes the netlist.

The re-coding matters more than anything else: six seeds ended between 316 and 403 on the same problem. Try
several seeds, anneal each for a few minutes, keep the best.

Double dabble 12bit: 327 gates at 4 ticks (315 without the input gates). The Python search this replaced
reached 451 without the input gates.

## sat: exact answers for small blocks

```
python optimizers/fable/sat/cell_search.py <wires> <gates>
```

`synth2.py` encodes "is there a circuit of k gates that does this" for a SAT solver, for several small circuits
at once that may share an unknown signal encoding. `cell_search.py` uses it on one double dabble cell.
Results: a cell in plain binary needs 7 gates; with a free 3-wire encoding 6 gates are enough and 5 are not.
It gets slow above about 8 gates.
