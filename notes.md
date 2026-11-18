# Create Crafts & Additions — port notes (slice 1: the FE half)

Ported from **Create Crafts & Additions 1.7.2** (`MRH0/createaddition`, **MIT**),
the exact ATM10 jar `createaddition-1.7.2.jar` (SHA-256
`71e9c3ac39089e72c1e47a129a44e41eb55efc9534eecfdead8da8fb64313414`, 1,124
entries, kept at `ports/_shared/licence-cache/`). Upstream behaviour was read
from the public repository (`github.com/mrh0/createaddition`, commit
`658a390b8ff4daa17013e9447b29949330c8a42a`; analysis only — this is a clean-room
rewrite, see `FINDINGS.md`).

**LOCAL-ONLY** until the maintainer approves publishing (ARCHITECTURE.md §8).
MIT is licence rank 0 (permissive); the approval is still required.

Status: **slice 1 implemented and delivered as a flat SDK package (mod API 0.2),
version 0.2.0** — the two connectors, wire links and the modular accumulator, with the jar's
rates and connection rules, in one `createaddition.hmod` that carries the code *and* the
content. The 0.1.0 `.hnmod` was retired by Horizon's mod-API merge (#16941); the migration is
[`## Migration to the SDK format`](#migration-to-the-sdk-format), and the 0.1.0 in-game evidence
stays historical because no launcher exists at this pin (`## Gaps`).

## Class + tier, approach, licence

- **Class `content`, tier `deep`, add-on** (`ports/ledger.csv`, from
  `ports/_shared/source-map/atm10-sources.csv`). The jar's
  `neoforge.mods.toml` says `license="MIT"` → **rank 0 (permissive)**; it needs
  `neoforge`, `create` and `minecraft`, and `jei`/`computercraft`/`simulated`
  are **optional** (lines 56–67), so the block that made it look queued
  (`digital_adapter`, the CC bridge) is not needed by this slice.
- **Why this mod now:** rank 0, `content`, workable — the parent Create is
  provided upstream (`not-needed`), so the queue's add-on rule is satisfied.
  It is the first Create add-on whose slice can stand alone without kinetics.
- **Approach** (`horizon/docs/modpacks.md`): the tier is `deep` because the mod
  normally bridges Create's kinetic network and FE. That bridge **cannot be
  built**: the parent is a separate native mod, and a package has no dependency
  field, no published crate and no plugin registry
  (`ports/_shared/external/create-horizon-architecture.md` §0). The only
  incidental route — naming another mod's Rust module — is forbidden, and
  `if_installed!` needs an *optional* dependency the registry cannot express for
  a mod that is not a crate. What *is* supported is world-state interop
  (`LevelRef::capability` reads a neighbour's exposed `ENERGY`), so rotation may
  be **consumed**, never produced or joined — and this slice does not need it at
  all.
  The slice is thus the **FE half**: a self-contained electric network with the
  jar's own rates, `machines`-shaped inside a `deep` row.
- **Carry-over vs agent port:** models, textures, 26.3 item definitions, loot
  tables and lang are carried from the jar (`tools/carry_content.py`, decisions
  in [`CONTENT.md`](CONTENT.md)); the block entity, the wire links, the network
  pass, the commands and the runtime are the agent port.
- **The row is not re-tiered.** The rotation content of the `deep` tier is a
  declared gap, below.

## What the slice does

- **`createaddition:connector`** (small) and **`createaddition:large_connector`**
  — a block entity whose window is `min(maxIn, maxOut)` = 1,000 FE (small) or
  5,000 FE (large), declared through `Registrar::block_energy` as
  `Capabilities.Energy.BLOCK`. Placed against a face, `facing` points at the
  block it is mounted on.
- **Placement mode**, upstream's `ConnectorMode.test` on the first pass after it
  is placed: the mounted block can give → **Pull**, it can only take → **Push**,
  else **None**. Stored in the compound (`mode`) and shown in the block's `mode`
  property, so the jar's `pull`/`push`/`none` models follow it.
  `/createaddition mode <x y z> <mode>` sets it (upstream: Create's wrench,
  which is not installed — a gap below).
- **Wire links**, upstream's `WireSpool.useOn` rules: a spool's first right
  click selects a free node, the second lays the wire, the empty spool takes
  every wire of the clicked connector out. Refusals: `LONG` (the **shorter**
  of the two connectors' 16/32 blocks, on squared block distance), `COUNT`
  (4/6 nodes), `EXISTS` (one wire per pair), `INVALID` (itself) and
  `REQUIRES_HIGH_CURRENT` (copper between two large connectors). In survival a
  link costs one spool and returns the empty one.
- **The network pass**, after the level's tick: every connector that ticked is
  read, grouped into wire networks from its saved links (union-find), and each
  network with two or more members runs upstream's cycle — swap the double
  buffer, then push into every **push** connector's block, then pull from every
  **pull** connector's block. The limits are the jar's: 1,000/5,000 FE a tick,
  the buffer `min(80,000, members * (3 * demand + 10))`, and a one-tick arrival
  delay from the double buffer.
- **`createaddition:modular_accumulator`** — 2,000,000 FE, 5,000 FE/t in and
  out, on every side as a capability, and it feeds the blocks on its **UP** and
  **DOWN** faces each tick (`tickOutput`), which is what a single block of the
  jar's tower does (both of its `top`/`bottom` bits set).
- **Commands** — the harness's and an admin's interface, because the slice has
  no GUI: `/createaddition info|energy|mode|link|unlink`, each reply optionally
  tagged (`#tag`) so a harness can tell answers apart.

## How the energy is booked

Upstream keeps one `EnergyNetwork` per wire graph **in memory**, rebuilt on
load. This port books the same buffer in the saved compound of the network's
**lowest-positioned connector** — `netReady`/`netNext` and the last tick's
`netDemand` — so a reload cannot lose it. It is the same double-buffered
account (this tick's intake is next tick's supply), and every move goes through
the engine's `ENERGY` capability (`EnergyHandler::insert`/`extract`, and
`move_energy` for the accumulator): the runtime's conservation test and the
0.1.0 in-game check both show source + sink + buffer constant across a burst.
A block's own charge is still the compound's `energy` int, which is where the
SDK's `ModEnergyBuffer` documents it, so 0.1.0's readout and the retired saves
agree.

## Deviations (measured, not guessed)

1. **The connector's capability is reachable from every side.** Upstream
   exposes it only on the mounted face (`getEnergyStorage(direction)`), but
   `ModEnergyBuffer` declares faces on the block type, not per block state, so
   `insert_faces` and `extract_faces` are all-true. Nothing in the slice depends
   on the passive path; a block pushing into a connector fills its small window,
   which is spent by the network on the next pull.
2. **The network buffer's ceiling is computed on the total**, upstream on the
   intake half (`getMaxBuff() - inBuff`); the port's is stricter by at most one
   half-buffer.
3. **The wire's `transfer` field is not enforced.** `WireType` (copper 256, gold
   1024, electrum 8196) is **read by nothing** in 1.7.2 — the connector's limit
   is the throughput — so the port matches the jar and records the dead field.
4. **Node choice.** Upstream picks the node nearest the click
   (`getAvailableNode(clickLocation)`); `use_item_on` gives the face, not the
   click point, so a link takes the **lowest free node** (the layout differs,
   the count 4/6 is enforced).
5. **A removed wire leaves its buffer in the controller.** Upstream discards the
   graph's buffer with the graph; here the FE stays in the controller's compound
   (conserved, spendable when the network is re-linked), which the 0.1.0 control
   leg shows (`pool 0/1000` while nothing moves).
6. **Mode is set by a command, not Create's wrench** (not installed).
7. **The accumulator is one block.** The multiblock (up to 5 high × 3 wide,
   `getTotalAccumulatorSize()`) is a gap; the block behaves as a single block of
   the tower (2,000,000 FE, 5,000/t, feeds UP and DOWN). A consequence, measured
   in the 0.1.0 e2e: two **stacked single** accumulators trade energy both ways
   and settle at zero in the upper one — upstream would have merged them into
   one controller, which is exactly the missing multiblock.
8. **A foreign block's mode is read as "it can give" (0.2.0).** The SDK's
   `EnergyHandler` has `amount`/`capacity`/`insert`/`extract` and **no**
   `can_insert`/`can_extract`; the retired pin's `EnergyInfo` had both. So a
   connector placed against another mod's energy block settles **Pull** (the
   capability answers at that side ⇒ treated as a source), where upstream would
   have settled **Push** for a fill-only block. Our own blocks are settled
   exactly from their declared buffer (both directions, every face ⇒ Pull), and
   `/createaddition mode` is the escape hatch that already existed for Create's
   wrench. Recorded, not faked.

## Upstream defects fixed

Checked against the ruling of 2026-11-11 (AGENTS.md), evidence from the jar and
1.7.2 sources:

1. **The unreachable `passive` connector mode.** `ConnectorMode` declares
   `Passive` and the jar's blockstates ship `passive` models for both
   connectors (`block/connector/small/passive.json`, `.../large/passive.json`),
   but `getNext()` cycles `None → Pull → Push → None` — the wrench can never
   reach `Passive`, so those models can never be shown. **Fixed**: the port
   declares `mode` as `none|pull|push` only; the dead model is not carried, so
   the blockstate cannot promise a state the player can never set.
2. **Stale duplicate assets.** `blockstates/connector_old.json` and
   `blockstates/accumulator.json` describe block ids the mod no longer
   registers (`accumulator`, and the pre-1.7 connector layout). **Fixed**: not
   carried (a content file naming an unregistered block is dead weight; nothing
   reads it).
3. **`WireType.transfer` is dead.** Copper 256, gold 1024, electrum 8196 are
   declared and **read by nothing** in 1.7.2 — the connector's own 1,000/5,000
   is the throughput. This is not a broken behaviour, so it is kept as a
   **deviation** (the port does not invent a wire limit the jar does not
   apply): `FINDINGS.md` records it and the e2e's rate assertion pins the
   connector limit.
4. **Recipes that can never match.** Every upstream recipe for these blocks
   needs `create:` items or `c:` tags the parent provides. A post-1.21.1
   content pack cannot make them match, so they are **not carried** (see the
   first entry of `## Defects fixed in the migration` for the 0.2.0 response).
   Fixing this "minimally" would mean substituting vanilla items — inventing a
   recipe the mod does not have — which the ruling forbids, and the mod is an
   add-on whose recipes are supposed to need its parent. The craftable path
   returns with the parent Create port; until then the blocks are creative-only,
   and that is stated in `README.md`.

A user would notice (1) and (2) not at all — they cannot reach those states —
and (3) and (4) only as "no crafting recipe yet".

## Defects fixed in the migration (0.1.0's own, not upstream's)

The ruling above is about the Java mod; a migration must not carry **our** bugs either.

1. **One wire spool returned two empty spools.** 0.1.0's `use_item_on` handled the last wire
   spool by putting an empty spool in the hand *and* running `native_give_items(SPOOL, 1)`
   unconditionally, so a spool of one laid a wire and produced two empty spools. Upstream
   "consumes one spool and gives the empty spool back" (`FINDINGS.md`, `item/WireSpool.java`),
   which is what the other branch already did. **Fixed** as the pure decision
   `spool_after_link(count, creative)` — hand *OneFewer* + one returned for `count > 1`, hand
   *EmptySpool* + none returned for `count == 1` — unit-tested by
   `the_last_wire_spool_becomes_the_empty_spool_and_is_not_given_twice`. The fail-capable
   in-game proof is blocked with the rest of the scenario (`## Gaps`).
2. **A mixed-wire connector handed back the wrong spools.** 0.1.0's empty spool counted a
   connector's wires and gave back that many **wire spools of the first wire's kind**
   (`native_give_items(wire_item(wires[0].wire), spools.max(1))`), so a connector holding copper
   *and* gold returned gold spools for both. The jar coils each wire back onto its own spool —
   which is why the empty spool exists. **Fixed** as `spools_back(wires)` (one spool per wire, of
   that wire's own kind), unit-tested by
   `an_empty_spool_hands_back_one_spool_of_each_wires_own_kind`. Same blocked proof.

## Gaps (the rest of the mod, and the verification that cannot run)

- **The in-game e2e has not been run against this package.** `cargo horizon-mod build` packs;
  the native compile is `horizon-launcher native-build … --trust-native-code`, and
  `horizon-launcher` is **not installed** in this environment, so no `0b14f8a` game exists
  here. `tools/e2e.sh` is updated for a launcher-built game (`GAME=`, optional
  `CONTROL_GAME=`, `SELFTEST=1` kept) and its assertions are unchanged; the 0.1.0 runs in
  `ports/_shared/evidence/createaddition/e2e-*.log` are **historical** (retired format). A
  claimed e2e without a launcher would be a rejected report, so this is recorded instead.
- **The rotation half**: `electric_motor`, `alternator`, `rolling_mill`,
  `tesla_coil`, `servo_motor`, and the `charging/` recipe type (40+ oxidisation
  and charging recipes). Blocked by the missing mod-to-mod API: a rotation
  **consumer** could read a neighbour's published `Speed`/`Capacity`/`Stress`
  with `ENERGY`-style capability reads, but nothing here needs it, and producing
  or joining Create's network is impossible without forking its engine.
- **`small_light_connector`** (its 1 FE/t lamp consumption).
- **`creative_energy`**, the e2e's source of last resort: the harness seeds an
  accumulator with `/createaddition energy` instead.
- **`portable_energy_interface`**, **`redstone_relay`**, **`energy_meter`**,
  **`electric_pump`**, **`digital_adapter`** (the ComputerCraft bridge — CC is
  an optional dependency and not ported).
- **GUI/tooltips**: no goggles-style readout, no mode wrench, no comparator
  output on the accumulator, no `PhysicsBlock`/display source for the gauge.
- **Crafting**: no recipes (`CONTENT.md`) — the blocks are creative-only until
  the parent's items exist.
- **Festive wire and spool**, the iron wire, and the electrum/brass/copper metal
  items.
- **Contraption/Sable integration** (wires that follow a moving contraption).
- **A 0.1.0 world's block entities do not survive the migration.** 0.1.0's entities were typed
  by their **block id** (the `block_entity` hook returned `(block_id, Compound)`); 0.2.0
  declares one type, `createaddition:energy`, for all three blocks. A world saved by 0.1.0
  therefore loads its blocks but not their entities: modes, wire links and charges are gone and
  the connectors stop ticking. It cannot matter in practice — 0.1.0 was **never published**
  (PR #824 was closed by the format purge) — and it is recorded because a re-typed block entity
  is a save-visible change.
- **No logger.** A package has no `log` crate and no diagnostics channel, so the retired
  `log::warn!` on an unwritable `.horizon-cache` is gone with the embedded-content trick: the
  engine's own refusals are the only report, plus the readout command.

## Verification (evidence in `ports/_shared/evidence/createaddition/`)

**0.2.0 (this package) — the ladder, all inside one build slot, `CARGO_INCREMENTAL=0`:**

| Step | Command | Result |
|---|---|---|
| runtime carried | `cp` + `cmp` + `sha256sum` | byte-identical, `03721ca4…f962`; packaged `src/runtime.rs` has the same sha256 |
| runtime tests, **before any glue existed** | `cargo test --offline` on the skeleton (`src/lib.rs` = `pub mod runtime;`) | **30 passed**, 0 failed (that run's log was overwritten by the final one; the fact is the ladder's, and `sdk-glue-red.log` is the run right after, with the glue stubs in place) |
| glue first red | same | **30 passed, 10 failed** on `todo!()` / the assertion diffs, exit 101 (`sdk-glue-red.log`) |
| tests | `cargo test --offline` | **41 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `target/horizon-mod/createaddition.hmod`, exit 0 (`sdk-build.log`) |
| package, second build | same again | **byte-identical** to the first (`cmp`) (`sdk-build-second.log`) |
| registry check | `cargo horizon-mod check createaddition.hmod` | exit 0 (`sdk-check.log`) |
| fits | `cargo horizon-mod fits createaddition.hmod --build nightly` | `0b14f8a made for`, exit 0 (`sdk-fits.log`) |
| carried content | `python3 tools/carry_content.py --check` | `assets/ and data/ match the jar carry`, exit 0 |

Deliverable: **`createaddition.hmod`, 119,430 B, SHA-256
`28aee5dcc361ed04dddbdb4a7a5743165f47fe9b7fe4761124222fe94afa1c8b`**, 78 entries
(`horizon-mod.toml`, `Cargo.toml`, `README.md`, `LICENSE`, `LICENSE-NOTICE.md`, `src/lib.rs`,
`src/network.rs`, `src/runtime.rs`, `src/rustfmt.toml`, `uses.json`, 55 `assets/` and 13
`data/` files), reproducible: a second build gives the same bytes. `uses.json` has **363
entries** — 27 `hook:` (10 hook names + their fields), 37 `method:`, **2 capabilities**
(`horizon:energy`, `registries`), 4 `part:`, 3 `macro:`, 173 `type:`, 89 `handle:`, 27
`trait:`, 1 `fn:move_energy` (`sdk-uses-from-package.json`, `sdk-uses-summary.log`).

**0.1.0 (historical, retired `.hnmod`, kept because they are real in-game runs):**

- `cargo fmt --check` exit 0 (`rustfmt.toml`: `tab_spaces = 2`), `cargo clippy --all-targets
  -- -D warnings` exit 0, `cargo test` **30 passed, 0 failed**.
- Ladder through the registry at the old pin (`horizon.py stage → check → test`):
  `cargo_exit 0`, **0 warnings**, embedded tests **30 passed, 0 failed**
  (`game::createaddition::tests`, 1240 filtered out).
- `tools/e2e.sh` default leg: **PASS + EXIT:0** (`e2e-default.log`) — the blocks register, the
  connector places `pull` by itself, 100,000 FE seeded into the source reaches the sink at
  1,000 FE/t with the total conserved at 100,000, the unlinked control moves nothing, a 17-block
  wire is refused, copper between two large connectors is refused (gold accepted), and an
  accumulator fills the 1,000 FE window of the block above it (10,000 → 9,000 with the window
  0 → 1,000, conserved) after a control where both empty stay empty; and the unreachable
  `passive` mode is refused while the connector keeps the mode it had.
- `EMBEDDED=1` leg: **PASS + EXIT:0** (`e2e-embedded.log`) — no `--dev-mod`, the bundle's
  embedded content byte-identical to `content/createaddition.hmod`, and the control run
  (`.horizon-cache` unusable) has neither the item nor the block. **That leg no longer exists**:
  one package carries the content.
- `SELFTEST=1` leg: **FAIL + EXIT:1** (`e2e-selftest.log`) — the flipped conservation assertion
  trips, so the scenario can fail.
- Mutant check (`red-leg.log`): the pool's swap reverted to "drop the undrained half" fails the
  new test; the jar-correct swap passes. That bug was real — the first in-game run lost 60 FE —
  found by the conservation assertion.
- `python3 ports/tools/check-e2e-logs.py createaddition --strict` exit 0 (3 logs: PASS/EXIT:0,
  PASS/EXIT:0, FAIL/EXIT:1) — still true after the migration, since those logs are untouched.

---

## Migration to the SDK format

*Written 2026-11-18 from this port; the recipe is `ports/imfast/notes.md` → `## Migration to the
SDK format`, with `ports/enderstorage/notes.md` (block entities, a container, capabilities) and
`ports/laserio/notes.md` (a tick queue, a per-level pass, typed commands) as the closest
analogues.*

**Why.** Horizon's mod-API merge (#16941, launcher ≥ 0.2.70, PR #17992) retired the `.hnmod`
container; the registry closed every `.hnmod` submission on 2026-11-18 with one reason. The unit
is now **one flat `.hmod`**: `horizon-mod.toml` (`api = "0.2"`) + `Cargo.toml` + `src/lib.rs`
(`impl mod_api::Hooks`) + `assets/` + `data/`. **0.2.0** is the same behaviour republished that
way; it supersedes the closed **PR #824**.

Pins: SDK nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (`package_format` 6, `content_api`
"0.2", toolchain rust 1.99.1), mixins **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, launcher
0.2.78 (**not installed here**), `cargo-horizon-mod` from `horizon-rust/horizon@main`. The SDK is
cached in-tree at `../_shared/sdk/0b14f8a/` and copied into `.horizon/sdk/` (gitignored, never
packed).

### Files moved (before the first build)

The tool refuses a root holding a non-package path (`unexpected package path: …`), so the retired
tree was **moved**, not copied, into
`ports/_shared/evidence/createaddition/retired-hnmod-0.1.0/`:

| 0.1.0 path | Where it is now |
|---|---|
| `mixins/createaddition/` (`horizon-mixin.toml` + 10 hook helpers + `support.rs` + `runtime.rs` + the embedded content `.hmod`) | `retired-hnmod-0.1.0/mixins/createaddition/` |
| `createaddition.hnmod` (92,897 B, embedded the content) | `retired-hnmod-0.1.0/createaddition.hnmod` |
| `compatibility.json`, `rust-toolchain.toml`, `Cargo.lock` | `retired-hnmod-0.1.0/` |
| `content/` (its own `horizon-mod.toml`, `class` old, `createaddition.hmod`, `assets/`, `data/`) | `retired-hnmod-0.1.0/content-old/` |
| `tools/horizon.py`, `tools/package.py`, `tools/hmod-load.sh` | `retired-hnmod-0.1.0/tools/` |

**Kept:** `notes.md`, `FINDINGS.md`, `VALIDATION.md`, `CONTENT.md`, `LICENSE`,
`LICENSE-NOTICE.md`, `tools/e2e.sh`, `tools/carry_content.py`, `tools/parse_info.py`.

### File map (0.2.0)

| Path | What it is |
|---|---|
| `horizon-mod.toml` | the only manifest and identity: `id = "createaddition"`, `version = "0.2.0"`, `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]`, `[native] game/horizon/mixins` |
| `Cargo.toml` | one crate, `edition = "2024"`; its only dependency is `horizon-mod-api` from `.horizon/sdk/` |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks`: `register`, `level_tick_post`, `register_commands`, `run_mod_command` |
| `src/network.rs` | all the game glue: the block entity Part, the tick queue, the spool click, the per-level pass, the two command helpers and 10 unit tests |
| `src/runtime.rs` | the pure truth (`Connector`, `Wire`, `Mode`, `Pool`, the link and transfer rules), **byte-identical**, 30 tests |
| `src/rustfmt.toml` | `tab_spaces = 2` — see trap 6: the campaign style has to live beside the sources, because the packer refuses a root `rustfmt.toml` |
| `assets/createaddition/`, `data/createaddition/` | the 68 carried content files, moved from `content/`; **unchanged** (`diff -r` against `content-old/` is empty, and `tools/carry_content.py --check` re-derives them from the jar) |
| `README.md`, `LICENSE`, `LICENSE-NOTICE.md` | inside the package (the build packs them) |
| `createaddition.hmod` | the deliverable: one package, code + content |

### Old injection → new surface

| 0.1.0 injection (helper) | 0.2.0 |
|---|---|
| `embedded-content` (`content_root` HEAD, `native_data!` + `.horizon-cache`) | **gone** — `assets/` and `data/` are in the same package |
| `block-entity` (`block_entity`, HEAD, `Some((block_id, Compound))`) | `Hooks::register`: `registrar.block_entity_type("energy").blocks(&["connector","large_connector","modular_accumulator"]).build()`; a declared type starts with an empty compound |
| `block-entity-ticks` (15) | `Ticker`'s `BlockEntityPart::block_entity_ticks` → `event.ticks = true` |
| `block-entity-energy` (142) | `Registrar::block_energy` per block (`Kind::buffer()`), not a hook |
| `block-entity-tick` (17, wrote nothing) | `Ticker`'s `BlockEntityPart::block_entity_tick` → **queues** `(level, pos)` |
| `state-for-placement` (76) | `ConnectorShape`'s `BlockPart::state_for_placement` |
| `use-item-on` (13, global, `claimed`) | `ConnectorShape`'s `BlockPart::use_item_on` → `Option<InteractionResult>` (the pre-block, cancellable route; the item side's `use_on` fires *after* the block's) |
| `level-tick-post` (23, enumerated `game.world.block_entities`) | `Hooks::level_tick_post` + the tick **queue** (the SDK cannot enumerate a level's block entities) |
| `register-commands` (8) | `Hooks::register_commands` (`Command::new` ×5, now with `[tag:word]`) |
| `perform-command` (8, a split line) | `Hooks::run_mod_command` (`CommandInvocation`: typed `pos`/`pos2`/`amount`/`wire`/`mode`/`tag`) |
| `game.native_energy` / `native_energy_insert` / `_extract` / `native_move_energy` | `LevelRef::capability(host, &ENERGY, pos, Some(side))` → `EnergyHandler::{amount,capacity,insert,extract}`, inside `host.transaction(…)`; `move_energy` for the accumulator |
| `game.native_with_mod_block_entity` / `native_set_block_property` | `host.block_entity(level, pos)` → `BlockEntityRef::{data,merge_data}`; `host.set_block_property` (keeps the block entity) |
| `game.held()` / `held_mut()` / `hand_stack_replaced` / `native_give_items` | `ItemPlace::PlayerHand{player,hand}` → `amount`/`extract`/`exchange`/`edit`; `PlayerRef::give_stack` |
| `game.native_feedback(Text::literal(…))` | `host.commands().feedback(Text::literal(…))` |
| `static` state in `support.rs` | a `Mutex<Queue>` field on the mod struct (the macro boxes one `Default`) |

### What the wire did not need to change

`src/runtime.rs`, and therefore every rate, refusal, buffer rule and test: copied with `cp`,
proved with `cmp` and `sha256sum`, and re-proved inside the package with
`unzip -p createaddition.hmod src/runtime.rs | sha256sum`.

### Traps this migration hit (each cost a run; the ones the brief already lists are not repeated)

1. **A root `rustfmt.toml` refuses the package.** `error: unexpected package path: rustfmt.toml`
   — the root may hold only package paths (`*.md`, `LICENSE*`, `icon.png`, `Cargo.toml`,
   `horizon-mod.toml`, `src/`, `assets/`, `data/`). The campaign style is `tab_spaces = 2` and
   `src/runtime.rs` is formatted that way, so the file cannot simply go: without it
   `cargo fmt --check` fails on `runtime.rs` and `cargo fmt` would rewrite it. It moved to
   **`src/rustfmt.toml`** (rustfmt reads the config from the file's own directory upward), so the
   package has one 15-byte extra entry and the formatting stays reproducible.
2. **`build-slot.sh` forwards to the far box, whose cargo cache lacks the SDK's crates.**
   `error: no matching package named 'unicode_names2' found … offline mode` — the second machine
   resolves the API crate from its own registry cache. The whole ladder here ran with
   `BUILD_LOCAL=1`, which is a scheduling cost, not a port problem; the far box needs one online
   `cargo fetch` in `weights/.horizon/sdk/api` before it can take an SDK migration.
3. **A `tee` in the wrapped command masks cargo's exit status.** `cargo test … | tee log` exits
   with `tee`'s 0, so the slot released **exit 0 on a compile error**. Every step now runs under
   `bash -o pipefail -c`.
4. **`build-slot.sh run` from the campaign root cds to `ports/<id>` on the far box**, so the
   brief's `bash -c 'cd ports/createaddition && …'` fails there (`cd: No such file or directory`).
   Run the runner **from inside the port** (`../tools/build-slot.sh`) with the bare command: the
   far box mirrors the caller's relative directory, and the local run already starts there.
5. **The readout must not be re-derived from the new API.** It reads the compound's `energy`
   int — the amount the SDK's own `ModEnergyBuffer` documents — rather than asking the
   capability, so the text stays byte-identical to 0.1.0 and the scenario's assertions still
   match. The capability is only for **moves**.
6. **The queue is the pass's index.** 0.1.0 enumerated `game.world.block_entities`; a package
   cannot. `block_entity_tick` queues the position (and the level) and `level_tick_post` drains
   that level's set — and re-reads each block, because the queue can name a block removed since
   and a settled connector's compound is stale by then.
7. **`level_tick_post` is both sides.** `HostSide::Server(host)` is the only arm that acts;
   `LevelRef::capability` answers only for the level **in place**, so the pass returns early
   when `event.level.in_place(host)` is false and lets the next tick retry (the queue is bounded
   by our block count).
8. **A bail-out in a Part still has to answer.** 0.1.0 set `*claimed = true` before its second
   read, so a first connector removed between the two clicks still consumed the click; a `?` on
   that read would now hand the click to the block. Both reads return `Some(Success)` instead.
9. **`ItemPlace::edit` needs a transaction, and `PlayerHand` is the hand that clicked.** Both
   the pending selection and the spool spend run inside `host.transaction(…)`; `extract` on a
   creative player's hand moves the item without using it up, so the creative branch is decided
   by `PlayerRef::is_creative` **before** the move, not inferred from it.
10. **`stack.set_components` merges; there is no `set_other`.** The spool's pending selection is
    written as the whole `minecraft:custom_data` compound (read, edit our key, write back), so
    another mod's keys under it survive. Clearing leaves an empty compound behind, which nothing
    reads.
11. **The block entity type is a new identity, and that is save-visible.** One type
    (`createaddition:energy`) replaces "the block id is the type"; see the last entry of
    `## Gaps`.
12. **`ModEnergyBuffer` is a declaration, not the amount.** `Registrar::block_energy` declares
    capacity, per-call limits and faces; the charge stays in the compound's `energy`, so the
    `/createaddition energy` command is one `merge_data` and not a loop of inserts.
13. **`horizon-mod build` packs, it does not compile.** The external compile is
    `horizon-launcher native-build … --trust-native-code`, absent here; `fits` only answers "made
    for". So the compile gate is `cargo test` + `clippy -D warnings`, and the in-game leg is a
    recorded gap, not a claim.

### The changelog this version carries

> **0.2.0 — republished as a native SDK `.hmod` (mod API 0.2).** Supersedes the closed **#824**
> (the 0.1.0 `.hnmod`, retired with the container in Horizon's mod-API merge #16941). What
> changed: one flat package — `horizon-mod.toml` (`api = "0.2"`, `[native]` pinned to
> `0b14f8a`/`e59da90`) with the code in `src/` and the connectors' and accumulator's content in
> `assets/`/`data/` — so the embedded-content trick, its `.horizon-cache` write and its
> `content_root` injection are gone; the ten hook helpers and the plan are replaced by one
> `impl Hooks`, two `BlockPart`s, one `BlockEntityPart` and `Registrar` declarations; the level
> pass indexes a tick queue, because a package cannot enumerate a block entity, and every energy
> move now runs in a transaction through the `ENERGY` capability; `/createaddition` takes typed
> arguments and an optional `tag` word. The behaviour, the rates and `src/runtime.rs` are
> unchanged, and two of this port's own defects are fixed: the last wire spool no longer returns two
> empty spools, and the empty spool now hands back each wire on a spool of that wire's own kind.
> Not carried: the in-game run (no launcher at this pin).

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port
