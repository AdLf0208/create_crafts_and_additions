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

Status: **slice 1 implemented and delivered as a flat SDK package (mod API 0.2), version
0.2.5** — the two connectors, wire links and the modular accumulator, with the jar's rates and
connection rules, plus the **redstone relay** that bridges one wire network into another while
powered and the **small light connector** whose lamp draws 1 FE a tick and lights while its network
pays it, in one `createaddition.hmod` that carries the code *and* the content. (0.2.0
was the migration; 0.2.2 the tower; 0.2.3 the three review fixes, §
[`## Review fixes — 0.2.3`](#review-fixes--023-pr-1526-2026-11-21); 0.2.4 the relay, §
[`## Slice 4 — the redstone relay (0.2.4)`](#slice-4--the-redstone-relay-024); 0.2.5 the lamp, §
[`## Slice 5 — the small light connector (0.2.5)`](#slice-5--the-small-light-connector-025).) The
0.1.0 `.hnmod` was retired by Horizon's mod-API merge (#16941); the migration is
[`## Migration to the SDK format`](#migration-to-the-sdk-format), and the 0.1.0 in-game evidence
stays historical (`## Gaps`).

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
7. **The accumulator was one block (0.2.0–0.2.1); 0.2.2 forms the tower.** The
   multiblock gap is closed: contiguous accumulators up to 3×3 wide and 5 high
   form one controller with `blocks * 2,000,000` FE, the jar's `top`/`bottom`
   bits, and every part answering with the pool — see
   [`## Slice 2 — the modular accumulator multiblock (0.2.2)`](#slice-2--the-modular-accumulator-multiblock-022).
   Two stacked singles no longer trade energy both ways: a formed tower's lowest
   block outputs only DOWN and its highest only UP (`tickOutput` gated by the
   jar's own state bits), which is the behaviour the 0.1.0 run measured wrong.

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

## Gaps (the rest of the mod, and what cannot run)

- **The in-game e2e now runs, at `0a875da` (0.2.2).** 0.2.0/0.2.1 could not run one: the native
  compile needs `horizon-launcher`, which was not installed then. It is now
  (`launcher-game.sh ensure`, launcher 0.2.92), and the whole scenario — A–H, including the
  accumulator tower, the drop leg and the control/selftest legs — passes in a launcher-built game;
  `ports/_shared/evidence/createaddition-e2e/e2e-tower.log` is that run, `…-control.log`,
  `…-selftest.log` and `…-loot-prefix.log` the legs that must fail.
- **The rotation half**: `electric_motor`, `alternator`, `rolling_mill`,
  `tesla_coil`, `servo_motor`, and the `charging/` recipe type (40+ oxidisation
  and charging recipes). Blocked by the missing mod-to-mod API: a rotation
  **consumer** could read a neighbour's published `Speed`/`Capacity`/`Stress`
  with `ENERGY`-style capability reads, but nothing here needs it, and producing
  or joining Create's network is impossible without forking its engine.
- ~~**`small_light_connector`** (its 1 FE/t lamp consumption)~~ — **closed in 0.2.5**: the
  lamp draws 1 FE a tick from its network, lights after its position-seeded hysteresis and emits
  block light 15 while lit, § [`## Slice 5`](#slice-5--the-small-light-connector-025).
- **`creative_energy`**, the e2e's source of last resort: the harness seeds an
  accumulator with `/createaddition energy` instead.
- **`portable_energy_interface`**, **`energy_meter`**,
  **`electric_pump`**, **`digital_adapter`** (the ComputerCraft bridge — CC is
  an optional dependency and not ported). (`redstone_relay` moved out of this list in 0.2.4.)
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
| tests | `cargo test --offline` | **41 passed**, 0 failed (`sdk-test-0.2.0.log`; `sdk-test.log` carries 0.2.2's 53) |
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

## Review fixes (2026-11-18, PR #1244 — held)

The reviewer held this PR on three package findings (and two entry-only ones):

1. **Every command was level 0.** `energy`, `mode`, `link` and `unlink` wrote to arbitrary loaded
   coordinates in the caller's level: remote tampering, free stored FE, and wire links without a
   spool. **Fix:** `energy`, `link` and `unlink` are `Command::op(…, 2)`; `info` and `mode` remain
   player controls at level 0 but require the acting player to be within six blocks of the block
   (`within_reach`/`has_access`), while the console and command blocks keep acting anywhere.
2. **Wire removal refunded without reciprocal state.** `remove_wires` removed each side
   independently, refunded every wire on the clicked connector even when the peer was unloaded or
   its write failed, and left the peer's endpoint as a live edge that could be refunded again.
   **Fix:** `reciprocal_wires` selects only edges the peer also records; the peer's edit is
   committed and re-read first, then this connector's, and a spool is returned only for a wire
   confirmed gone from **both** saved lists. A failed or unloaded peer leaves the edge and refunds
   nothing.
3. **Entry (not the package):** the upstream link and the MRH0 credit are in
   `mods/createaddition.toml`, which the orchestrator owns; this package's `horizon-mod.toml`
   already lists `MRH0 (upstream)` and has no relative link. Left for the entry fix.
- **Harness fix found in the run.** The optional `tag` argument was `word`, which the dispatcher
  refuses for the `#q1` markers the e2e sends (`Expected whitespace to end one argument`); it is a
  trailing `text` now.
- **Tests.** `the_mutating_commands_are_operator_only_and_the_controls_are_not`,
  `a_block_beyond_reach_is_not_a_player_control`, `a_wire_is_removed_only_where_the_peer_records_it`.
- **In-game.** All command phases pass (energy, wire, unlink, limits, accumulator); evidence
  `../_shared/evidence/createaddition-e2e/e2e-launcher.log` (`PASS` + `EXIT:0`) and
  `e2e-launcher-control.log` (`EXIT:1`); checker green.
- **Package.** `createaddition.hmod`, **0.2.1**, 121,378 B, sha256
  `16ce1dec5e002333871794ca3839e837b6d3b04a7b0139d2c1d52ca36926d50c`; two builds byte-identical,
  `check` and `fits --build 0b14f8a` exit 0.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port, rust-review

---

## Slice 2 — the modular accumulator multiblock (0.2.2)

### The slice, and the gap lines it comes from

Two lines of this port's own record, both fixed by this slice:

> **Gaps**: "The rotation half … The accumulator is one block. The multiblock (up to 5 high × 3
> wide, `getTotalAccumulatorSize()`) is a gap" — 0.1.0's `## Gaps`, carried into 0.2.0.

> **Deviations** 7: "two **stacked single** accumulators trade energy both ways and settle at zero
> in the upper one — upstream would have merged them into one controller, which is exactly the
> missing multiblock."

**Choice.** The multiblock is the highest-value family left inside the `deep` row's *FE half*: it
is the one behaviour the FE half still got visibly wrong, it is a multiblock (the family the parity
ruling of 2026-11-20 calls out as the usual "it registers, so it must work" illusion), and it needs
no Create. The rotation half (`electric_motor`, `alternator`, `rolling_mill`, `tesla_coil`,
`servo_motor`) stays a gap: it needs the parent's kinetic API, which a package cannot reach
(`ports/_shared/external/create-horizon-architecture.md` §0).

**Tier fit.** `deep`: a block entity with formation state, a dynamic capability, per-block saved
data and a level pass — not a `data` registration, not a plain `machines` GUI. No renderer, no
dimension, no entity AI: nothing beyond the row.

### What it does (upstream `CAConnectivityHandler` / `ModularAccumulatorBlockEntity`)

- **Formation.** Contiguous accumulator blocks form one tower: a square footprint up to **3×3**
  and up to **5 high** (`accumulator_max_width`/`accumulator_max_height`), anchored at its lowest
  corner, which is the controller. The pass plans the largest box from the lowest free block and
  repeats for the remainder (`plan_towers`), which is upstream's greedy `formMulti` order.
  Capacity is `width² × height × 2,000,000` FE (`getTotalAccumulatorSize` ×
  `accumulator_capacity`); 45 blocks is 90,000,000 FE.
- **One pool behind every block.** Registered **ahead of the game's own** energy provider
  (`Registrar::block_capability_before_game(&ENERGY, …)`, `AccumulatorEnergy` in `network.rs`), the
  accumulator's `ENERGY` capability answers with the tower's whole charge and its whole capacity,
  as upstream's `handlerForCapability` gives every part the controller's storage. A connector on
  any block therefore drains the tower, not one block's 2,000,000.
- **Charge per block, not in the controller.** Each block keeps its 2,000,000 FE in its own saved
  compound (`energy`, the SDK's own key), so a split loses nothing and a 0.2.1 save stays readable.
  A store fills lowest-first and a drain takes highest-first (`store_tower`/`drain_tower`), both
  capped at the accumulator's 5,000 FE a call (`accumulator_max_input`/`_output`).
- **The jar's `top`/`bottom` bits.** Each member's state carries `top` (its layer is the tower's
  highest) and `bottom` (lowest); only those output to the block above/below (`tickOutput`), so a
  tower never feeds itself and two stacked accumulators no longer trade both ways.
- **Content.** The jar's own four-part blockstate (`block_single`/`block_top`/`block_middle`/
  `block_bottom`) and the three models are carried, so the tower looks like the jar's and
  `ctl block` reads the bits.

### The SDK fact this needed

`Registrar::block_energy` (`ModEnergyBuffer`) is a **static** declaration per block: capacity,
per-call limits and faces, with the amount in the compound. A multiblock's capacity is not static.
The pin's `block_capability_before_game` (NeoForge's `RegisterCapabilitiesEvent` priority) is what
lets a mod answer `ENERGY` itself ahead of that declaration — the handler gets the `Host` in every
method, and `merge_data` inside a transaction is rolled back on abort, so a pooled insert/extract
stays all-or-nothing. `cargo horizon-mod hooks block_capability` is the reference.

### Red → green (TDD)

The rules went in as failing tests first, against the 0.2.1 behaviour (every block its own tower;
one block's charge), then the glue. `red-tower.log`:

```
thread 'network::tests::a_column_of_three_is_one_tower_of_six_million_fe' panicked at src/network.rs:1735:5:
assertion `left == right` failed
  left: (1, 1)
 right: (1, 3)
thread 'network::tests::a_three_by_three_plate_is_one_tower_of_eighteen_million_fe' panicked at src/network.rs:1756:5:
assertion `left == right` failed
  left: (1, 1)
 right: (3, 1)
thread 'network::tests::a_column_of_six_is_a_five_high_tower_and_a_single' panicked at src/network.rs:1777:5:
assertion `left == right` failed
  left: (1, 1)
 right: (1, 5)
thread 'network::tests::a_cube_and_a_stray_block_form_the_largest_box_and_a_single' panicked at src/network.rs:1798:5:
assertion `left == right` failed
  left: (1, 1)
 right: (2, 2)
thread 'network::tests::storing_fills_the_lowest_block_first_and_draining_takes_the_highest_first' panicked at src/network.rs:1811:5:
assertion `left == right` failed
  left: 2000000
 right: 5000000
thread 'network::tests::a_tower_never_gives_more_than_its_blocks_hold' panicked at src/network.rs:1839:5:
assertion `left == right` failed
  left: 0
 right: 1000000
test result: FAILED. 46 passed; 6 failed; 0 ignored; 0 measured; 0 filtered out
```

Green: **53 passed, 0 failed** (`sdk-test.log`) — the 12 new tests are the tower planner, the
top/bottom roles, the saved geometry round-trip and the fill/drain order.

### The two carry errors, and the tag one (same class)

The Java mod is **correct** in all three. What was wrong was this port's carried JSON: it was in
**1.21.1's dialect**, and 26.3 accepted it at load, in `check`, in `fits` and in every unit test
while the data did nothing in game.

| # | What | Evidence | Now |
|---|---|---|---|
| 1 | **Recipe ingredients** (`{"item": …}` / `{"tag": …}` where 26.3 wants a bare string) — the class `mekanism` confirmed in game | **not present here**: this slice carries no recipe at all (they need `create:` items, `CONTENT.md`), so there is nothing to convert | `tools/data_shape.py` checks it, so a later slice that carries recipes cannot ship the object form |
| 2 | **Loot pool conditions** in the 1.21.1 array form with `bonus_rolls` | `tools/data_shape.py` on 0.2.2's first build: 3 tables, 9 problems; a real break with an iron pickaxe dropped **0** (`e2e-loot-prefix.log`, `broke and dropped 0`) | `carry_content.py` rewrites each pool to 26.3's shape: singular `"condition": {"type": "minecraft:survives_explosion"}`, no `bonus_rolls`, `rolls: 1` — the shape of `data/minecraft/loot_table/blocks/acacia_button.json` in the prepared resources. `--check` re-derives it; `data_shape.py` green; a real break drops **1** |
| 3 | **The harvest tag was not carried at all** (found by the same break leg) | a real break in survival with an iron pickaxe broke the block and left `items_left_on_ground: 0`: `requires_tool: true` plus no `minecraft:mineable/pickaxe` membership means no tool is ever "correct for drops" | `carry_content.py` carries `data/minecraft/tags/block/mineable/pickaxe.json` for the three carried blocks (the jar lists all 17 it registers) |

`tools/data_shape.py` is the generalised check (the idea of
`ports/modularrouters/tools/data_shape.py`, widened to also flag `bonus_rolls`, the inner
`"condition"` spelling and `1.0` rolls, and fixed to keep the whole path when reading a `--data`
tree — the shared copy classified a bare `connector.json` by its file name and skipped it):

```
$ python3 tools/data_shape.py createaddition.hmod      # 0.2.2's first build: red
  FAIL … loot_table/blocks/connector.json: 1.21.1 loot pool — pool 0: "conditions": [{"condition": "minecraft:survives_explosion"}] (26.3 uses "condition": {...})
  FAIL … loot_table/blocks/connector.json: 1.21.1 loot pool — pool 0: "bonus_rolls": 0.0 (26.3 has no such key)
  FAIL … loot_table/blocks/connector.json: 1.21.1 loot pool — pool 0: "rolls": 1.0 (26.3 spells an integral roll 1)
  (… the same three for large_connector.json and modular_accumulator.json; 9 problems)
$ python3 tools/data_shape.py createaddition.hmod      # the delivered package: green
  ok   … every one in the 26.3 dialect
```

### In-game (launcher-built game at `0a875da`; `e2e-tower.log`, `EXIT:0`)

```
== G: three stacked accumulators form one tower of 6,000,000 FE ==
  createaddition:modular_accumulator[bottom=true,top=false]
  createaddition:modular_accumulator[bottom=false,top=false]
  createaddition:modular_accumulator[bottom=false,top=true]
  Modular Accumulator at 220,100,0: 0/6000000 FE, tower 1x3 at 220,100,0 (t=754)
== G2: one pool — 5,000,000 FE into the top block is read from every part ==
  Modular Accumulator at 220,102,0: 5000000/6000000 FE, tower 1x3 at 220,100,0 (t=769)
  Modular Accumulator at 220,100,0: 5000000/6000000 FE, tower 1x3 at 220,100,0 (t=776)
== G3: breaking the top re-forms the tower at four million ==
  Modular Accumulator at 220,101,0: 4000000/4000000 FE, tower 1x2 at 220,100,0 (t=844)
== G4: a formed tower serves a wire from its pooled charge ==
  tower 3992960->3875920, sink 7040->124080, pool 0, total 4000000->4000000
== G5: CONTROL — three separated accumulators stay singles of 2,000,000 FE ==
  separate stack seeded 5000000 holds 2000000/2000000, the block two above 0
== H: every block's loot table drops the block itself (the 26.3 dialect) ==
  createaddition:connector: broke and dropped 1 (held 0 -> 1)
  createaddition:large_connector: broke and dropped 1 (held 0 -> 1)
  createaddition:modular_accumulator: broke and dropped 1 (held 0 -> 1)
PASS: …
EXIT:0
```

Why these can fail: G2's 5,000,000 seed is **impossible** on the 0.2.1 package (the command clamps a
single block at 2,000,000, which is exactly what G5 measures on three separated blocks); G3's
4,000,000 is what only two blocks that kept their own charge can report; G4's conservation is the
same assertion that caught the 0.1.0 pool bug; H's `dropped 1` was `dropped 0` on 0.2.2's first
build (`e2e-loot-prefix.log`); and `SELFTEST=1` (`e2e-tower-selftest.log`) plus the game without
the package (`e2e-tower-control.log`) both exit 1.
`check-e2e-logs.py createaddition createaddition-e2e --strict`: **logs=9 ok=9 missing-exit=0
bad-exit=0**.

### Coverage (this slice's families)

| family | status |
|---|---|
| tower formation, controller, `blocks × 2,000,000` capacity | **tested** — G (state bits on all three blocks; `tower 1x3 at 220,100,0`, `0/6000000`) |
| one pool behind every part (capability ahead of the game's) | **tested** — G2 (`5000000/6000000` from the top **and** the bottom) |
| split on a broken block, per-block charge kept | **tested** — G3 (`4000000/4000000` at `tower 1x2`) |
| output sides gated by `top`/`bottom` | **tested** — G's bits, F's single-block UP feed, G4's conservation (no self-feed) |
| a formed tower serves a wire | **tested** — G4 (`tower 3992960->3875920`, sink `7040->124080`, conserved) |
| separated accumulators stay singles | **tested** — G5 (2,000,000 cap, neighbours 0) |
| each block's loot table | **tested** — H (each of the three drops 1 when broken) |
| recipes | **not ported** — every one needs `create:` items (`CONTENT.md`) |
| comparator output, goggles/display source | **not ported** — `analog_output` exists at this pin; a later slice |

### Deviations of this slice (declared)

1. **Charge per block, not one `EnergyContent` in the controller.** Upstream keeps the tower's
   whole charge in the controller and redistributes on a split (dropping what will not fit). Here
   each block keeps its own 2,000,000, the capability sums them, and a split therefore loses
   nothing. What a player sees is the same: the pool, its capacity, the 5,000 FE a call.
2. **Formation is decided by the pass, one tick after placement.** Upstream forms on `onPlace` and
   `neighborChanged`; a package has both hooks, but the block entity is out of the world during its
   own tick, so the port forms in the level pass it already runs. A tower is one tick late, not
   absent.
3. **A store fills lowest-first and a drain takes highest-first.** Upstream's controller has one
   amount, so the order has no meaning there; here it is the rule that makes a split deterministic
   (and the e2e's G3 relies on it).
4. **The plan is deterministic where upstream is greedy per placed block.** `plan_towers` anchors
   on the lowest free block and repeats, so an L-shape always splits the same way; upstream's
   `PriorityQueue` over candidate controllers happens to agree on the boxes this slice forms.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port, rust-review

---

## Review fixes — 0.2.3 (PR #1526, 2026-11-21)

Three package findings from the reviewer (`levzzz5154`). Each was made red first; the reds are
quoted verbatim from this tree's logs (`ports/_shared/evidence/createaddition/`).

### 1. The tower plan was quadratic

`plan_towers` found each next anchor with `left.iter().min_by_key(|pos| (pos[1], pos[0], pos[2]))`
over the whole remainder, once per tower. `n` disconnected accumulators therefore cost
`n + (n-1) + … + 1 = n(n+1)/2` position examinations — the remainder was rescanned for every
disconnected block.

**Fix.** The remainder is an **ordered anchor set**: `Anchors` wraps a `BTreeSet<Anchor>`, and
`Anchor`'s `Ord` is `(y, x, z)` — upstream's controller order. `Anchors::first` takes the lowest
anchor with one set lookup and leaves it in the set so its own box can include it; every planned
member is removed once. One pass is now `n` anchor lookups + `n` removals + a constant number of
box probes per anchor (six for a disconnected singleton: `1×1` probes twice, `2×2`/`3×3` twice
each), so `O(n log n)` with a linear lookup count, never a remainder scan.

**Red** (`tower-red.log`), against the old remainder scan:

```
thread 'network::tests::the_tower_plan_is_linear_in_the_number_of_disconnected_accumulators' (2586304) panicked at src/network.rs:2204:5:
assertion `left == right` failed: doubling the accumulators must double the anchor work; rescanning the remainder for every disconnected accumulator would quadruple it
  left: 8256
 right: 4160
```

**The work-unit assertion** (`network.rs`, `the_tower_plan_is_linear_in_the_number_of_disconnected_accumulators`) —
set lookups, not a wall clock, so a faster machine cannot change it:

```rust
assert_eq!(
  doubled.anchors,
  2 * base.anchors,
  "doubling the accumulators must double the anchor work; rescanning the remainder for \
   every disconnected accumulator would quadruple it"
);
assert_eq!(base.anchors, 64, "one anchor taken per accumulator");
assert_eq!(base.removals, 64, "one member removed per accumulator");
// ...
assert_eq!(base, Lookups { anchors: 64, probes: 384, removals: 64 });
assert_eq!(doubled, Lookups { anchors: 128, probes: 768, removals: 128 });
```

So 64 disconnected accumulators take **512** set lookups and 128 take **1024** — exactly double;
the old scan took 8,256 anchor examinations at 128, which is why the red shows 8256 against 4160.
`Lookups` is a test-only meter (`Work` trait; production uses the no-op `Silent`), so the shipped
pass pays nothing for the proof. The formation result is unchanged: G/G2–G5 still form the same
3×3×5 boxes (the anchor order is the old `min_by_key` order).

### 2. A doctored saved tower was trusted before arithmetic

`tower_saved` accepted any `tw`/`th` >= 1 and only then called `tower.contains`; `Tower::capacity`
multiplied `width² × height × 2,000,000`, `members()` built a `Vec::with_capacity(blocks)`, and
`contains` added `controller + width` in `i32`. A save edited with `/data merge block` (the block
entity's saved compound **is** its NBT root: `{tw, th, tx, ty, tz, …, energy}`) could request
`1000×1000×1000` blocks, overflow the capacity, or overflow the coordinate box.

**Fix.** `tower_saved` rejects `width` outside `1..=3` or `height` outside `1..=5` **before** reading
the controller, before `contains`, `capacity` or `members`, and falls back to `Tower::single(pos)`
(the block's own 2,000,000 FE). `Tower::contains` compares in `i64`, so a legal 3×1 box whose
`controller + width` would cross `i32::MAX` cannot overflow either.

**Reds** (`tower-red.log`, `doctored-controller-red.log`):

```
thread 'network::tests::a_saved_tower_beyond_the_limit_is_read_as_a_single_block' (2586290) panicked at src/network.rs:2170:7:
assertion `left == right` failed: a saved 4x1 tower must be refused before any arithmetic
  left: Tower { controller: [4, 70, 9], width: 4, height: 1 }
 right: Tower { controller: [4, 70, 9], width: 1, height: 1 }
```

```
thread 'network::tests::a_saved_controller_past_the_coordinate_limit_does_not_overflow' (2586844) panicked at src/network.rs:1137:19:
attempt to add with overflow
```

**In-game proof** (`e2e-review-default.log`, G6): a real accumulator is seeded with 123 FE, then,
with the level frozen (`/tick freeze`, so the pass cannot rewrite it first), its block entity is
doctored to `tw:4,th:1` and then `tw:1000,th:1000`. Both read as the block's own single:

```
== G6: a doctored saved tower is refused before capacity arithmetic ==
  Modular Accumulator at 224,100,0: 123/2000000 FE (t=1079)
  Modular Accumulator at 224,100,0: 123/2000000 FE (t=1093)
  doctored tw/th 4x1 and 1000x1000 both read as the block's own 123/2000000
  the next pass re-formed it as a 1x1 single
```

Pre-fix, the 4×1 read would report `123/32000000` (4×4×1×2,000,000) and the 1000×1000 read would
allocate a billion `members()` entries. After unfreeze the next pass re-forms the legal 1×1, which
the leg asserts too.

### 3. The original MIT notice now travels in the package

**Red** (`licence-notice-red.log`): the 0.2.2 package carried only this port's `LICENSE` (which
paraphrased the copyright line), not the upstream notice:

```
FAIL: createaddition.hmod carries no LICENSE-UPSTREAM (the upstream MIT notice)
```

**Fix.** `LICENSE-UPSTREAM` is the upstream `LICENSE` file carried **verbatim** from the pinned
commit `658a390b8ff4daa17013e9447b29949330c8a42a` (`Copyright (c) 2024 MRH` plus the MIT grant);
the packer includes it (83 entries now). `LICENSE` remains this port's grant with the upstream
copyright beside it, `LICENSE-NOTICE.md` and `README.md` name `LICENSE-UPSTREAM`, and
`tools/check_licence.py` re-checks the packed bytes against the tree copy and the copyright line
(green: `ok createaddition.hmod carries the upstream MIT notice verbatim (1242 B, 'Copyright (c) 2024 MRH')`).

**Listing credit** (for `mods/createaddition.toml`, owned by the orchestrator): upstream author
**MRH0** — the copyright line reads "Copyright (c) 2024 MRH" — at
**https://github.com/MRH0/createaddition**.

### The 0.2.3 ladder

| Step | Command | Result |
|---|---|---|
| tests | `cargo test --offline` | **56 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `target/horizon-mod/createaddition.hmod`, exit 0 (`sdk-build.log`) |
| package, second build | same again | **byte-identical** (`cmp`) (`sdk-build-second.log`) |
| registry check | `cargo horizon-mod check createaddition.hmod` | exit 0 (`sdk-check.log`) |
| fits, campaign pin | `cargo horizon-mod fits createaddition.hmod --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a` | `0a875da  made for`, exit 0 (`sdk-fits-pin.log`) |
| fits, another build | `… --build 0b14f8acfd97fa2c750e42795249dcc981178ca4` | `0b14f8a  fits`, exit 0 (`sdk-fits-other.log`) |
| licence notice | `python3 tools/check_licence.py` | `ok … carries the upstream MIT notice verbatim`, exit 0 (`licence-notice.log`) |
| package verifier | `python3 ports/tools/verify-sdk-package.py createaddition` | **1/1** (`verify-sdk-package.log`) |

Deliverable: **`createaddition.hmod`, 133,587 B, SHA-256
`1b29b731068bc6a2f21281fc88f60b7643b39f819a4a2f8885d0ad0818fe53d6`**, 83 entries, reproducible (two
builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f962`, packed too).

```
createaddition 0.2.3 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  133587 bytes, SHA-256 1b29b731068bc6a2f21281fc88f60b7643b39f819a4a2f8885d0ad0818fe53d6, unsigned
```

```
Create Crafts & Additions:
0a875da  made for
Create Crafts & Additions:
0b14f8a  fits
```

In-game (launcher-built game at `0a875da`, launcher 0.2.92, `E2E_TMP` honoured — the script no
longer hardcodes `/tmp`):

| log | verdict | what it adds |
|---|---|---|
| `e2e-review-default.log` | `PASS` + `EXIT:0` | A–H unchanged, plus **G6**: the doctored 4×1 and 1000×1000 saves read as `123/2000000`, and the next pass re-forms the 1×1 |
| `e2e-review-selftest.log` | `FAIL` + `EXIT:1` | `SELFTEST: the conservation assertion did not trip` |
| `e2e-review-control.log` | `FAIL` + `EXIT:1` | the same item assertion on the package-less control game |
| `probe-doctored.log` | `EXIT:0` | established the saved-compound NBT path and that `/tick freeze` holds the pass off |

`python3 ports/tools/check-e2e-logs.py createaddition createaddition-e2e --strict`:
**logs=12 ok=12 missing-exit=0 bad-exit=0**.

### Coverage added by this slice

| family | status |
|---|---|
| tower plan work, linear in the accumulators | **tested** — `the_tower_plan_is_linear_…` (exact `Lookups` 64→512, 128→1024; red was 8256 vs 4160) |
| a saved shape outside 3×3×5 | **tested** — `a_saved_tower_beyond_the_limit_is_read_as_a_single_block` (dims 4×1, 1×6, 1000×1000, `i32::MAX`, 0, negative) + G6 |
| a saved controller at the coordinate limit | **tested** — `a_saved_controller_past_the_coordinate_limit_does_not_overflow` (i64 box test; red was `attempt to add with overflow`) |
| the upstream MIT notice in the package | **tested** — `tools/check_licence.py` (packed bytes == tree copy; `Copyright (c) 2024 MRH`) |

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port, rust-review

---

## Slice 4 — the redstone relay (0.2.4)

### The slice, and the gap line it comes from

The port's own gap list named it directly:

> **Gaps**: "**`portable_energy_interface`**, **`redstone_relay`**, **`energy_meter`**, …"

**Choice.** The relay is the one remaining family in the jar's FE half that adds a *rule* rather
than another buffer: upstream's `RedstoneRelayBlockEntity` is eight wire nodes in two ports (0–3
input, 4–7 output) and `WireGraph.tickBridges` moves energy from the input network to the output
network only while `RedstoneRelayBlock.POWERED` is set. It needs no Create kinetics, no GUI and no
renderer, and it is the only family left that can be proved in game with a conservation assertion.

**Tier fit.** `deep`: a block entity with two wire ports, a per-tick graph rule, a block-state gate
driven by the game's redstone engine, saved in-flight energy and a carried blockstate/model set.
No renderer, no dimension, no entity AI.

**Upstream behaviour read** (repo `658a390b…`, analysis only): `WireNodeKind.RELAY` (`portOf`,
`portCount`, `isBridge`, `isPowered`), `WireGraph.tickBridges` (`out.push(in.pull(bridge.demand))`),
`RedstoneRelayBlock.updateState`/`calculateInputStrength`/`getAvailableNode`, and
`RedstoneRelayBlockEntity.NODE_COUNT`, `isNodeInput`, `getMaxWireLength`.

### What it does

- **Eight nodes, two ports.** One `createaddition:redstone_relay` with a wire list: nodes 0–3 are the
  input port, 4–7 the output port. A wire's port is decided by which half of the relay the other end
  stands on (horizontal: the `facing` side is the output; vertical: the two halves swap, as
  `getAvailableNode` has them). A full port refuses the wire instead of silently using the other.
- **One block, two networks.** The relay's input port joins the record's input wire network and its
  output port the output one; the two are never merged. Each network keeps its own pool where it
  always did (its lowest member's compound).
- **The gate.** Every pass, `update_relay` reads the two redstone sides (`relay_sides`:
  above/below when vertical, across the long axis otherwise), takes the neighbour's signal and —
  horizontal only — its strong power, and writes the block's own `powered` bit. Unpowered, the
  bridge returns `RelayMove::default()` and no energy crosses; powered, it tops a 1,000 FE in-flight
  buffer up from the input pool and gives the output pool what it holds, at most 1,000 FE a tick
  either way. The in-flight buffer is the block's `energy` int, so a reload cannot lose it.
- **Content.** The jar's own 16-variant blockstate (`facing`/`powered`/`vertical`), its three models,
  its two stone textures and `connector.png`, the item definition, the loot table (26.3 dialect),
  the block declaration (plate shape) and the lang keys.
- **No capability.** Upstream's relay exposes no energy storage; the port declares no
  `block_energy` for it. Its 1,000 FE is the bridge's in-flight buffer, readable with
  `/createaddition info` but not insertable by a neighbour.

### Red → green (TDD)

The new rules went in first, against the pre-slice semantics (one port, one merged network, no
redstone gate). `relay-red.log`:

```
thread 'network::tests::a_horizontal_relay_takes_on_the_far_side_and_gives_on_the_facing_side' (1792454) panicked at src/network.rs:2986:5:
assertion `left == right` failed
  left: Input
 right: Output
thread 'network::tests::a_vertical_relay_reads_above_and_below_and_swaps_its_ports' (1792465) panicked at src/network.rs:3014:5:
assertion `left == right` failed
  left: Input
 right: Output
thread 'network::tests::an_unpowered_relay_carries_nothing_even_with_energy_in_flight' (1792469) panicked at src/network.rs:3041:5:
assertion `left == right` failed: the redstone signal is the gate: no bridge may move anything unpowered
  left: RelayMove { into: 200, out: 1000 }
 right: RelayMove { into: 0, out: 0 }
thread 'network::tests::a_relay_splits_a_wire_run_into_an_input_and_an_output_network' (1792458) panicked at src/network.rs:3112:5:
assertion `left == right` failed: the relay's two ports are separate networks, and the extra connector joins only the input one
  left: [[PortMember { pos: [0, 0, 0], port: 0 }, PortMember { pos: [1, 0, 0], port: 0 }, PortMember { pos: [1, 0, 0], port: 1 }, PortMember { pos: [2, 0, 0], port: 0 }, PortMember { pos: [0, 0, 1], port: 0 }]]
 right: [[PortMember { pos: [0, 0, 0], port: 0 }, PortMember { pos: [1, 0, 0], port: 0 }, PortMember { pos: [0, 0, 1], port: 0 }], [PortMember { pos: [1, 0, 0], port: 1 }, PortMember { pos: [2, 0, 0], port: 0 }]]
thread 'network::tests::the_relay_rig_moves_energy_only_while_powered' (1792477) panicked at src/network.rs:3158:5:
assertion `left == right` failed: an unpowered relay must not let one FE reach the sink
  left: 18000
 right: 0
```

`test result: FAILED. 61 passed; 5 failed`.

Green: **66 passed, 0 failed** (`sdk-test.log`) — the ten new tests are the node/port split, the
horizontal and vertical port choice, the full-port refusal, the gate, the 1,000 FE/t bridge with its
buffer, either-side powering, the two-network split, the rig simulation and the placement yaw.

### The harness defect this slice found (worth more than the slice)

The first in-game run powered the relay, measured 6,040 FE into the sink, and then reported **no
movement over the next 20 ticks** — the harness's, not the mod's. `relay_measure` held the read
argument in a local `powered`, and `info` evals `parse_info.py`'s `powered=…` field; bash's **dynamic
scoping** put that assignment into the caller's local, so the "restore the redstone block" branch
saw `powered=0` and never ran. Renamed to `with_power` with a comment. The probe
(`probe-relay.log`) shows the mod moving 99,980 → 0 into the sink across a power cycle.

### The 0.2.4 ladder

| Step | Command | Result |
|---|---|---|
| tests | `cargo test --offline` | **66 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `target/horizon-mod/createaddition.hmod`, exit 0 (`sdk-build.log`) |
| package, second build | same again | **byte-identical** (`cmp`) (`sdk-build-second.log`) |
| registry check | `cargo horizon-mod check createaddition.hmod` | exit 0 (`sdk-check.log`) |
| fits, campaign pin | `cargo horizon-mod fits createaddition.hmod --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a` | `0a875da  made for`, exit 0 (`sdk-fits-pin.log`) |
| fits, another build | `… --build 0b14f8acfd97fa2c750e42795249dcc981178ca4` | `0b14f8a  fits`, exit 0 (`sdk-fits-other.log`) |
| licence notice | `python3 tools/check_licence.py` | `ok … carries the upstream MIT notice verbatim`, exit 0 (`licence-notice.log`) |
| data shape | `python3 tools/data_shape.py createaddition.hmod` | 4/4 loot tables in the 26.3 dialect |
| content carry | `python3 tools/carry_content.py --check` | `assets/ and data/ match the jar carry` |
| package verifier | `python3 ports/tools/verify-sdk-package.py createaddition` | **1/1** (`verify-sdk-package.log`) |

Deliverable: **`createaddition.hmod`, 150,070 B, SHA-256
`8b5b1c2250b1d884d0b35fdc90a1cff5d46e3de7354dd95640ed34572783eacd`**, **94 entries**, reproducible
(two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f962`, packed too). 83 carried `assets/`+`data/` files (was 72).

```
createaddition 0.2.4 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  150070 bytes, SHA-256 8b5b1c2250b1d884d0b35fdc90a1cff5d46e3de7354dd95640ed34572783eacd, unsigned
```

```
Create Crafts & Additions:
0a875da  made for
Create Crafts & Additions:
0b14f8a  fits
```

The launcher rebuilt the game for this package (**`native-0a875da-39565764a7f0`**); the manifest
version bump made the first build new too (`native-0a875da-93f70f80b6b6`), and a source-only change
under the same version still recompiled, so the launcher fingerprints the package and not just its
manifest.

### In-game (`e2e-relay-default.log`, `PASS` + `EXIT:0`; `== I`)

```
== I: the redstone relay gates a wire between two accumulators ==
  createaddition:redstone_relay[facing=east,powered=false,vertical=false]
  Connector at 233,100,0: mode pull, buffer 0/1000 FE, links [], pool 0/0, demand 0, t=1386
  Redstone Relay at 234,100,0: powered false, relay 0/1000 FE, input [(233,100,0)/0/copper], output [(236,100,0)/4/copper], pool 0/0, demand 0, t=1415
  unpowered: source 99980, sink 0->0, pools 20/0/0, relay 0, total 100000->100000
  createaddition:redstone_relay[facing=east,powered=true,vertical=false]
  Modular Accumulator at 232,100,0: 92960/2000000 FE (t=1609)
  Modular Accumulator at 237,100,0: 5040/2000000 FE (t=1616)
  powered: source 92960->68940, sink 5040->29060, pools 2000/0/0, relay 0, total 100000->100000
  the powered relay carried 24020 FE to the sink, conserved at 100000
```

The rig is `A(232) – C1(233) – relay(234, facing east) – C2(236, push) – B(237)` on its own stone
pad; the relay reads `powered=false` from `ctl block` before the redstone block is placed and
`powered=true` after it. **Unpowered, the sink does not move for 20 ticks** (the differential); with
a redstone block on the relay's south side, 24,020 FE leaves the source and arrives at the sink in 20
ticks, and source + sink + both network pools + the relay's buffer stay at 100,000.

| log | verdict | what it proves |
|---|---|---|
| `e2e-relay-default.log` | `PASS` + `EXIT:0` | A–I: the whole earlier scenario plus the relay (unpowered ≠ powered), and all **four** carried loot tables drop one of themselves |
| `e2e-relay-selftest.log` | `FAIL` + `EXIT:1` | `SELFTEST: the conservation assertion did not trip` (the suite's own counterfactual) |
| `e2e-relay-gate-selftest.log` | `FAIL` + `EXIT:1` | `RELAY_SELFTEST: the unpowered relay moved the sink` — the relay gate assertion itself can fail |
| `e2e-relay-control.log` | `FAIL` + `EXIT:1` | the package-less game has neither the item nor the relay block id |
| `probe-relay.log` | `EXIT:0` | the probe: 99,980 → 0 across a power cycle, with the relay's saved `netReady/netNext/netDemand` and `energy` read from `/data get block` |

`python3 ports/tools/check-e2e-logs.py createaddition createaddition-e2e --strict`:
**logs=16 ok=16 missing-exit=0 bad-exit=0**.

### Coverage added by this slice

| family | status |
|---|---|
| the relay's eight nodes and two ports | **tested** — `a_relay_has_eight_nodes_in_two_ports`; in game the info line shows node 0 input, node 4 output |
| a wire's port follows the peer's side (horizontal and vertical) | **tested** — `a_horizontal_relay_takes_on_the_far_side_and_gives_on_the_facing_side`, `a_vertical_relay_reads_above_and_below_and_swaps_its_ports` |
| a full port refuses instead of borrowing the other | **tested** — `a_full_relay_port_refuses_rather_than_using_the_other_one` |
| the redstone gate (unpowered moves nothing) | **tested** — `an_unpowered_relay_carries_nothing_even_with_energy_in_flight`, `any_signal_on_either_side_powers_the_relay`, `the_relay_rig_moves_energy_only_while_powered`; in game I's unpowered 20 ticks and `RELAY_SELFTEST` |
| 1,000 FE/t through the bridge, in-flight buffer conserved | **tested** — `a_powered_relay_carries_a_thousand_fe_a_tick_and_keeps_the_rest`; in game I's `24020` conserved at `100000` |
| two separate networks across one relay | **tested** — `a_relay_splits_a_wire_run_into_an_input_and_an_output_network`; in game I's per-network pools |
| the block, its state and its `powered` bit | **tested** — `ctl block` reads `powered=false`/`true`; the port choice by side is in the link |
| the relay's loot table | **tested** — H (`createaddition:redstone_relay: broke and dropped 1`) |
| the relay's recipe | **not ported** — it needs `create:electron_tube` (`CONTENT.md`) |
| a real player's placement yaw (flat vs vertical) | **unit-tested** (`a_player_facing_north_places_the_relay_east_west`); not exercised in game (the e2e places with `/setblock`) |
| the comparator/ComputerCraft peripheral, goggles readout | **not ported** — CC is optional and not ported; the readout command is the interface |

### Deviations of this slice (declared)

1. **No energy capability on the relay.** Upstream's relay is a wire node, not a storage block; the
   port likewise declares no `block_energy`, so the 1,000 FE buffer is reachable neither by a
   neighbour's insert/extract nor by a connector mounted on the relay (which settles `none`). This is
   faithful, and it is why the bridge is the only way energy crosses.
2. **The port is chosen from the peer's side, not the click point.** The pin's `use_item_on` gives
   the face, not the hit location, and the link command has no click at all: a relay's port is the
   half the other wire end stands on. The count (4 + 4) and the refusal are the jar's.
3. **The `powered` bit is refreshed in the level pass**, one tick after the neighbour change, instead
   of upstream's two-tick scheduled `updateState`. The bit itself is the game's block state, so
   `ctl block` and the jar's own on/off models read it.
4. **A connector mounted on a relay settles `none`.** Upstream's `ConnectorMode.test` asks the
   capability, and the relay has none; the port's `mounted_mode` returns `none` for `Kind::Relay`
   instead of the pull a declared buffer would give.
5. **Vertical relays are supported by the state and the port rule, not by the e2e.** The block can be
   placed vertical (and `state_for_placement` does it on a side click), and the port mapping is
   unit-tested; the in-game leg uses a flat relay so the rig is readable.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port, rust-review

---

## Slice 5 — the small light connector (0.2.5)

### The slice, and the gap lines it comes from

Two lines of this port's own record, both from the jar inventory in `FINDINGS.md`:

> **Gaps**: "**`small_light_connector`** (its 1 FE/t lamp consumption)."

> **Where this port differs** 3: "**`small_light_connector`** (1 FE/t consumption, festive lamp) is a
> gap."

The 0.2.4 report's next-slice proposal named it explicitly. **Choice.** It is the one remaining
family in the jar's FE half that is a *rule over the network* rather than another buffer: upstream's
`SmallLightConnectorBlockEntity.specialTick` pulls 1 FE a tick out of the wire network and lights the
block at a per-position hysteresis threshold. It needs no Create kinetics, no GUI and no renderer,
and it is provable in game with a conservation assertion plus the light engine's own answer.

**Tier fit.** `deep`: a block entity with a per-tick network draw, a position-seeded hysteresis
counter, saved state (`tick_toggle_timer`), a block-state gate (`powered`), a `dynamic_light`
`light_emission` Part and a carried blockstate/model/texture set — inside the row, no dimension, no
entity AI.

**Upstream behaviour read** (repo `658a390b…`, analysis only): `SmallLightConnectorBlockEntity`
(`posTimeOffset = 10 + (abs(x*31 + y*45 + z*33) % 7) * 3`, `getMaxIn/Out` = the small connector's,
`getNodeCount` = 4, `getMaxWireLength` = 16, `specialTick`: `network.demand(1)`,
`network.pull(SMALL_LIGHT_CONNECTOR_CONSUMPTION, false) > 0`, the ±1 counter clamped at
`±posTimeOffset`, and the `POWERED` write at each clamp), `SmallLightConnectorBlock`
(`POWERED` in the state, `getLightEmission` = 15 when powered), `CommonConfig` (consumption **1**).

### What it does

- **A small connector with a lamp.** `Kind::Light` is `Connector::Small` for every connector rule
  (1,000 FE/t either way, 16-block wires, 4 nodes, pull/push/none modes, the spool click, its own
  1,000 FE window as a declared `block_energy`). One block entity type serves it with the other four
  blocks.
- **The lamp draws 1 FE a tick from its network.** In the level pass, after the relay step and before
  the pull connectors, every lamp calls `pool.take(1)` on its network's buffer and adds its 1 to the
  network's demand (upstream's `demand(1)`), whichever mode it is in — a lamp is a consumer, not a
  connector. The energy is spent: `source + pool + the lamp's consumption` is constant.
- **The hysteresis.** `light_clock` runs once per queued lamp per tick: the saved `tick_toggle_timer`
  rises when the pass paid the lamp and falls when it did not, clamped to
  `±light_period(pos)`; at `+period` the block's `powered` bit is set, at `-period` cleared, and
  `LevelRef::check_light` is called so the light engine re-asks. A lamp outside any network still
  counts down (`specialTick` runs before upstream's mode check).
- **The light.** The block declares `light: 0` + `dynamic_light: true`, and `LightShape`'s
  `light_emission` answers 15 when the state matches `powered=true` (with `block_states().matches`),
  so the answer cannot drift from the bit and reads the same on both sides.
- **Content.** The jar's own 36-part blockstate (facing × mode × powered; its six parts' girder
  variants are dropped with the undeclared `variant` property), its six models and the item model,
  the lit and dark bulb textures, the item definition and declaration, the loot table (26.3 dialect),
  the harvest-tag entry and the lang key.

### Red → green (TDD)

The rules went in first as stubs (`light_period` returning 10, `light_tick` returning the counter
unchanged) with their tests, against the pre-slice code. `lamp-red.log`:

```
thread 'network::tests::a_lamp_lights_after_its_period_of_energy_and_darkens_after_as_long_without' panicked at src/network.rs:3240:5:
assertion `left == right` failed
  left: (0, None)
 right: (1, None)
thread 'network::tests::a_lamp_on_a_network_with_no_energy_draws_nothing' (126446) panicked at src/network.rs:3301:5:
assertion `left == right` failed
  left: (0, None)
 right: (-1, None)
thread 'network::tests::a_lamp_that_never_got_energy_goes_dark_and_stays_dark' panicked at src/network.rs:3256:5:
assertion `left == right` failed
  left: (0, None)
 right: (-1, None)
thread 'network::tests::a_lamps_period_is_its_own_ten_to_twenty_eight_tick_cycle' (126448) panicked at src/network.rs:3221:5:
assertion `left == right` failed: 2880 % 7 = 3
  left: 10
 right: 19
test result: FAILED. 68 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out
```

Green: **72 passed, 0 failed** (`sdk-test.log`) — the six new tests are the lamp's kind/window, the
position-seeded period (including the absolute seed and the 10..=28 range over a 16×16 grid), both
hysteresis thresholds with the clamps, a lamp that never got energy, the rig's 1 FE/t conservation,
and the unlinked lamp that draws nothing. `src/runtime.rs` is untouched
(`03721ca4…f962`, packed too).

### The 0.2.5 ladder

| Step | Command | Result |
|---|---|---|
| tests | `cargo test --offline` | **72 passed**, 0 failed (`sdk-test.log`, regenerated for this version) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `target/horizon-mod/createaddition.hmod`, exit 0 (`lamp-build-1.log`) |
| package, second build | same again | **byte-identical** (`cmp`) (`lamp-build-2.log`) |
| registry check | `cargo horizon-mod check createaddition.hmod` | exit 0 (`lamp-check.log`) |
| fits, campaign pin | `cargo horizon-mod fits … --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a` | `0a875da  made for`, exit 0 (`lamp-fits-pin.log`) |
| fits, another build | `… --build 0b14f8ac…` and `… --build 6d765e0e…` | **`does not fit: 1 item changed — the method BlockStates::light_emission (removed)`**, exit 1 (`lamp-fits-other.log`) |
| data shape | `python3 tools/data_shape.py createaddition.hmod` | 5/5 loot tables in the 26.3 dialect |
| content carry | `python3 tools/carry_content.py --check` | `assets/ and data/ match the jar carry` |
| licence notice | `python3 tools/check_licence.py` | `ok … carries the upstream MIT notice verbatim` |
| package verifier | `python3 ports/tools/verify-sdk-package.py createaddition` | **1/1** |

**The second `fits` run is red on purpose, and that is the finding.** `light_emission` (the hook, the
`BlockStates` service method and `dynamic_light`) does not exist at `0b14f8a` or at the next nightly
`6d765e0`: the lamp's light surface is **new at the 0a875da pin**. So the pin run is not a tautology —
the older builds prove the check discriminates — but no earlier build can carry this family, and a
future repin must keep `light_emission` (or the package loses the lamp).

Deliverable: **`createaddition.hmod`, 168,353 B, SHA-256
`854e9d6ac9a8b9af0501310053c8e9c833215694663959501447a15199372b41`**, **108 entries**, reproducible
(two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f962`, packed too). 79 `assets/` and 18 `data/` files (was 67 + 16; the lamp adds its
blockstate, seven models, two textures, its item definition and item model, and two data files).

```
createaddition 0.2.5 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  168353 bytes, SHA-256 854e9d6ac9a8b9af0501310053c8e9c833215694663959501447a15199372b41, unsigned
```

```
Create Crafts & Additions:
0a875da  made for
Create Crafts & Additions:
0b14f8a  does not fit: 1 item changed
    the method `BlockStates::light_emission` (removed)
```

The launcher rebuilt the game for this package (`native-0a875da-9e6e38b74abd`, 6 s: the 0.2.4 build
`native-0a875da-39565764a7f0` was not reused, so the version bump is again what forces a recompile).

### In-game (`e2e-lamp-default.log`, `PASS` + `EXIT:0`; `== J`)

The rig is `accumulator(250) – connector(251, pull) – lamp(254, on a stone wall at 255)` on its own
stone pad, linked with copper; the player stands on the pad so every read is within reach.

```
== J: the small light connector lights on its network's one FE a tick ==
  the lamp at 254,100,0 cycles in 25 ticks (the jar's position-seeded period)
  Small Light Connector at 254,100,0: mode none, powered false, lamp timer -10/25, draw 1 FE/t, buffer 0/1000 FE, links [], pool 0/0, demand 0, t=1879
  -- at 0 FE the lamp is dark and its counter sits at the bottom clamp
  createaddition:small_light_connector[facing=east,mode=none,powered=false], block light 0
  Small Light Connector at 254,100,0: mode none, powered false, lamp timer -25/25, draw 1 FE/t, buffer 0/1000 FE, links [], pool 0/0, demand 0, t=1919
  -- seeding 5,000 FE and linking the wire lights it
  link 251-254 (copper): Nodes linked.
  createaddition:small_light_connector[facing=east,mode=none,powered=true], block light f
  Small Light Connector at 254,100,0: mode none, powered true, lamp timer 25/25, draw 1 FE/t, buffer 0/1000 FE, links [(251,100,0)/0/copper], pool 0/0, demand 0, t=1998
  -- it draws exactly one FE a tick, and the network's pool holds
  Modular Accumulator at 250,100,0: 4903/2000000 FE (t=2005)
  Connector at 251,100,0: mode pull, buffer 0/1000 FE, links [(254,100,0)/0/copper], pool 6/20, demand 1, t=2012
  Connector at 251,100,0: mode pull, buffer 0/1000 FE, links [(254,100,0)/0/copper], pool 6/20, demand 1, t=2040
  Modular Accumulator at 250,100,0: 4861/2000000 FE (t=2047)
  createaddition:small_light_connector[facing=east,mode=none,powered=true], block light f
  source 4903->4861 over 42 ticks: 42 FE, one a tick, pool steady at 26
  -- CONTROL: unlinked, the lamp draws nothing and the source does not move
  unlink 251-254: Wire removed.
  Modular Accumulator at 250,100,0: 4852/2000000 FE (t=2063)
  createaddition:small_light_connector[facing=east,mode=none,powered=false], block light 0
  Modular Accumulator at 250,100,0: 4852/2000000 FE (t=2127)
  unlinked: the lamp went dark and the source stayed at 4852 FE
```

Why these can fail: at 0 FE, a lit lamp would fail `powered=false`/`light 0`; the seed leg's
`block light f` is the light engine's answer, not the mod's readout; `42 FE over 42 ticks` is exact
(one unit of the source per tick with the pool held at 26 — a 2 FE/t or 0 FE/t draw fails it); and
the counter's readout (`-25/25` then `25/25`) is the hysteresis itself. `LAMP_SELFTEST=1`
(`e2e-lamp-gate-selftest.log`) flips the unlinked assertion and trips; `SELFTEST=1`
(`e2e-lamp-selftest.log`) flips B's conservation; the package-less control
(`e2e-lamp-control.log`) fails at the first item assertion.

**Correction to the brief's expected numbers.** The brief asked for `powered=false` at 0 FE after
`posTimeOffset+5` ticks and `powered=true` after the *same* wait once seeded. The first half holds
(the counter starts at 0 and reaches `-period` in `period` ticks). The second does not: upstream
clamps the counter at `±posTimeOffset`, so a lamp that has run dark sits at `-period` and needs
**`2 * period`** ticks of energy to reach `+period`. The leg therefore waits `2 * period + 5` and
records why. The same is true of the unlink leg: from `+period` it needs `2 * period` ticks to go
dark. This is upstream's own hysteresis, not a port choice.

| log | verdict | what it proves |
|---|---|---|
| `e2e-lamp-default.log` | `PASS` + `EXIT:0` | A–I unchanged (the relay still carries 25,020 FE with the total conserved), plus **J**: dark at 0 FE (`light 0`), lit after its hysteresis (`powered=true`, `light f`), exactly 1 FE/t over 42 ticks with the pool steady, and the unlinked differential (dark, source unchanged) |
| `e2e-lamp-selftest.log` | `FAIL` + `EXIT:1` | `SELFTEST: the conservation assertion did not trip` (B's counterfactual) |
| `e2e-lamp-gate-selftest.log` | `FAIL` + `EXIT:1` | `LAMP_SELFTEST: the unlinked lamp drew nothing, so the assertion is alive` (J's own counterfactual) |
| `e2e-lamp-control.log` | `FAIL` + `EXIT:1` | the package-less game has neither the accumulator item nor the lamp block id (and refuses `createaddition:redstone_relay` and `createaddition:small_light_connector` by name) |

`python3 ports/tools/check-e2e-logs.py createaddition createaddition-e2e --strict`:
**logs=20 ok=20 missing-exit=0 bad-exit=0**.

### Harness facts this slice produced

1. **`ctl messages` is newest-first, so a failure dump must print `head`, not `tail`.** The first
   run's final read timed out and the diagnostic printed the *oldest* 15 of the 60-line window,
   hiding the answer. `fail()` now prints `chat | head -20`.
2. **A chat line can be lost.** The same `info` command succeeded 55 ticks earlier and then got no
   reply at all, so `command_reply` now re-sends the *same tagged line* once before failing. The
   assertion is unchanged: only a matching reply ends the wait.
3. **`ctl light X Y Z X Y Z` answers one hex digit** (`.`block`, `f` = 15), which is how the lamp's
   light is read from the light engine rather than from the mod's readout.

### Coverage added by this slice

| family | status |
|---|---|
| the lamp is a full small connector (rates, nodes, reach, modes, its 1,000 FE window) | **tested** — `the_light_connector_is_a_small_connector_with_a_lamp`; in game its mode settled to `none` on stone and the copper wire linked |
| the position-seeded hysteresis period | **tested** — `a_lamps_period_is_its_own_ten_to_twenty_eight_tick_cycle`; in game `lamp timer 25/25` at 254,100,0 |
| the counter and both clamps | **tested** — `a_lamp_lights_after_its_period_of_energy_and_darkens_after_as_long_without`, `a_lamp_that_never_got_energy_goes_dark_and_stays_dark`; in game `-25/25` → `25/25` → dark |
| the 1 FE/t network draw, conserved | **tested** — `the_lamp_rig_draws_one_fe_a_tick_and_conserves_it`; in game `42 FE over 42 ticks`, pool steady at 26 |
| the `powered` bit and block light 15 / 0 | **tested** — in game `ctl block` `powered=true` + `ctl light` `f`, then `powered=false` + `0` |
| an unlinked lamp draws nothing | **tested** — `a_lamp_on_a_network_with_no_energy_draws_nothing`; in game the source stayed at 4,852 over 66 ticks, plus `LAMP_SELFTEST` |
| the lamp's loot table | **tested** — H now breaks all five blocks (`small_light_connector: broke and dropped 1`) |
| the lamp's recipe | **not ported** — it needs `c:wires/iron` and `createaddition:connector`, and the iron wire is not carried (`CONTENT.md`) |
| a lamp's own `pull`/`push` mode feeding the wire | unit-tested as the connector rules; not exercised in game (the e2e's lamp is mounted on stone and idle, which is the readout's `mode none`) |

### Deviations of this slice (declared)

1. **The lamp's draw is booked in the network pass, not in its own tick.** Upstream pulls from the
   in-memory `EnergyNetwork` inside `specialTick`; this port's pass already owns the network's buffer,
   so the draw is one `pool.take(1)` in the same transaction as the moves, and the lamp's counter is
   updated from the result. The order (after the relay, before the pulls) is what gives the steady
   state the jar's numbers do: the lamp spends 1, the pull connector fills exactly that back.
2. **The counter is saved every tick it changes.** Upstream keeps `tick_toggle_timer` in memory and
   writes it in `writeSafe`; here it is the block entity's saved compound, so a reload keeps the
   hysteresis instead of restarting it. The write only happens when the clamped counter moves.
3. **The `powered` bit is written one tick later than upstream's counter clamp**, in the pass, as the
   relay's bit already is; the bit is the game's own state, so `ctl block`, the models and the light
   engine all read it.
4. **`dynamic_light` instead of a per-state emission.** The content schema has no per-state light, so
   the block declares `dynamic_light: true` and `LightShape::light_emission` answers from the state.
   That hook is new at this pin (see the `fits` finding above).

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port, rust-review
