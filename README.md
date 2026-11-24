# Create Crafts & Additions — Horizon port (slices 1–9: the FE half)

A native Horizon port of **Create Crafts & Additions 1.7.2** (`MRH0/createaddition`, **MIT**): the
electric half of the mod — two connectors, wire links and the modular accumulator, including the
jar's 3×3×5 accumulator multiblock and its comparator output, the redstone relay that bridges one
wire network into another while powered, the small light connector whose lamp draws one FE a
tick and lights while its network pays it, the creative generator that gives without limit, and
the jar's four wire kinds (the festive spool included) — with the jar's own transfer rates.

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

Six blocks and nine items:

| Kind | Ids |
|---|---|
| Blocks | `createaddition:connector`, `createaddition:large_connector`, `createaddition:small_light_connector`, `createaddition:modular_accumulator`, `createaddition:redstone_relay`, `createaddition:creative_energy` |
| Items | `createaddition:copper_wire`, `gold_wire`, `electrum_wire`, `iron_wire`, `spool`, `copper_spool`, `gold_spool`, `electrum_spool`, `festive_spool` |

Each block drops itself when mined with a pickaxe (its own 26.3 loot table, and the carried
`minecraft:mineable/pickaxe` tag); the six blocks are also creative-obtainable.

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
  **copper and the festive wire** are refused between two large connectors (the jar's low-current
  pair; gold and electrum are the high-current ones). The empty spool coils each wire back onto a
  spool of its own kind, so a festive link hands back a festive spool. **Breaking a connector drops
  its wires where it stood** — four of each wire's own kind, and a festive wire's four are copper
  wires, as the jar's `WireType.FESTIVE` says — while its peers let go of it and nothing drops to a
  creative player. `createaddition:iron_wire`
  is carried because 1.7.2 registers the item, but upstream comments out its wire kind and its
  spool, so it lays nothing here either.
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
- The accumulator **answers a comparator** with its tower's fill
  (`ComparatorUtil.fractionToRedstoneLevel(getFillState())`): 0 empty, **8** at half — the jar's
  `* 14 + 1` midpoint — and 15 full, so a comparator on any part of the tower reads the whole pool.
- The **redstone relay** has eight wire nodes in two ports (0–3 input, 4–7 output). Its input port
  joins one wire network and its output port another, and energy crosses only while the block's
  `powered` bit is set — a redstone block or wire on either side of its long axis sets it, at most
  **1,000 FE/t** through a 1,000 FE in-flight buffer. Unpowered, nothing crosses; the e2e measures
  the sink staying still for 20 ticks.
- The **small light connector** is a small connector with a lamp: every tick it asks its wire
  network for **1 FE** and keeps a counter that rises while it is paid and falls while it is not,
  clamped to `10 + (|x*31+y*45+z*33| % 7) * 3` ticks either way. At the top clamp its `powered` bit
  is set and it emits block light **15**; at the bottom clamp it goes dark (`dynamic_light`, answered
  from the state by the block's Part). It is a consumer, not a buffer: it declares the small
  connector's 1,000 FE window and no way to give energy back to its network.
- The **creative generator** is an unlimited FE source, as upstream's `CreativeEnergyStorage` is:
  its `ENERGY` capability answers `u32::MAX` stored and capacity, gives whatever is asked and
  takes nothing — a connector mounted on it settles `pull` and a wire carries 1,000 FE a tick.
  Every tick it also offers each of its six neighbours a full fill
  (`receiveEnergy(Integer.MAX_VALUE, false)`), so an accumulator beside it fills at that block's
  5,000 FE a tick with no connector at all.
- FE is moved through the engine's `ENERGY` capability (`LevelRef::capability` +
  `EnergyHandler::insert`/`extract`, and `move_energy` for the accumulator), never created: the
  runtime's own conservation test and the in-game check both prove source + sink + buffer is
  constant. The creative generator is the one block whose energy is deliberately not conserved —
  it is the source.
- The **client half** is a renderer in the frame's own wgpu pass: a laid wire is drawn as a ribbon
  on the jar's catenary in its `WireType` colour (a festive wire carries its string of lights), and
  the **accumulator's gauge needle** is drawn on all four horizontal faces at the jar's own
  `rotateXDegrees(-180 * fill)` — empty points one way, full the other. Both use one pipeline and
  the depth of the frame; a single-player or LAN-host client draws what its own level's pass read.

## Commands

`/createaddition info|energy|link|unlink|mode|signals …` are the harness's readout and the controls
the GUI and Create's wrench would give a player. Every usage takes an optional trailing `tag:word`,
which the reply repeats as ` [#tag]` so a harness can tell answers apart. The readout text is
byte-identical to 0.1.0's, so the scenario's existing assertions still match; `signals` is newer and
prints `answers=<n> dimensions=<dim>:<n>,… pending=<n>` — how many comparator answers the mod holds
per dimension and how many spool selections are in progress (`pending=none` never appears; the
dimension list is `none` when nothing is held).

An accumulator's comparator answer is kept only as long as the accumulator is: the block's removal
(`on_block_state_change`), its chunk's unload (`chunk_unload`) and its level's close (`level_unload`)
each forget it, and a spool's first click is dropped when its level closes or its player leaves.

A wire block whose support is broken is removed by the level the way upstream's `neighborChanged`
removes it: the supports are the face behind a connector's (or a vertical relay's) `facing` and the
block below a flat relay, the check runs in the level pass (`block_face_sturdy`, `SupportType.Center`),
and the removal drops the block's own loot plus four wires per wire it held, clearing the peer's
link. A connector placed with no support is upstream's `connector_ignore_face_check` default and is
not treated as an orphan; only a support that goes away takes its block.

## Layout

```
horizon-mod.toml               the only manifest and identity: id, version 0.2.14, api 0.2, [native]
Cargo.toml                     one crate; its only dependency is horizon-mod-api from .horizon/sdk/
src/rustfmt.toml               tab_spaces = 2, the campaign's style (a root one is refused)
src/lib.rs                     the one impl Hooks: register, level_tick_post, the three lifecycle clears, commands
src/network.rs                 game glue: the block entity Part, the wire links, the pass, commands
src/runtime.rs                 the pure rules (30 tests), byte-identical to the retired bundle's (the fourth wire kind is glue, `network::Laying`)
assets/createaddition/         92 files: blockstates, models, textures, items, lang
data/                          22 files (incl. the minecraft harvest tag): block and item definitions, loot tables
createaddition.hmod            the deliverable: one file, code + content
tools/
  carry_content.py [--check]   jar -> assets/ and data/, with the patches documented
  parse_info.py                `/createaddition info` line -> shell fields
  e2e.sh                       the in-game scenario (PASS + EXIT:<status>)
notes.md, FINDINGS.md, VALIDATION.md, CONTENT.md, AGENTS.md
LICENSE, LICENSE-UPSTREAM, LICENSE-NOTICE.md
```

`LICENSE-UPSTREAM` is the upstream MIT notice carried verbatim; `LICENSE` is this port's grant,
which keeps the upstream copyright line.

The retired 0.1.0 tree (the `.hnmod`, its `mixins/`, `compatibility.json`, `rust-toolchain.toml`
and the old `content/`) is evidence at
`ports/_shared/evidence/createaddition/retired-hnmod-0.1.0/`.

## Build and check

```sh
cd ports/createaddition
CARGO_INCREMENTAL=0 cargo test --offline                    # 84 tests
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
`horizon-launcher native-build . --instance <i> --trust-native-code`, which the campaign's
`ports/tools/launcher-game.sh` drives for the launcher-built game (`GAME=`) that `tools/e2e.sh`
runs against. Every run ends with `EXIT:<status>`, which `python3
ports/tools/check-e2e-logs.py createaddition createaddition-e2e --strict` reads.
