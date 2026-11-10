# Pareto Circuits

AI-optimized logic circuits for Scrap Mechanic.

For every circuit we search for the best trade-offs between speed (ticks) and size (gate count), and publish three builds of it: the fastest, the smallest, and a balanced one. A circuit is Pareto optimal when no other circuit beats it on both at once, which is where the name comes from.

The core of the project is **generators**: functions that build a whole circuit family at any size from parameters. The blueprints in this repo and on the Steam Workshop are generated from them at the standard sizes (2, 4, 8, 16, 32 and 64 bit). Later the generators go into a mod, so anyone can make the size they need in game.

## Repo layout

```
pareto-circuits/
├── circuits/            One folder per circuit family (double-dabble, ram, adder ...)
│   └── <family>/
│       ├── README.md    What it does, pin layout, records table
│       ├── generator/   Code that builds this family at any size
│       └── builds/      Finished blueprints (see "Builds")
├── optimizers/          The search / AI systems, one folder per system
├── tools/               Shared scripts both systems use
├── mod/                 Scrap Mechanic mod that ships the generators
└── wip/                 Scratch space, no rules
```

Naming: everything is lowercase with hyphens. A family is named after what a player would search for (`double-dabble`, `ram`, `adder`), not after the trick used to build it.

## Scripts

| Where | What goes there |
|---|---|
| `circuits/<family>/generator/` | The generator for that family. Takes size and profile, outputs the circuit. How to run it is written in the family README. |
| `optimizers/<name>/` | One complete search system per folder (for example niknal's generator search, or the Fable + innovation.md setup). Free layout inside, with its own README on how to run it. |
| `tools/` | Anything both systems need: simulator, verifier, gate and tick counter, blueprint import/export. One job per script, named after the job. |
| `wip/` | Experiments and half-finished ideas. |

Rules:

- The shared verifier in `tools/` is the only judge of correctness, gates and ticks. A result counts once it passes there, no matter which system found it.
- Testing is the bottleneck, so speedups to the simulator go in `tools/` where both systems benefit.
- Run output (logs, checkpoints, simulation caches) goes in a `runs/` folder inside the optimizer and is git-ignored.
- No API keys or machine-specific paths in committed files.

For now each system's simulator and verifier sit in their own folder, `tools/bygg-logic-sim/` and
`tools/niknal-logic-sim/`. The best parts of both become `tools/logic-sim/` in Rust, which is then the shared
verifier.

The Rust tools build together: `cargo build --release` in the repo makes `logic-sim` (simulator and verifier)
and `pareto` (runs the generators and writes build slots) in `target/release/`.

## Builds

A build is one finished, verified blueprint. Each one lives in its own slot:

```
circuits/<family>/builds/<size>/<profile>/

circuits/double-dabble/builds/16bit/fast/
├── blueprint/
│   ├── blueprint.json
│   ├── description.json
│   └── icon.png
└── meta.json
```

**Size** is `2bit`, `4bit`, `8bit`, `16bit`, `32bit` or `64bit`. Other sizes are fine when someone needs them. Circuits with two parameters use `<a>x<b>bit` (for example RAM `16x8bit`), and the family README says which number is which.

**Profile** decides which build wins the slot:

| Profile | Wins by | Tie-break |
|---|---|---|
| `fast` | Fewest ticks | Fewer gates |
| `small` | Fewest gates | Fewer ticks |
| `balanced` | Lowest gates × ticks | Fewer ticks |

If the same build wins two profiles, save it in both slots.

### How we count

- **gates**: every logic gate in the blueprint, input and output gates included.
- **ticks**: worst-case ticks from an input changing until the outputs are correct. Each gate adds 1 tick, so this is the number of gates on the longest path from an input to an output, counting both ends.
- Circuits with memory (RAM, counters) define what `ticks` means in their family README, for example read delay.

### meta.json

Example values:

```json
{
  "family": "double-dabble",
  "size": "16bit",
  "profile": "fast",
  "version": "1.0",
  "gates": 315,
  "ticks": 3,
  "source": "generator",
  "verified": "exhaustive",
  "optimal": "best-known",
  "authors": ["niknal", "byggmesterPRO"],
  "date": "2026-11-10",
  "workshop": "",
  "notes": ""
}
```

- `source`: `generator` (made by the family generator), `search` (a one-off result for this size that beats the generator) or `manual`.
- `verified`: `exhaustive` (every input combination simulated) or `sampled` (random plus edge cases, for sizes too big to test fully).
- `optimal`: `best-known`, or `proven` if there is a proof that nothing smaller or faster exists.
- `workshop`: link to the Steam Workshop item once it is published.

### In-game name and description

Set these in `description.json`, and use the same title on the Workshop:

- Name: `Pareto <Family> <Size> <Profile>`, for example `Pareto Double Dabble 16bit Fast`. Searching "pareto double dabble 16 bit" should always find it.
- Description, first line: `v1.0 | 315 gates | 3 ticks`, followed by the repo link.
- Keep `localId` (and the Workshop `fileId` once it has one) unchanged between versions, so the Workshop item is updated in place and not duplicated.

`python tools/workshop/cards.py` writes both for every build, from `meta.json`: the description (what it does, the pins, the profile, how it works, and the link to the family) and `icon.png` (the same in a picture, with the build's own layout). Run it after writing a slot, then the build can be posted from the game as it is.

Each family README keeps a records table of its current builds:

| Size | Profile | Version | Gates | Ticks | Workshop |
|---|---|---|---|---|---|
| 16bit | fast | v1.0 | 315 | 3 | link |

## Versioning

Every slot (family + size + profile) has its own version, `v<major>.<minor>`, starting at `v1.0`.

| Bump | When | For someone already using it |
|---|---|---|
| Minor (`v1.0` to `v1.1`) | Fewer gates, or a fix. Same ticks, same pins. | Drop-in replacement |
| Major (`v1.3` to `v2.0`) | Ticks changed, or pins moved, reordered, added or removed. | Has to adjust timing or wiring |

Rules:

- A new build only replaces the current one if it wins by the profile's rule. Same numbers is not a new version.
- Never change a published blueprint without bumping its version.
- One slot is one folder. Old versions are not kept as folders, they live in git history.
- Tag every release as `<family>-<size>-<profile>-v<version>`, for example `double-dabble-16bit-fast-v1.1`.
- Commit message: `double-dabble 16bit fast v1.1: 404 -> 315 gates, 3 ticks`
- Generators are versioned by git only. When a generator improves, regenerate the builds it affects and bump each one that changed.

### Adding or updating a build

1. Generate or build the circuit.
2. Run the shared verifier to confirm it works and to get its gates and ticks.
3. Check that it beats the current build in the slot.
4. Replace `blueprint/`, then update `meta.json` and the records table in the family README.
5. Commit, tag, and update the Workshop item.

## Installing a blueprint

Subscribe on the Steam Workshop (links are in each family's records table), or copy the `blueprint/` folder into

```
%APPDATA%\Axolot Games\Scrap Mechanic\User\User_<steam id>\Blueprints\
```

as a new folder named after the `localId` in its `description.json`.

## Still to decide

- How input and output gates are marked in a blueprint (for example by color), so the verifier can find them and their bit order.
- The common format generators and optimizers hand to `tools/`: a blueprint directly, or a netlist that gets converted.
- How generators get into the mod. Mods run Lua, so generators written in another language need to be ported or pre-generated.
