# Create Crafts & Additions — Horizon port (slices 1–2: the FE half)

A native Horizon port of **Create Crafts & Additions 1.7.2** (`MRH0/createaddition`, **MIT**): the
electric half of the mod — two connectors, wire links and the modular accumulator, including the
jar's 3×3×5 accumulator multiblock — with the jar's own transfer rates.

**LOCAL-ONLY** until the maintainer approves publishing (ARCHITECTURE.md §8). MIT is licence rank 0
(permissive); the approval is still required.

The rotation half (`electric_motor`, `alternator`, `rolling_mill`, `tesla_coil`, `servo_motor`) is
**not** in this slice: the parent Create is a separate native mod with no mod-to-mod API, so those
blocks cannot join its kinetic network. They are a recorded gap, not an omission.

Behaviour reading: [`FINDINGS.md`](FINDINGS.md). Ladder, deviations, gaps, the hook table and the
migration: [`notes.md`](notes.md). Verification evidence:
[`VALIDATION.md`](VALIDATION.md) and `ports/_shared/evidence/createaddition/`. Content decisions:
[`CONTENT.md`](CONTENT.md).

## One package, code and content

This is a **flat SDK package** (mod API 0.2): `horizon-mod.toml` + `Cargo.toml` + `src/` + the
carried `assets/` and `data/`. One `createaddition.hmod` installs both — there is no separate content
file, no `.horizon-cache` write and no dev-mod list to manage (the retired 0.1.0 `.hnmod` embedded
the content that way; see `notes.md`).

Three blocks and seven items:

| Kind | Ids |
|---|---|
| Blocks | `createaddition:connector`, `createaddition:large_connector`, `createaddition:modular_accumulator` |
| Items | `createaddition:copper_wire`, `gold_wire`, `electrum_wire`, `spool`, `copper_spool`, `gold_spool`, `electrum_spool` |

Each block drops itself when mined with a pickaxe (its own 26.3 loot table, and the carried
`minecraft:mineable/pickaxe` tag); the three blocks are also creative-obtainable.

## What it does

- A connector is mounted on a face, like upstream's: it is placed against the block you click and
  its `facing` points at it (`BlockPart::state_for_placement`).
- Its **mode** is decided from that block on the first pass after it is placed, as upstream's
  `ConnectorMode.test` does: a block that can give is **pulled** from, one that can only take is
  **pushed** into. `/createaddition mode` sets it (upstream uses Create's wrench, which is not
  installed here).
- A **spool** lays a wire: right-click one connector, then another. The empty spool takes the wires
  out. The rules are the jar's: a small connector's wire reaches 16 blocks and a large one's 32 (the
  shorter of the two bounds the pair), each connector has 4 or 6 nodes, one wire per node, and
  copper is refused between two large connectors.
- Every tick after the level's own, each wire network moves FE: a pull connector draws up to
  **1,000 FE/t** (small) or **5,000 FE/t** (large) from its block into the network's buffer, and a
  push connector feeds its block from that buffer. The network buffer is double-buffered like
  upstream's `EnergyNetwork`, so energy arrives one tick after it is drawn, and it is capped by
  `min(80,000, members * (3 * demand + 10))`.
- The **modular accumulator** is 2,000,000 FE a block, 5,000 FE/t either way. Contiguous
  accumulators up to **3×3 wide and 5 high** form one tower (`CAConnectivityHandler`): its lowest
  corner is the controller, its capacity is `blocks * 2,000,000`, and **every block answers with the
  whole pool**, so a connector on any part drains the tower. The jar's `top`/`bottom` state bits
  decide which layer outputs above and which below (`tickOutput`), so a tower never feeds itself;
  a single block keeps both bits and feeds both ways.
- FE is moved through the engine's `ENERGY` capability (`LevelRef::capability` +
  `EnergyHandler::insert`/`extract`, and `move_energy` for the accumulator), never created: the
  runtime's own conservation test and the in-game check both prove source + sink + buffer is
  constant.

## Commands

`/createaddition info|energy|link|unlink|mode …` are the harness's readout and the controls the GUI
and Create's wrench would give a player. Every usage takes an optional trailing `tag:word`, which the
reply repeats as ` [#tag]` so a harness can tell answers apart. The readout text is byte-identical to
0.1.0's, so the scenario's existing assertions still match.

## Layout

```
horizon-mod.toml               the only manifest and identity: id, version 0.2.0, api 0.2, [native]
Cargo.toml                     one crate; its only dependency is horizon-mod-api from .horizon/sdk/
rustfmt.toml                   tab_spaces = 2, the campaign's style
src/lib.rs                     the one impl Hooks: register, level_tick_post, commands
src/network.rs                 game glue: the block entity Part, the wire links, the pass, commands
src/runtime.rs                 the pure rules (30 tests), byte-identical to the retired bundle's
assets/createaddition/         55 carried files: blockstates, models, textures, items, lang
data/createaddition/           13 carried files: block and item definitions, loot tables
createaddition.hmod            the deliverable: one file, code + content
tools/
  carry_content.py [--check]   jar -> assets/ and data/, with the patches documented
  parse_info.py                `/createaddition info` line -> shell fields
  e2e.sh                       the in-game scenario (PASS + EXIT:<status>)
notes.md, FINDINGS.md, VALIDATION.md, CONTENT.md, AGENTS.md
LICENSE, LICENSE-NOTICE.md
```

The retired 0.1.0 tree (the `.hnmod`, its `mixins/`, `compatibility.json`, `rust-toolchain.toml`
and the old `content/`) is evidence at
`ports/_shared/evidence/createaddition/retired-hnmod-0.1.0/`.

## Build and check

```sh
cd ports/createaddition
CARGO_INCREMENTAL=0 cargo test --offline                    # 41 tests
CARGO_INCREMENTAL=0 cargo clippy --all-targets --offline -- -D warnings
cargo fmt --check                                           # never --all: it formats the SDK
rm -f Cargo.lock && cargo horizon-mod build                  # -> target/horizon-mod/createaddition.hmod
cargo horizon-mod check createaddition.hmod
cargo horizon-mod fits createaddition.hmod --build nightly
```

Every step above runs inside a campaign build slot (`ports/tools/build-slot.sh run --job … --port
ports/createaddition --step …`) so the box-wide concurrency accounting keeps working; the port has no
staging step, because the SDK crate is compiled in place against `.horizon/sdk/`.

`cargo horizon-mod build` **packs**; it does not compile against the game. The native compile is
`horizon-launcher native-build . --instance <i> --trust-native-code`, which is not installed here —
so `tools/e2e.sh` needs a launcher-built game in `GAME=` and has not been run at this pin. It ends
with `EXIT:<status>`, which `python3 ports/tools/check-e2e-logs.py createaddition --strict` reads.
