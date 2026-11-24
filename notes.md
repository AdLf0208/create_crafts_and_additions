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

Status: **slice 17 implemented and delivered as a flat SDK package (mod API 0.2), version
0.2.16** — the two connectors, wire links and the modular accumulator, with the jar's rates and
connection rules, plus the **redstone relay** that bridges one wire network into another while
powered, the **small light connector** whose lamp draws 1 FE a tick and lights while its network
pays it, the accumulator's **comparator output** (the jar's fill fraction), the **creative
generator** whose capability answers infinity and which fills every neighbour each tick, the jar's
**fourth wire, the festive spool**, the **wire renderer** (the wire family's client half), the
**wires a broken connector drops**, and the **lifecycle clears** that take a comparator answer out
with its block, its chunk and its level (plus a spool selection with its level and its player), in
one `createaddition.hmod` that carries the code *and* the content. (0.2.0
was the migration; 0.2.2 the tower; 0.2.3 the three review fixes, §
[`## Review fixes — 0.2.3`](#review-fixes--023-pr-1526-2026-11-21); 0.2.4 the relay, §
[`## Slice 4 — the redstone relay (0.2.4)`](#slice-4--the-redstone-relay-024); 0.2.5 the lamp, §
[`## Slice 5 — the small light connector (0.2.5)`](#slice-5--the-small-light-connector-025); 0.2.6
the comparator, § [`## Slice 6 — the accumulator's comparator output (0.2.6)`](#slice-6--the-accumulators-comparator-output-026);
0.2.7 the creative generator, § [`## Slice 8 — the creative generator (0.2.7)`](#slice-8--the-creative-generator-027);
0.2.8 the festive spool; 0.2.9 the wire renderer; 0.2.10 the break-drop path; 0.2.11 the lifecycle
clears, § [`## Slice 12 — the lifecycle clears (0.2.11)`](#slice-12--the-lifecycle-clears-0211);
0.2.12 the support rule; 0.2.13 the accumulator's gauge needle, §
[`## Slice 14 — the accumulator's gauge readout (0.2.13)`](#slice-14--the-accumulators-gauge-readout-0213);
0.2.14 the wire renderer's lightmap, §
[`## Slice 15 — the wire renderer's lightmap (0.2.14)`](#slice-15--the-wire-renderers-lightmap-0214);
0.2.15 the held-wire preview, §
[`## Slice 16 — the held-wire preview (0.2.15)`](#slice-16--the-held-wire-preview-0215);
0.2.16 the brush wire drop, §
[`## Slice 17 — the brush wire drop (0.2.16)`](#slice-17--the-brush-wire-drop-0216).) The
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
  no GUI: `/createaddition info|energy|mode|link|unlink|signals`, each reply optionally
  tagged (`#tag`) so a harness can tell answers apart (`signals`, 0.2.11, is the readout of the
  answers the mod holds and the selections in progress — § *Slice 12*).
- **`createaddition:creative_energy`** (0.2.7) — upstream's creative generator: its
  `ENERGY` capability answers `u32::MAX` stored and capacity, gives whatever is asked and takes
  nothing, so a connector mounted on it settles **pull**; and each tick it offers every one of
  its six neighbours a full fill (`receiveEnergy(Integer.MAX_VALUE, false)`), which the
  neighbour's own insert caps (1,000 into a connector's window, 5,000 into an accumulator).

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
- ~~**`creative_energy`**, the e2e's source of last resort: the harness seeds an
  accumulator with `/createaddition energy` instead.~~ **Closed in 0.2.7**: the creative
  generator (infinite capability + per-tick neighbour fill), §
  [`## Slice 8`](#slice-8--the-creative-generator-027).
- **`portable_energy_interface`** (needs Create's contraption actor),
  **`electric_pump`** (extends Create's `PumpBlock`), **`digital_adapter`** (the ComputerCraft
  bridge — CC is an optional dependency and not ported). (`redstone_relay` moved out of this list
  in 0.2.4; `energy_meter` is **not in the 1.7.2 jar at all** — it is commented-out dead code on
  upstream `1.21.1`, § [`## Slice 7`](#slice-7--the-energy-meter-blocked-not-in-the-artefact).)
- **GUI/tooltips**: no goggles-style readout, no mode wrench, no `PhysicsBlock`/display source for
  the gauge. (The accumulator's comparator output moved out of this line in 0.2.6, §
  [`## Slice 6`](#slice-6--the-accumulators-comparator-output-026).)
- **Crafting**: no recipes (`CONTENT.md`) — the blocks are creative-only until
  the parent's items exist.
- **Festive wire and spool**, the iron wire, and the electrum/brass/copper metal
  items. (All but the metal items are carried since 0.2.8; the wire is *drawn* since
  0.2.9, § [`## Slice 10`](#slice-10--the-wire-renderer-029).)
- ~~**The wire-drop path** (`WireType.getDrop()`/`getSourceDrop()` on a broken connector): a
  broken connector dropped nothing and left the wire in its peer's list.~~ **Closed in 0.2.10**:
  four wires of each kind at the broken block, a festive wire's four being copper wires, the peer's
  link removed without a second drop, and nothing in creative, §
  [`## Slice 11`](#slice-11--the-wires-a-broken-connector-drops-0210).
- ~~**The lightmap on a wire** (block/sky light interpolation): a wire in a dark room was as bright
  as one in daylight.~~ **Closed in 0.2.14**: the jar's endpoint light reads, per-piece `Mth.lerp`
  and `LightTexture.pack` are carried into the existing renderer, §
  [`## Slice 15`](#slice-15--the-wire-renderers-lightmap-0214).
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
| comparator output, goggles/display source | **comparator closed in 0.2.6** — § [`## Slice 6`](#slice-6--the-accumulators-comparator-output-026); goggles/display source **not ported** (client half) |

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

---

## Slice 6 — the accumulator's comparator output (0.2.6)

### The slice, and the gap line it comes from

The port's own coverage table named it, and the 0.2.5 report proposed it as the next slice:

> **Coverage, slice 2**: "comparator output, goggles/display source | **not ported** —
> `analog_output` exists at this pin; a later slice"

**Choice.** The accumulator's comparator answer is the last family of the FE half's *behaviour* that
the jar has and the port does not: upstream's `ModularAccumulatorBlock` tells a comparator yes and
then answers with the tower's fill, so a tower is readable by the redstone the game already has (a
display lamp, a hatch, a level sensor). It needs no Create kinetics, no GUI and no renderer, and it
is provable in game on the **world's own** state — the wire's `power` — not on a mod readout.

**Tier fit.** `deep`: a block Part on the `has_analog_output`/`analog_output` hook pair, a per-level
answer map, a neighbour-signal update that has to be told only when the number changes, and an answer
computed from a multiblock pool rather than a block's own buffer. No renderer, no dimension, no
entity AI; nothing beyond the row.

**Upstream behaviour read** (repo `658a390b…`, analysis only; the parent's mapping read from
`Creators-of-Create/Create` `mc1.21.1/dev`):

- `ModularAccumulatorBlock.hasAnalogOutputSignal(state)` → `true` for every state.
- `ModularAccumulatorBlock.getAnalogOutputSignal(state, level, pos)` →
  `getBlockEntityOptional(level, pos).map(ModularAccumulatorBlockEntity::getControllerBE)
  .map(te -> ComparatorUtil.fractionToRedstoneLevel(te.getFillState())).orElse(0)` — so **any**
  part of a tower answers the controller's fill.
- `ModularAccumulatorBlockEntity.getFillState()` →
  `(float) energyCapability.getEnergyStored() / energyCapability.getMaxEnergyStored()`.
- `com.simibubi.create.foundation.blockEntity.ComparatorUtil.fractionToRedstoneLevel(double frac)` →
  `Mth.floor(Mth.clamp(frac * 14 + (frac > 0 ? 1 : 0), 0, 15))`. The `+1` is why a container just
  above empty reads **1** and half full reads **8**, not 7 — the middle value the e2e pins.

### What it does

- **The pair, on the accumulator's block key.** `AccumulatorShape` (a `BlockPart`) answers
  `has_analog_output` true for the accumulator's states and `analog_output` with the block's last
  recorded answer, so a comparator asks only for our block and gets the tower's fraction.
- **Computed in the pass, read back by the hook.** `analog_output` is asked with a `LevelHost`,
  which cannot open a block entity (`HostBlockStates` is all it offers), and the hook's own doc says
  not to change the world there. The level pass already reads every tower's charges after its moves,
  so it computes `tower_signal(tower, amount)` for every queued accumulator block, records it in
  `Signals` keyed by the dimension id and the block position, and — only when the number changed —
  calls `level.update_neighbors(host, pos, state, NeighbourUpdate::OutputSignal)`, which is vanilla's
  `updateNeighbourForOutputSignal`: the comparators beside the block read again. A settled tower
  therefore costs no redstone update at all.
- **The jar's arithmetic, including its float.** `fraction_to_level` is the parent's formula
  verbatim, and the fraction is the jar's `float` division (`getFillState()`) widened to the double
  the parent's method takes — kept, not tidied: at a 45-block tower's 90,000,000 FE the float and a
  double disagree on 18 exact charges.
- **No content change.** The family is a block Part and a rule; the package's `assets/`/`data/` are
  the same 108 entries as 0.2.5's.

### Red → green (TDD)

The rules went in first, against the pre-slice code (no answer at all: `analog_output` did not
exist, and the two helpers were stubs). `comparator-red.log`, verbatim:

```
thread 'network::tests::the_comparator_level_is_the_jars_fraction_mapping' (1746826) panicked at src/network.rs:3533:5:
assertion `left == right` failed: half full is 8, not 7: the jar's +1 (a 0-vs-15 run would read 15)
  left: 15
 right: 8
thread 'network::tests::a_towers_level_uses_the_towers_capacity_not_one_blocks' (1746816) panicked at src/network.rs:3562:5:
assertion `left == right` failed: a half-full 1x3 tower reads the tower's fraction, not a block's
  left: 15
 right: 8
thread 'network::tests::a_comparator_is_told_only_when_its_answer_changes' (1746795) panicked at src/network.rs:3585:5:
the first answer is a change, so the comparator is told
test result: FAILED. 72 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out
```

The **fidelity** rule had its own red (`comparator-fraction-red.log`), taken by reverting the f32
step to the double one after the three rules above had gone green, then restoring the fix:

```
test network::tests::the_fraction_is_the_jars_single_precision_division ... FAILED
thread 'network::tests::the_fraction_is_the_jars_single_precision_division' (936759) panicked at src/network.rs:3675:5:
  left: 3
 right: 4
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 75 filtered out
```

**A test bug the red surfaced, not a code bug.** `a_comparator_is_told_only_when_its_answer_changes`
asserted `!note(...)` for "another dimension's block at the same coordinates **is a first answer**"
and for "another position is a first answer". A first answer counts as a change by the map's own
contract, so the two assertions were inverted; they were corrected to `note(...) == true` (the
`value(...)` expectations beside them were already right). Nothing in the shipped rule changed.

Green: **76 passed, 0 failed** (`sdk-test.log`, regenerated for 0.2.6) — the eight new tests are the
fraction mapping (0, 1, the 8 midpoint, 14, 15, a save past capacity, both sides of the 1/14 step),
the f32 fidelity case, the tower-vs-block fraction, the change detection, and the `Signals` map.

### The geometry, measured rather than assumed (`e2e-comparator-probe.log`)

A comparator's `facing` is the side it **reads**; it drives the block behind it. The probe placed one
comparator on each of the two horizontal sides of a full tower, each with its own wire:

```
  probe: comparatorok facing=north, wire z=2 power=15
  probe: comparatorok facing=south, wire z=-2 power=15
```

Both mirror orientations answer 15, and each drives the wire on the opposite side of `facing`, which
is the geometry the default run then asserts. `CMP_ONLY=1` keeps the probe runnable.

### The 0.2.6 ladder

| Step | Command | Result |
|---|---|---|
| tests | `cargo test --offline` | **76 passed**, 0 failed (`sdk-test.log`, regenerated for 0.2.6) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | clean (`sdk-fmt.log`; `src/runtime.rs` unchanged, `03721ca4…f962`) |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `target/horizon-mod/createaddition.hmod`, exit 0 (`comparator-build-1.log`) |
| package, second build | same again | **byte-identical** (`cmp`) (`comparator-build-2.log`) |
| registry check | `cargo horizon-mod check createaddition.hmod` | exit 0 (`comparator-check.log`) |
| fits, campaign pin | `cargo horizon-mod fits … --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a` | `0a875da  made for`, exit 0 (`comparator-fits-pin.log`) |
| fits, another build | `… --build 0b14f8acfd97fa2c750e42795249dcc981178ca4` | **`does not fit: 1 item changed — the method BlockStates::light_emission (removed)`**, exit 1 (`comparator-fits-other.log`) — 0.2.5's finding, unchanged |
| data shape | `python3 tools/data_shape.py createaddition.hmod` | 5/5 loot tables in the 26.3 dialect |
| content carry | `python3 tools/carry_content.py --check` | `assets/ and data/ match the jar carry` |
| licence notice | `python3 tools/check_licence.py` | `ok … carries the upstream MIT notice verbatim` |
| package verifier | `python3 ports/tools/verify-sdk-package.py createaddition` | **1/1** (`verify-sdk-package.log`) |

The second `fits` run is red for 0.2.5's reason, not this slice's: the lamp's `light_emission`
surface does not exist at `0b14f8a`, so no earlier build can carry the package. The comparator
itself uses no new surface — `has_analog_output`/`analog_output` and
`NeighbourUpdate::OutputSignal` are at the pin (the same hook pair other ports at this pin answer).

Deliverable: **`createaddition.hmod`, 172,461 B, SHA-256
`9daebe6da2536041f42f76e2b7f169ee2d243ae60fbbc0312f719508b6e756f4`**, **108 entries**, reproducible
(two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f962`, packed too).

```
createaddition 0.2.6 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  172461 bytes, SHA-256 9daebe6da2536041f42f76e2b7f169ee2d243ae60fbbc0312f719508b6e756f4, unsigned
```

```
Create Crafts & Additions:
0a875da  made for
Create Crafts & Additions:
0b14f8a  does not fit: 1 item changed
    the method `BlockStates::light_emission` (removed)
```

The launcher rebuilt the game for this package (`native-0a875da-fa83417f8a7d`,
`Compiling createaddition (1 of 1)`): the manifest bump is again what forces the recompile.

### In-game (`e2e-comparator-default.log`, `PASS` + `EXIT:0`; `== K`)

The rig is a formed 1×3 tower at `260,100,0` (6,000,000 FE), a comparator at `260,101,1` whose
`facing` side touches the tower's middle block, and a redstone wire at `260,101,2`; the differential
is the same comparator at `263,101,1` reading a stone block at `263,101,0`, with its own wire at
`263,101,2`. The player stands on the pad, so every read is within reach.

```
== K: the accumulator answers a comparator with the jar's fraction ==
  Modular Accumulator at 260,100,0: 0/6000000 FE, tower 1x3 at 260,100,0 (t=2293)
  -- an empty 1x3 tower reads power=0
  an empty tower: redstone_wire at 260,101,2 power=0
  -- 3,000,000 of 6,000,000 reads the jar's 8 (7 is the un-offset fraction, 15 a 0/15 mapping)
  Modular Accumulator at 260,100,0: 3000000/6000000 FE, tower 1x3 at 260,100,0 (t=2341)
  a half-full tower: redstone_wire at 260,101,2 power=8
  -- a full tower reads power=15
  Modular Accumulator at 260,100,0: 6000000/6000000 FE, tower 1x3 at 260,100,0 (t=2365)
  a full tower: redstone_wire at 260,101,2 power=15
  -- the differential: the same comparator rig behind a stone block reads 0
  a stone block: redstone_wire at 263,101,2 power=0
  -- emptied with /createaddition energy, the wire falls back to 0
  emptied: redstone_wire at 260,101,2 power=0
```

Why these can fail: the **middle** seed is the jar's own midpoint — a mapping that answered 15 for
any charge (or 7 from the un-offset fraction, or 0 from `stored / capacity * 15`) fails `power=8`;
the stone rig reads 0 in the same tick the tower reads 15, so an implementation that answered its
fraction for every block would fail there; and the emptied step moves the wire back to 0, which is
the assertion `COMPARATOR_SELFTEST=1` inverts (it demands the wire have stayed at 15) — that leg
fails, `FAIL: COMPARATOR_SELFTEST: emptying the tower moved the wire from power=15 to power=0, so
the assertion is alive` (`e2e-comparator-selftest.log`, `EXIT:1`).

| log | verdict | what it proves |
|---|---|---|
| `e2e-comparator-default.log` | `PASS` + `EXIT:0` | A–J unchanged, plus **K**: the wire reads 0, 8 and 15 for the three seeds, the stone rig reads 0 while the tower reads 15, and emptying the tower returns the wire to 0 |
| `e2e-comparator-selftest.log` | `FAIL` + `EXIT:1` | K's own counterfactual (`COMPARATOR_SELFTEST`) |
| `e2e-comparator-suite-selftest.log` | `FAIL` + `EXIT:1` | the suite's conservation counterfactual, still alive (`SELFTEST`) |
| `e2e-comparator-control.log` | `FAIL` + `EXIT:1` | a game without the package has no accumulator item, so the ids come from it |
| `e2e-comparator-probe.log` | `PASS` + `EXIT:0` | the comparator geometry: both mirror orientations, each driving the wire behind it |

`python3 ports/tools/check-e2e-logs.py createaddition createaddition-e2e --strict`:
**logs=25 ok=25 missing-exit=0 bad-exit=0**.

### Coverage added by this slice

| family | status |
|---|---|
| the comparator is told the accumulator has an output | **tested** — in game `ctl block` on the wire is non-empty at 0 FE; the answer is asked for by the game only because `has_analog_output` said yes |
| the jar's fraction mapping (0, 1, 8 at half, 15 at full, the 1/14 step) | **tested** — `the_comparator_level_is_the_jars_fraction_mapping`; in game `power=0`, `power=8`, `power=15` |
| the fraction is the jar's f32 division | **tested** — `the_fraction_is_the_jars_single_precision_division` (red was 3 vs 4 at 19,285,714/90,000,000) |
| a tower answers its whole pool, not one block's charge | **tested** — `a_towers_level_uses_the_towers_capacity_not_one_blocks`; in game 3,000,000 of 6,000,000 reads 8 where a block's fraction would read 15 |
| comparators are told only when the answer changed | **tested** — `a_comparator_is_told_only_when_its_answer_changes`; in game a settled tower's wire does not flicker (each seed's step is one change) |
| a block that is not ours answers nothing | **tested** — in game the same rig behind `minecraft:stone` reads `power=0` |
| emptying and refilling follows | **tested** — in game the emptied tower takes the wire to 0; `COMPARATOR_SELFTEST` proves that step can fail |
| goggles / display source for the gauge | **not ported** — the client half (a renderer), outside the tier's server shape |

### Deviations of this slice (declared)

1. **The answer is computed in the level pass and read back by the hook.** `analog_output` gets a
   level host that cannot open a block entity, and its doc forbids changing the world there, so the
   pass records the number and the hook answers it. A comparator therefore sees a change one tick
   after the charge moved, and a block placed and destroyed between two passes can answer once from
   the map (a reloaded chunk's comparator may ask before the first pass — the entry is kept
   deliberately for that case).
2. **The map is keyed by dimension id and position, and it is not pruned on break.** One entry per
   placed accumulator; a broken block's answer stays until the process ends. It is the answer the
   block would give, and pruning needs a break hook this Part does not have.
   *(Fixed in 0.2.11 — the answer now goes with its block, its chunk and its level; § *Slice 12*,
   after the reviewer held 0.2.6/0.2.7 for exactly this.)*
3. **Every member answers; upstream answers through the controller.** Upstream's
   `getAnalogOutputSignal` maps the block entity to `getControllerBE()`, so a non-controller part
   reports the controller's fill; here each block of the tower answers the tower's fill directly.
   The number is the same; the e2e reads the middle block to pin it.
4. **The f32 fraction is deliberate.** The jar's `getFillState()` is single precision; the port keeps
   that step so the two agree bit for bit, including the 18 charges of a 45-block tower where a
   double would read one level low. A test pins it.
5. **The comparator's state is not touched.** The port answers the vanilla `getAnalogOutputSignal`
   path only; the comparator block itself is vanilla, so its model, mode and delay are the game's.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port, rust-review

---

## Slice 7 — the energy meter: **blocked, not in the artefact**

### The slice the brief named

The 0.2.6 report's next-slice proposal and this port's `## Gaps` between them named:

> **`portable_energy_interface`**, **`energy_meter`**, **`electric_pump`**, **`digital_adapter`** …

The brief for 0.2.7 fixed `energy_meter`: place it in the
`accumulator – connector – wire – connector – accumulator` rig, run 1,000 FE/t for 20 ticks, then
read its `/createaddition info` **and** the comparator wire's `power`.

### Why it cannot be built: the artefact does not contain it

`AGENTS.md` ("A brief may not invent a mod's contents from its name … when a brief and the
artefact disagree, the artefact wins and the agent stops and reports") and the brief's own stop
condition ("Stop and report instead of guessing if the gap list has nothing implementable") apply.
Evidence, all in `ports/_shared/evidence/createaddition/energy-meter-artefact.log`:

| check | result |
|---|---|
| `sha256sum`/`stat` of the audited jar | `createaddition-1.7.2.jar`, `71e9c3ac…3414`, 1,772,844 B |
| jar entries matching `meter` | only `assets/createaddition/textures/item/multimeter.png` (an orphan texture) |
| jar entries under `blocks/energy_meter` | **0** |
| the jar's 25 blockstates | no `energy_meter` (they do include `redstone_relay`, `small_light_connector`, `creative_energy`) |
| the jar's `en_us.json` | no `energy_meter`/`multimeter` key |
| upstream repo `658a390` (1.21.1, analysis only) | `EnergyMeterBlock.java`, `EnergyMeterBlockEntity.java`, `EnergyMeterRenderer.java` all open with `/*` — **commented out** |
| registrations of `EnergyMeter`/`ENERGY_METER` outside that folder | **0** |

The family is 1.7.3-dev dead code: `gradle.properties` is `mod_version=1.7.3`, the pack jar is
`1.7.2`, and `EnergyMeterBlock.newBlockEntity` even points at `CATileEntities.REDSTONE_RELAY`, a
copy-paste leftover. Nothing ships it: no block, item, blockstate, model, loot table, lang key or
recipe. Implementing it would be inventing a feature the mod does not have, which the brief and
`AGENTS.md` forbid.

Contrast with the four slices already delivered: the accumulator's `hasAnalogOutputSignal` /
`getAnalogOutputSignal` are in the 1.7.2 jar (its class strings), and `redstone_relay` and
`small_light_connector` are in the jar's blocks, blockstates and models. Every earlier slice is in
the artefact; this one is not.

### What was and was not done

- **No code was written**, so there is no red to quote and no green to claim. TDD's "no production
  code without a failing test" never started.
- **No version bump, build, `check`, `fits` run or e2e leg** was produced for 0.2.7. The delivered
  package stays **0.2.6**, 172,461 B, sha256 `9daebe6d…56f4`; its last recorded gate is
  `0a875da  made for` (pin) / `0b14f8a  does not fit … BlockStates::light_emission (removed)` (the
  0.2.5 lamp's surface, unchanged).
- The port's sources, `horizon-mod.toml`/`Cargo.toml` version and package are untouched.

### Coverage

| family | status |
|---|---|
| `energy_meter` (the brief's slice) | **blocked(energy_meter-not-in-jar)** — not a block, item, asset or key in `createaddition-1.7.2.jar`; commented-out dead code upstream |

### Next slice

**`creative_energy`** (the "Creative Generator"), the one remaining FE family that *is* in the jar
and needs no Create network: `assets/createaddition/blockstates/creative_energy.json`, its two
models and item model, its loot table and lang all exist, and the classes are live. (Its Java class
extends Create's `CrateBlock`, but only for the block's shape; the energy behaviour is
self-contained.) Gap line: "**`creative_energy`**, the e2e's source of last resort: the harness seeds
an accumulator with `/createaddition energy` instead." Tier `deep` fits: a block entity whose
`ENERGY` capability answers infinite stored/capacity and `extract == maxExtract`, plus a per-tick
push into every neighbouring storage (upstream `CreativeEnergyBlockEntity.tick` →
`ies.receiveEnergy(Integer.MAX_VALUE, false)`). It is a source rather than a rule, but it is the last
implementable FE family and it retires the "no in-game source of last resort" gap.

Assertion for that slice: place `createaddition:creative_energy` beside a `push` connector linked to
a second connector on a `modular_accumulator`; `ctl block` at the creative block reads
`createaddition:creative_energy`, the source connector settles `mode pull`, and the accumulator's
`/createaddition info … #tag` rises by **20,000 FE over 20 game ticks** (1,000 FE/t on the wire,
measured after the connector's window has filled so the one-time window is excluded). Differential:
the same rig with `minecraft:stone` in place of the creative block moves nothing; a
`CREATIVE_SELFTEST=1` leg inverts the gain assertion and exits 1.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices (loaded; no Rust change — slice blocked before code)

---

## Slice 8 — the creative generator (0.2.7)

### The slice, and the gap line it comes from

The 0.2.6 report's `## Next slice` (and `## Slice 7`'s proposal) named it:

> **Gaps**: "**`creative_energy`**, the e2e's source of last resort: the harness seeds an
> accumulator with `/createaddition energy` instead."

**Choice.** It is the one remaining family of the jar's FE half that is *in the artefact* and needs
no Create: `assets/createaddition/blockstates/creative_energy.json`, its two block models and item
model, its loot table and three `en_us` keys are all there, and `CreativeEnergyBlock` /
`CreativeEnergyBlockEntity` / `CreativeEnergyStorage` are live classes (unlike `energy_meter`, §
[`## Slice 7`](#slice-7--the-energy-meter-blocked-not-in-the-artefact)). It is a source rather than
a rule, but it retires "there is no in-game source of last resort" and it is provable in game with
a rate assertion that can fail.

**Tier fit.** `deep`: one more member of the existing block entity type, a capability handler
registered ahead of the game's (`Registrar::block_capability_before_game`), a per-tick neighbour
pass, and carried content (its state, models, two **generated** textures, loot table, lang, block
declaration and tag entry). No renderer, no dimension, no entity AI.

**Upstream behaviour read** (repo `658a390b…`, analysis only; `FINDINGS.md` → "Creative
generator"): `CreativeEnergyStorage` (stored/capacity `Integer.MAX_VALUE`, `extractEnergy` returns
`maxExtract`, `receiveEnergy` returns 0, `canExtract` true, `canReceive` false) and
`CreativeEnergyBlockEntity.tick` (per side, `ies.receiveEnergy(Integer.MAX_VALUE, false)`).
`CreativeEnergyBlock` extends Create's `CrateBlock` only for its look; the shape is
`CAShapes.shape(1,0,1,15,16,15).add(0,2,0,16,14,16)`.

### What it does

- **An unlimited source.** `CreativeSupply` answers the `ENERGY` capability for
  `createaddition:creative_energy` ahead of any declaration: `amount`/`capacity` are `u32::MAX`,
  `insert` returns 0, `extract` returns what is asked. There is no saved charge and no
  `block_energy` declaration. A connector mounted on it settles **pull** (its declaration gives and
  never takes), and a wire drains it at the connector's 1,000 FE a tick.
- **A per-tick fill of every neighbour.** After the towers have formed and before the wire
  network's moves, each generator offers all six neighbours `creative_offer() = u32::MAX`; the
  engine's `insert` caps each at what that neighbour takes. A connector's 1,000 FE window fills; an
  accumulator beside it takes 5,000 FE a tick. Re-read every pass — no capability cache to
  invalidate (deviation 11).
- **Content.** The jar's own one-variant blockstate and two-element crate model (its elements, UVs
  and display untouched), the item model and 26.3 item definition, the loot table (26.3 dialect),
  the block declaration with the jar's `CREATIVE_ENERGY_SHAPE`, the harvest-tag entry and the lang
  keys. The model's three texture slots name **Create's** `crate_creative`/`creative_fluid_tank`;
  since Create is not installed and its art is not in this artefact, `carry_content.py` **draws**
  two deterministic 16×16 PNGs (`block/creative_energy/{crate,tank}.png`) and points the model at
  them (deviation 12).
- **No new API.** `block_capability_before_game`, the `ENERGY` trait and `LevelRef::capability` are
  all at the pin; this slice adds no surface (the second `fits` line is red only for the 0.2.5
  lamp's `light_emission`, below).

### Red → green (TDD)

The new rules went in first, against stubs (`CREATIVE_SUPPLY` 0, `creative_offer` 0, the handler
returning 0). `creative-red.log`, verbatim:

```
thread 'network::tests::the_creative_generator_stores_and_gives_without_limit' (2434605) panicked at src/network.rs:3815:5:
assertion `left == right` failed: the creative generator's stored and capacity are the largest number, as upstream's Integer.MAX_VALUE is
  left: 0
 right: 4294967295
thread 'network::tests::the_creative_generator_wire_rig_delivers_a_thousand_fe_a_tick' (2434606) panicked at src/network.rs:3889:5:
assertion `left == right` failed: the twenty ticks after the window filled deliver exactly 1,000 FE each: 20,000 in all
  left: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
 right: [1000, 1000, 1000, 1000, 1000, 1000, 1000, 1000, 1000, 1000, 1000, 1000, 1000, 1000, 1000, 1000, 1000, 1000, 1000, 1000]
thread 'network::tests::the_creative_generator_offers_every_neighbour_everything' (2434604) panicked at src/network.rs:3856:5:
assertion `left == right` failed: upstream offers Integer.MAX_VALUE and the neighbour's own insert caps it, so asking for less would cap a large sink
  left: 0
 right: 4294967295
test result: FAILED. 77 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out
```

Green: **80 passed, 0 failed** (`sdk-test.log`, regenerated for 0.2.7) — the four new tests are the
unlimited supply (`extract` gives what is asked, `insert` takes nothing), the connector's settled
`pull` from every face and the give-never-take declaration, the `u32::MAX` offer, and the wire rig
in the rules (first tick primes the double buffer and delivers nothing, the next twenty deliver
1,000 each — 20,000). `src/runtime.rs` is untouched (`03721ca4…f962`, packed too).

### The 0.2.7 ladder

| Step | Command | Result |
|---|---|---|
| tests | `cargo test --offline` | **80 passed**, 0 failed (`sdk-test.log`, regenerated for 0.2.7) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | clean (`sdk-fmt.log`; `src/runtime.rs` unchanged, `03721ca4…f962`) |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `target/horizon-mod/createaddition.hmod`, exit 0 (`creative-build-1.log`) |
| package, second build | same again | **byte-identical** (`cmp`, `creative-build-2.log` vs `creative-build-3.log`) |
| registry check | `cargo horizon-mod check createaddition.hmod` | exit 0 (`creative-check.log`) |
| fits, campaign pin | `cargo horizon-mod fits … --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a` | `0a875da  made for`, exit 0 (`creative-fits-pin.log`) |
| fits, another build | `… --build 0b14f8acfd97fa2c750e42795249dcc981178ca4` | **`does not fit: 1 item changed — the method BlockStates::light_emission (removed)`**, exit 1 (`creative-fits-other.log`) — 0.2.5's finding, unchanged |
| data shape | `python3 tools/data_shape.py createaddition.hmod` | 6/6 loot tables in the 26.3 dialect |
| content carry | `python3 tools/carry_content.py --check` | `assets/ and data/ match the jar carry` |
| licence notice | `python3 tools/check_licence.py` | `ok … carries the upstream MIT notice verbatim` |
| package verifier | `python3 ports/tools/verify-sdk-package.py createaddition` | **1/1** (`verify-sdk-package.log`) |

The second `fits` run is red for 0.2.5's reason, not this slice's: the lamp's `light_emission`
surface does not exist at `0b14f8a`, so no earlier build can carry the package. The creative
generator itself needs no new surface.

Deliverable: **`createaddition.hmod`, 178,357 B, SHA-256
`8387f865ed975963a9b7bd5b1d1d602434977c43a288dc0399e4cdbf962ad9b6`**, **117 entries**, reproducible
(three builds, two `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f962`, packed too). 86 `assets/` and 20 `data/` files (106 carried + generated; was 79 +
18).

```
createaddition 0.2.7 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  178357 bytes, SHA-256 8387f865ed975963a9b7bd5b1d1d602434977c43a288dc0399e4cdbf962ad9b6, unsigned
```

```
Create Crafts & Additions:
0a875da  made for
Create Crafts & Additions:
0b14f8a  does not fit: 1 item changed
    the method `BlockStates::light_emission` (removed)
```

The launcher rebuilt the game for this package (`native-0a875da-a4f96b049501`) and printed
`Compiling createaddition (1 of 1)`: the manifest bump is again what forces the recompile.

### In-game (`e2e-creative-default.log`, `PASS` + `EXIT:0`; `== L`)

The rig is `generator(170) – connector(171, facing west, settles pull) – wire – connector(175,
push) – accumulator(176)` on its own stone pad, and the direct pair `generator(166) –
accumulator(167)` beside it; the player stands within reach for every read.

```
== L: the creative generator gives without limit; a wire carries 1,000 FE a tick ==
  Connector at 171,100,0: mode pull, buffer 1000/1000 FE, links [], pool 0/0, demand 0, t=2606
  the connector's own state reads createaddition:connector[facing=west,mode=pull]
  Creative Generator at 170,100,0: infinite FE (t=2616)
  Modular Accumulator at 176,100,0: 0/2000000 FE (t=2625)
  mode 175 push: Mode set to push.
  link 171-175 (gold): Nodes linked.
  Modular Accumulator at 176,100,0: 13040/2000000 FE (t=2658)
  Modular Accumulator at 176,100,0: 43040/2000000 FE (t=2688)
  the connector pulls from the generator and the sink gained 30000 FE over 30 game ticks (exactly 1000/tick; 20000 FE in the 20-tick window)
  -- the differential: stone where the generator was, the same wire, moves nothing
  Modular Accumulator at 176,100,0: 0/2000000 FE (t=2731)
  Modular Accumulator at 176,100,0: 0/2000000 FE (t=2762)
  stone in the generator's place: the sink stayed at 0 FE over 31 game ticks
  -- the generator's own tick: an accumulator beside it fills with no connector at all
  Modular Accumulator at 167,100,0: 50000/2000000 FE (t=2796)
  Modular Accumulator at 167,100,0: 190000/2000000 FE (t=2824)
  the direct pair: the accumulator gained 140000 FE over 28 game ticks at 5,000/tick, no connector involved
```

Why these can fail: the sink's gain is asserted to be **exactly** `1,000 × (t_after − t_before)`
(not "some energy"), so a source that gave 2,000 or 0 a tick fails; the 20,000 is the minimum over
the 20-tick wait. The stone differential changes **only** the source block — the wire, the push
connector and the sink are the same — so anything that moved the sink without the generator would
fail there. The direct pair separates the two mechanisms: with no connector, the accumulator can
only gain from the generator's own tick, and it gains exactly 5,000 a tick (its insert cap), which
a package without the per-tick push cannot do.

| log | verdict | what it proves |
|---|---|---|
| `e2e-creative-default.log` | `PASS` + `EXIT:0` | A–K unchanged (the relay still carries 25,020 FE conserved, the lamp 1 FE/t, the comparator 0/8/15), plus **L**: the generator's id and `infinite FE` readout, a connector settles `pull` (state and readout), exactly 1,000 FE/t into the sink over 30 ticks, stone in its place moves nothing, and a generator beside an accumulator fills it at 5,000/t with no connector; **H** now breaks all six blocks |
| `e2e-creative-selftest.log` | `FAIL` + `EXIT:1` | `FAIL: CREATIVE_SELFTEST: the creative generator supplied 31000 FE, so the assertion is alive` (L's own counterfactual) |
| `e2e-creative-suite-selftest.log` | `FAIL` + `EXIT:1` | the suite's conservation counterfactual, still alive (`SELFTEST: the conservation assertion did not trip`) |
| `e2e-creative-control.log` | `FAIL` + `EXIT:1` | a game without the package cannot give the accumulator item (`ctl give` exits 1), and the default run's control leg also refuses `createaddition:creative_energy` by id (`the control's setblock left minecraft:air where the creative generator would be`) |

`python3 ports/tools/check-e2e-logs.py createaddition createaddition-e2e --strict`:
**logs=29 ok=29 missing-exit=0 bad-exit=0** (the 25 earlier legs still green).

### Coverage added by this slice

| family | status |
|---|---|
| the unlimited `ENERGY` supply (stored/capacity, gives any ask, takes nothing) | **tested** — `the_creative_generator_stores_and_gives_without_limit`; in game `Creative Generator at 170,100,0: infinite FE` and the wire's fill |
| a connector on the generator settles `pull` | **tested** — `a_connector_mounted_on_the_creative_generator_settles_pulling` (every face); in game `connector[facing=west,mode=pull]` and `mode pull` |
| the per-tick offer to every neighbour | **tested** — `the_creative_generator_offers_every_neighbour_everything`; in game the direct pair gains exactly 5,000 a tick |
| a wire from the generator at 1,000 FE/t | **tested** — `the_creative_generator_wire_rig_delivers_a_thousand_fe_a_tick` (20,000 over 20 ticks); in game `30000 FE over 30 game ticks` |
| the differential: no generator, no gain | **tested** — in game stone in its place leaves the sink at 0 over 31 ticks |
| the generator's loot table | **tested** — H (`createaddition:creative_energy: broke and dropped 1`) |
| the block state, model and item | **tested** — placed and read back (`ctl block`), given as an item, drops itself; the two textures are generated and packed |
| the generator's recipe | **not ported** — it needs `c:ingots/brass`/`create:` items (`CONTENT.md`) |
| the Create crate's own look (Create's textures) | **not carried** — generated stand-ins (deviation 12) |

### Deviations of this slice (declared)

1. **The supply is a registered handler, not a saved charge** (also `FINDINGS.md` 10). There is no
   `block_energy` for the block and `/createaddition energy` refuses it; the readout says
   `infinite FE`.
2. **The six neighbours are re-read every pass** (`FINDINGS.md` 11): upstream caches
   `BlockCapabilityCache`s and refreshes them on `neighborChanged`; a package's pass already walks
   the tick queue, so there is no cache to invalidate.
3. **Two generated textures** (`FINDINGS.md` 12): the jar's model names Create's crate and tank
   art, which is not in this artefact. The two 16×16 PNGs are drawn by `carry_content.py`, so
   `--check` reproduces them and no other mod's assets ship.
4. **The push runs in the level pass, not in the block entity's own tick.** A block entity is out
   of the world during its tick at this pin, so the queue is the only route; the pass runs it after
   the towers form (an adjacent accumulator answers the pool) and before the wire networks move.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-review

---

## Slice 9 — the festive wire and spool (0.2.8)

### The slice, and the gap line it comes from

This port's own gap list named it, and 0.2.7's report proposed it:

> **Gaps**: "**Festive wire and spool**, the iron wire, and the electrum/brass/copper metal
> items."

**Choice.** The festive wire is the jar's **fourth** wire kind and the last wire family that needs
no Create: `WireType.FESTIVE` is live, `assets/createaddition/models/item/festive_spool.json` and its
5,660-byte texture are in the jar, the lang keys and the class strings are there, and the only rule
it adds is one `if` in `IWireNode.connect`. The **iron wire** is carried with it as what 1.7.2
actually ships — an item whose `WireType.IRON` *and* `IRON_SPOOL` are commented out, so there is no
iron wire kind and no iron spool (proved in game, § *In-game*). The electrum/brass/copper metal
items stay a gap: they are crafting intermediates whose only recipes need `c:plates/iron` and
`create:` items, so they would be inert.

**Tier fit.** `deep`: a fourth wire kind in the saved link format and the link rule, the empty
spool's refund keyed by wire kind, two carried items with their models, textures, item definitions,
declarations and lang, and an in-game proof of the rule. No block, no renderer, no dimension.

### The wire types, as `WireType.java` reads them

```
COPPER(0, 256, 78, 37, 30, copper_wire x4, copper_spool)
GOLD(1, 1024, 98, 83, 29, gold_wire x4, gold_spool)
ELECTRUM(2, 8196, 88, 66, 37, electrum_wire x4, electrum_spool)
FESTIVE(3, 256, 26, 94, 12, copper_wire x4, festive_spool)
//IRON(4, 256, 87, 87, 87, CAItems.IRON_WIRE.asStack(4), CAItems.IRON_SPOOL.asStack());
```

- **The low/high-current sets are not fields**: `IWireNode.connect` says
  `if(wn1.getConnectorType() == ConnectorType.Large && wn2.getConnectorType() == ConnectorType.Large) { if(type == WireType.COPPER || type == WireType.FESTIVE) return WireConnectResult.REQUIRES_HIGH_CURRENT; }`.
  So **low current = {COPPER, FESTIVE}** (refused between two large connectors) and **high current
  = {GOLD, ELECTRUM}**; a large↔small pair takes any of the four.
- **There is no loss rate per wire kind.** `transfer()` (256 / 1024 / 8196 / 256) is read by
  **nothing** in 1.7.2 — `grep -rn "\.transfer\b" src/main/java` finds only the getter — so the
  throughput is the connector's (1,000 small / 5,000 large) and every wire, festive included,
  carries exactly that. Already deviation 3 of this port; this slice re-measured it in game:
  1,000 FE a tick over a festive wire.
- `getDrop()` is `copper_wire x4` for festive (a broken festive wire drops copper wires);
  `getSourceDrop()` is the **festive spool**, which is what the empty spool hands back.

### What it does

- **`createaddition:festive_spool`** — carried verbatim (its own model, its 5,660-byte texture, the
  26.3 item definition, the `copper_ingot` declaration and the jar's six lang keys), and it lays
  the jar's festive wire: `Laying::Festive`, saved as `type: "festive"`, printed by
  `/createaddition info` as `(x,y,z)/node/festive`, and accepted by `/createaddition link … festive`.
- **Low current, as copper.** `Laying::Festive.rule()` is [`Wire::Copper`], so the inherited link
  rule refuses festive between two large connectors and accepts it everywhere else.
- **A festive spool comes back.** The empty spool coils each wire onto its own spool, so removing a
  festive link hands back one `createaddition:festive_spool` (`getSourceDrop()`), not a copper one.
- **`createaddition:iron_wire`** — carried as content only: the jar registers the item and
  `c:wires/all_metal` + `c:wires/iron` name it, while `WireType.IRON` and `CAItems.IRON_SPOOL` are
  commented out. `/createaddition link … iron` therefore refuses (`The wire is copper, gold,
  electrum or festive.`), which is the in-game proof that no iron kind was invented.
- **No new API.** Nothing in this slice needs a hook the pin lacks; the second `fits` line is red
  only for the 0.2.5 lamp's `light_emission` (below).

### The wire kinds live in `network.rs`, not `runtime.rs`

`src/runtime.rs` must stay byte-identical to the retired bundle's (this port's `AGENTS.md`, and
every earlier slice kept it so: `03721ca4…f962`, packed too). Its [`Wire`] enum is the three-kind
table the inherited link rule was written against, so the jar's fourth kind is `network::Laying`
in the glue layer: `Copper | Gold | Electrum | Festive`, with `rule()` mapping festive onto copper's
rule, `name()`/`of_name()` for the saved links, and `SavedWire.wire: Laying`. The mapping is
unit-tested (`festive carries copper's rule`), so the two tables cannot drift silently.

### Red → green (TDD)

The rules went in first as the pre-slice behaviour — festive keyed by the rule it *would* borrow
(`Wire::Gold`, as if festive were the other 256-transfer family's high-current sibling) and the
refund keyed by that rule — with the tests written against `WireType`. `festive-red.log`, verbatim:

```
thread 'network::tests::a_festive_wire_is_refused_between_two_large_connectors_but_gold_is_not' (128257) panicked at src/network.rs:3927:5:
assertion `left == right` failed: festive is refused between two large connectors exactly as copper is
  left: Linked
 right: Refused(RequiresHighCurrent)
thread 'network::tests::an_empty_spool_hands_back_one_spool_of_each_wires_own_kind' (128282) panicked at src/network.rs:3856:5:
assertion `left == right` failed: a festive wire coils back onto a festive spool, not onto a copper one (WireType.FESTIVE.getSourceDrop() is the festive spool)
  left: [(Copper, 3), (Gold, 1), (Electrum, 0), (Festive, 0)]
 right: [(Copper, 2), (Gold, 1), (Electrum, 0), (Festive, 1)]
thread 'network::tests::the_festive_spool_lays_the_jars_fourth_wire' (128293) panicked at src/network.rs:3905:5:
assertion `left == right` failed: festive carries copper's rule: 256 like COPPER, and `connect` tests them together
  left: Gold
 right: Copper

test result: FAILED. 81 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out
```

Green: **84 passed, 0 failed** (`sdk-test.log`, regenerated for 0.2.8). The four new tests are the
wire-type table as read (four names, `festive` in and `iron` out, festive's rule is copper's), the
refusal between two large connectors with gold and electrum accepted, the saved link read back as
`festive` and printed as `(5,64,2)/3/festive`, the refund keyed by the wire's own kind, and the
spool's pending selection being the player's (below). `src/runtime.rs` is untouched
(`03721ca4…f962`, packed too).

### A port defect this slice found and fixed: the spool's first click

The first in-game probe could not lay a wire at all, and the reason was **this port's**, not the
jar's. `probe-festive.log` (0.2.7 code, the same click twice):

```
  probe: holding 1 festive spool(s), state createaddition:connector[facing=west,mode=pull]
  probe: after the first click the spool reads {"count":1,"item":"createaddition:festive_spool","slot":0}
{"components":"{\"minecraft:custom_data\":{createaddition:{node:0,x:269,y:102,z:0}}}","count":1,"item":"createaddition:festive_spool","slot":1}
  probe: after the second click the spool reads {"count":1,"item":"createaddition:festive_spool","slot":0}
{"components":"{\"minecraft:custom_data\":{createaddition:{node:0,x:269,y:102,z:0}}}","count":1,"item":"createaddition:festive_spool","slot":1}
{"components":"{\"minecraft:custom_data\":{createaddition:{node:0,x:273,y:102,z:0}}}","count":1,"item":"createaddition:festive_spool","slot":2}
  Connector at 269,102,0: mode pull, buffer 0/1000 FE, links [], pool 0/0, demand 0, t=238
```

One spool became **three**, each copy carrying a different click, and the hand kept a plain spool,
so every click was a "first" click and no wire was ever laid. The port wrote the selection with
`ItemPlace::edit`, which is an extract + insert; the insert of a stack whose components differ
lands in the next free slot, so the hand never holds the selection. Upstream mutates the stack in
place (`c.getItemInHand().set(DataComponents.CUSTOM_DATA, …)`), which the SDK does not offer.

**Fix.** The selection is the **player's** now (`network::Pending`, keyed by `PlayerRef::uuid`), not
the stack's: one selection per player (a second spool replaces it), cleared when the second click
lays or when the same block is clicked again, and not saved with the world (it is a click in
progress). The wire is still the kind of the stack that makes the second click, as
`WireType.of(heldItem)` has it. Unit-tested by `a_players_first_spool_click_is_the_mods_and_not_the_stacks`,
and the in-game proof is the phase below: one spool → one wire → exactly one empty spool.
This bug also made the *copper* spool unusable in game since 0.2.0 — no earlier leg used the item
(the commands stood in for it), so the wire family's item path had never been exercised.

### The 0.2.8 ladder

| Step | Command | Result |
|---|---|---|
| tests | `cargo test --offline` | **84 passed**, 0 failed (`sdk-test.log`) |
| red | the same against the pre-slice rule | **81 passed, 3 failed**, exit 101 (`festive-red.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `target/horizon-mod/createaddition.hmod`, exit 0 (`festive-build-1.log`) |
| package, second build | `rm -f Cargo.lock` + the same again | **byte-identical** (`cmp`) (`festive-build-2.log`) |
| registry check | `cargo horizon-mod check createaddition.hmod` | exit 0, `0.2.8 … 186895 bytes` (`festive-check.log`) |
| fits, campaign pin | `cargo horizon-mod fits … --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a` | `0a875da  made for`, exit 0 (`festive-fits-pin.log`) |
| fits, another build | `… --build 0b14f8acfd97fa2c750e42795249dcc981178ca4` | **`does not fit: 1 item changed — the method BlockStates::light_emission (removed)`**, exit 1 (`festive-fits-other.log`) — 0.2.5's finding, unchanged |
| data shape | `python3 tools/data_shape.py createaddition.hmod` | 6/6 loot tables in the 26.3 dialect |
| content carry | `python3 tools/carry_content.py --check` | `assets/ and data/ match the jar carry` |
| licence notice | `python3 tools/check_licence.py` | `ok … carries the upstream MIT notice verbatim` |
| package verifier | `python3 ports/tools/verify-sdk-package.py createaddition` | **1/1** (`verify-sdk-package.log`) |

Deliverable: **`createaddition.hmod`, 186,895 B, SHA-256
`715ce0ea799f9ad7c16d8bbc33df8b6c3fcafa2057ddf03bf60b84620a5025a4`**, **125 entries**, reproducible
(two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f962`, packed too). **114** `assets/`+`data/` files (was 106: two item models, two
textures, two 26.3 item definitions, two item declarations; the lang file grew by two keys).

```
createaddition 0.2.8 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  186895 bytes, SHA-256 715ce0ea799f9ad7c16d8bbc33df8b6c3fcafa2057ddf03bf60b84620a5025a4, unsigned
```

```
Create Crafts & Additions:
0a875da  made for
Create Crafts & Additions:
0b14f8a  does not fit: 1 item changed
    the method `BlockStates::light_emission` (removed)
```

The launcher rebuilt the game for this package (`native-0a875da-bbeb13d53095`) and printed
`Compiling createaddition (1 of 1)`: the manifest bump is again what forces the recompile.

### In-game (`e2e-festive-default.log`, `PASS` + `EXIT:0`; `== M`)

The rig hangs at eye height on its own pad at x=268…284 (`accumulator(268) – connector(269, facing
west) – festive wire – connector(273, facing east) – accumulator(274)`, plus the large pair at
280/284), and the player stands beside it: a connector's outline is the 4×4-pixel nub on the face it
is mounted on, so a click from the front must be almost horizontal — a player a block above the rig
crosses the nub's band and the ray hits nothing (the two `probe-festive.log` runs that preceded the
y=102 rig). The player is in survival from phase H's break leg, so the spool's **spend** path is the
one exercised.

```
== M: the festive spool lays the jar's fourth wire; festive is low-current like copper ==
  Connector at 269,102,0: mode pull, buffer 0/1000 FE, links [(273,102,0)/0/festive], pool 0/0, demand 0, t=2693
  the two clicks laid a festive wire: Connector at 269,102,0: … links [(273,102,0)/0/festive] …
  Connector at 273,102,0: mode pull, buffer 0/1000 FE, links [(269,102,0)/0/festive], pool 0/0, demand 0, t=2700
  both ends save type 'festive'; the one festive spool became exactly one empty spool
  -- it carries exactly 1,000 FE a tick, the same rate as copper
  Modular Accumulator at 274,102,0: 13040/2000000 FE (t=2742)
  Modular Accumulator at 274,102,0: 42040/2000000 FE (t=2771)
  the festive wire carried 29000 FE over 29 game ticks (exactly 1000/tick; 20000 FE in the 20-tick window)
  -- the differential: two large connectors refuse festive, and accept gold
  Large Connector at 280,102,0: mode none, buffer 0/5000 FE, links [], pool 0/0, demand 0, t=2805
  link 280-284 (festive): Two large connectors need a gold or electrum wire.
  festive refused between two large connectors, as copper is
  Large Connector at 280,102,0: mode none, buffer 0/5000 FE, links [], pool 0/0, demand 0, t=2819
  -- the iron wire is content only: the item exists and no wire kind does
  createaddition:iron_wire is held and /createaddition link refuses 'iron', as 1.7.2 has no IRON wire
  -- the empty spool coils a festive wire back onto a festive spool
  Connector at 269,102,0: mode pull, buffer 0/1000 FE, links [], pool 0/0, demand 0, t=2877
  the empty spool removed the festive wire and gave back one festive spool (0 -> 1)
```

Why these can fail: the wire is laid by the **item**, not the command (`ctl interact` twice with
`createaddition:festive_spool` in the hand), and both ends must save `festive` — the pre-slice code
laid nothing at all; the gain is asserted to be **exactly** `1,000 × the ticks the two reads are
apart` (29 → 29,000, i.e. 20,000 in the 20-tick wait), so a 2,000/tick or 0/tick wire fails; the
large pair is the same refusal copper gets, with **gold accepted** on the same pair in the same
tick, so a package that refused both would fail; `FESTIVE_SELFTEST=1` inverts the refusal
(`FAIL: FESTIVE_SELFTEST: festive was refused between two large connectors (Two large connectors
need a gold or electrum wire.), so the refusal assertion is alive`, `e2e-festive-selftest.log`); and
the empty spool must hand back **exactly one** festive spool (0 → 1), which is the 0.1.0
double-refund assertion on the new kind.

| log | verdict | what it proves |
|---|---|---|
| `e2e-festive-default.log` | `PASS` + `EXIT:0` | A–L unchanged in this run (the relay carried **25,020 FE** conserved, the lamp drew 1 FE/t over 43 ticks, the comparator read 0/8/15, the generator exactly 1,000 FE/t over 28 ticks), plus **M**: the festive spool lays a wire both ends save as `festive`, one spool → one empty spool, exactly 1,000 FE/t over 29 ticks, festive refused between two large connectors with gold accepted, `createaddition:iron_wire` held while `link … iron` is refused, and the empty spool returning one festive spool |
| `e2e-festive-selftest.log` | `FAIL` + `EXIT:1` | M's own counterfactual (`FESTIVE_SELFTEST`) |
| `e2e-festive-suite-selftest.log` | `FAIL` + `EXIT:1` | the suite's conservation counterfactual (`SELFTEST`), still alive |
| `e2e-festive-control.log` | `FAIL` + `EXIT:1` | a game without the package has no accumulator item; the default run's control leg also refuses `createaddition:festive_spool` and `createaddition:iron_wire` by id |
| `probe-festive.log` | `PASS` + `EXIT:0` | the probe that found the pending-selection defect (three spools, no wire) and pinned the click geometry (`FESTIVE_ONLY=1`) |

`python3 ports/tools/check-e2e-logs.py createaddition-e2e --strict`:
**logs=30 ok=30 missing-exit=0 bad-exit=0** (the 29 earlier legs still green).

### Coverage added by this slice

| family | status |
|---|---|
| the jar's four wire kinds, named and mapped to their rules | **tested** — `the_festive_spool_lays_the_jars_fourth_wire` (four names; `iron` and `Iron` are not wire kinds; festive's rule is copper's) |
| festive is low-current: refused between two large connectors, gold/electrum accepted | **tested** — `a_festive_wire_is_refused_between_two_large_connectors_but_gold_is_not`; in game M's refusal + gold on the same pair + `FESTIVE_SELFTEST` |
| a festive link is saved and read back as `festive` | **tested** — `a_festive_wire_is_saved_and_read_back_under_its_own_name`; in game both ends' `info` lines |
| the festive spool lays the wire (the item path, not the command) | **tested** — in game `ctl interact` twice with the item, `links [(273,102,0)/0/festive]` |
| the empty spool refunds a festive spool | **tested** — `an_empty_spool_hands_back_one_spool_of_each_wires_own_kind`; in game 0 → 1 |
| one wire spool becomes exactly one empty spool | **tested** — `the_last_wire_spool_becomes_the_empty_spool_and_is_not_given_twice`; in game the hand's count |
| the spool's first click is the player's, not the stack's | **tested** — `a_players_first_spool_click_is_the_mods_and_not_the_stacks`; in game the probe's before/after |
| a festive wire carries exactly the connector's 1,000 FE/t | **tested** — in game M's `29000 FE over 29 game ticks` |
| the iron wire item | **tested** — in game `createaddition:iron_wire` held, and `link … iron` refused |
| the festive spool's recipe | **not ported** — it needs `createaddition:biomass` (the rotation half) and `c:dusts/redstone` (`CONTENT.md`) |
| the iron wire's recipes / the wire→spool recipes | **not ported** — they need `createaddition:spool` and `c:plates/iron`, which come from Create (`CONTENT.md`) |
| the festive wire's string of lights (renderer) and the wire itself drawn at all | **tested in 0.2.9** — § [`## Slice 10`](#slice-10--the-wire-renderer-029) (a `[[native.mixin]]` at `horizon_mod_render::render::world_drawn`) |
| a broken festive wire dropping `copper_wire x4` (`WireType.getDrop`) | **not ported** — the port has no break-drop path for wires (0.1.0 never had one either; recorded in `## Deviations`) |
| the electrum/brass/copper metal items | **not ported** — crafting intermediates with no recipe that can match without Create |

### Deviations of this slice (declared)

1. **The four wire kinds live in `network.rs` (`Laying`), not in `runtime.rs`.** `runtime.rs` is the
   frozen inherited truth (byte-identity rule), so the jar's fourth kind is glue and borrows
   copper's rule through `Laying::rule()`; a test pins the mapping.
2. **The spool's pending selection is the player's, not the stack's** (the port defect above): one
   per player, not saved, cleared by the second click. What a player notices: two spools cannot hold
   two different selections at once, and swapping a spool for another mid-link uses the new kind.
3. **The festive wire has no renderer, like every other wire in this port.** Upstream draws the wire
   and, for festive, a string of coloured lights (`WireNodeRenderer`); a package draws through
   `mod_render::render::world_drawn`, a raw frame place outside `mod_api` that needs a native mixin —
   recorded as the next slice, not faked.
4. **A removed festive wire drops copper wires?** No: the port has no wire-drop path at all (a
   removed wire returns its spool; a broken connector loses its wires with the block entity, as
   since 0.1.0). `WireType.getDrop()` is therefore not exercised — it is the break path, and the
   break path is a gap this port already declared.

## Slice 10 — the wire renderer (0.2.9)

### The slice, and the gap line it comes from

0.2.8's coverage table named it, and its report proposed it as the next slice:

> **Gaps**: "the festive wire's string of lights (renderer) and the wire itself drawn at all —
> **not ported** — the client half; `native mod_render::render::world_drawn` is a raw frame place
> outside `mod_api` (next slice)"

**Choice.** A laid wire is *invisible* in this port: the links move energy and draw nothing, while
upstream's `WireNodeRenderer` draws every wire as a ribbon hung on the jar's catenary in its
`WireType`'s RGB, and gives the festive wire a string of coloured lights. It is the last client
half the wire family has and the one visible gap of a slice that otherwise works.

**Tier fit.** `deep`: a client renderer in the frame's own wgpu encoder — a pipeline, a vertex
buffer, the world's depth — with the jar's geometry (node offsets, catenary, ribbons, lights) as
pure, unit-tested arithmetic in the package and the link set fed from the server's level pass. No
block, no dimension, no new item.

### The route, verified at the pin before any code

The render function is **not** a `mod_api` hook at `0a875da`, so the earlier note was right about
the boundary and wrong about nothing else; the route is a `[[native.mixin]]` and was proved before
the slice:

1. `hooks/world_drawn.rs` declares `hook:world_drawn` with `target = "mod_render::render::world_drawn"`,
   side client, contract listen — and the name is **absent from the generated `Hooks` trait**
   (`crates/horizon-mod-vocab/generated/mod_api_generated.rs`, 1505 `fn`s), so a package cannot
   implement it. The catalogue entry is what a mixin names, not what a package overrides.
2. The pin's book says so: `docs/modding/src/concepts/native-route/2-mixins.md` § *Render
   functions* — "native-only, unstable raw frame places, outside `mod_api`. A native mod replaces
   one with a `HEAD` mixin at `horizon_mod_render::render::<name>`"; `reference/manifest.md`:
   "API function listeners use top-level `[[inject]]` … Native source edits use
   `[[native.mixin]]` with `id`, `target`, uppercase `kind`, `helper`".
3. The target is replaceable: `crates/horizon-mod-render/src/render.rs` carries
   `#[horizon_mod_api_macros::replaceable]` on `world_drawn` and `render_needs`, which is the
   marker `2-mixins.md` § *Naming the target* requires of a function below the game crate.
4. The packer accepts the table: `cargo horizon-mod check` (folder) exit 0 with both rows.
5. The launcher applies it without rebuilding the game: `Compiling createaddition (1 of 1)` and
   `Function replacement: 2 functions patched by 2 mixins, without rebuilding the game (copy 68 ms,
   horizon-mixins 101 ms, compile 1084 ms, total 1255 ms)` (`wire-launcher-patch.log`).

The two rows in `horizon-mod.toml`:

```toml
[[native.mixin]]
id = "wire-world-drawn"
target = "horizon_mod_render::render::world_drawn"
kind = "HEAD"
helper = "src/mixins/world_drawn.rs"
bindings = [{ name = "world", ty = "&mut WorldDrawn<'_>", value = "&mut *world" }]

[[native.mixin]]
id = "wire-render-needs"
target = "horizon_mod_render::render::render_needs"
kind = "HEAD"
helper = "src/mixins/render_needs.rs"
bindings = [{ name = "needs", ty = "&mut RenderNeeds", value = "&mut *needs" }]
```

The helper is compiled **into a copy of the game's render module** and calls this package's crate
by its name (`createaddition::render`), which is why the geometry is testable without a game and
the GPU calls live in the helper.

### What it does

- **Every laid wire, once.** The level pass reads each block's saved links, keeps the pair whose
  other end has the lower position (upstream's `shouldRenderConnection` comparison), and attaches
  each end at the jar's node: a connector's `3/16` of a block out of its mounting face, a relay's
  `5/16` along its own axis and `1/16` below its centre, with the port taken from the peer's side
  exactly as the wire's own node index is (`relay_port_for_peer`, already in the port).
- **The jar's geometry.** `WireNodeRenderer`: 24 straight pieces, a slope-aware catenary
  `sin(-part·PI) · (0.5 · distance / 16)`, a `0.025`-block cross-section, every other point at
  70 % brightness, and — festive only — a light every third point, offset `-0.03` down, cycling
  red/green/blue (the jar's `colors` and its `main` direction test).
- **The jar's colours.** `WireType`'s RGB: copper 78/37/30, gold 98/83/29, electrum 88/66/37,
  festive 26/94/12. The lights are the jar's red/green/blue at full brightness.
- **The frame's own depth.** The pass attaches `WorldDrawn::depth` (reversed, `Greater`, no write)
  so a wire is hidden by walls, and asks `render_needs` for `world_depth` — the second mixin. If
  the frame's depth is smaller than its colour target (a lower-resolution world), the wire is
  drawn without the depth test rather than into a mismatched pass.
- **One pipeline per format.** The pipeline is built once per `(color_format, depth)` and kept in
  the helper's own cache; the vertex buffer is rebuilt each frame from the pure vertex list.
- **Single player and LAN hosts.** The set is published by the level's tick pass, so an integrated
  server's client draws the links the server holds. A remote client has no copy (this port syncs
  no block-entity data), declared under deviations — the same boundary the port already has.

### Red → green (TDD)

The rules went in first as the pre-slice behaviour a renderer without the jar's constants would
have — a `WireType` colour table of `[0, 0, 0]`, no catenary (`hang` = 0) and no lights — and the
tests were written against `WireNodeRenderer`'s own numbers. `wire-red.log`, verbatim:

```
thread 'render::tests::every_wire_kind_draws_in_its_own_wire_type_colour' panicked at src/render.rs:400:5:
assertion `left == right` failed
  left: [0, 0, 0]
 right: [78, 37, 30]
thread 'render::tests::a_wire_hangs_below_the_straight_line_by_the_jars_catenary' panicked at src/render.rs:434:5:
a four-block wire sags by about 0.06, not 0
thread 'render::tests::a_festive_wire_carries_its_string_of_lights_and_the_others_do_not' panicked at src/render.rs:468:5:
a festive wire draws its lights (576 vertices against copper's 576)

test result: FAILED. 91 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out
```

Green: **94 passed, 0 failed** (`sdk-test.log`, regenerated for 0.2.9; 84 → 94: ten new tests).
They are the four `WireType` colours with the jar's 0.7 shading, the four-block catenary
(`hang(0.5, 4) = -0.0625`), the endpoints the wire links, a vertical wire without the jar's
division by zero, the festive lights (count and the red/green/blue cycle, and none on copper), the
relay's eight node offsets against `RedstoneRelayBlockEntity`'s table and a connector's six, the
cache's per-dimension replace, the uploaded vertex layout, and a wire behind the camera dropped
whole. `src/runtime.rs` is untouched (`03721ca4…f95d`).

Two defects the tests caught in this slice's own first draft: the relay's horizontal input/output
pair for an east/west facing (the jar's `OFFSET_EAST`/`OFFSET_WEST` are the *output* on the facing
side, and my first table had them the other way), and a projection test matrix written
column-major wrong (a `w` row that made every quad vanish — the test failed as "a wire in front of
the camera draws").

### The 0.2.9 ladder

| Step | Command | Result |
|---|---|---|
| tests | `cargo test --offline` | **94 passed**, 0 failed (`sdk-test.log`) |
| red | the same against the pre-slice renderer | **91 passed, 3 failed**, exit 101 (`wire-red.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | exit 0 (`wire-build-1.log`) |
| package, second build | `rm -f Cargo.lock` + the same again | **byte-identical** (`cmp`) (`wire-build-2.log`) |
| registry check | `cargo horizon-mod check createaddition.hmod` | exit 0, `0.2.9 … 187056 bytes` (`wire-check.log`) |
| fits, campaign pin | `cargo horizon-mod fits … --build 0a875da…` | **`is made for exactly one build (it has a [native.mixin] table), so fits doesn't apply`**, exit 2 (`wire-fits-pin.log`) |
| fits, another build | `… --build 0b14f8a…` | the same refusal, exit 2 (`wire-fits-other.log`) |
| launcher build | `launcher-game.sh build createaddition` | `Compiling createaddition (1 of 1)`, `Function replacement: 2 functions patched by 2 mixins`, `native-0a875da-3c69290c67cd` (`wire-launcher-build.log`; the replacement line in `wire-launcher-patch.log`) |
| data shape | `python3 tools/data_shape.py createaddition.hmod` | 6/6 loot tables in the 26.3 dialect |
| content carry | `python3 tools/carry_content.py --check` | `assets/ and data/ match the jar carry` |
| licence notice | `python3 tools/check_licence.py` | `ok … carries the upstream MIT notice verbatim` |
| package verifier | `python3 ports/tools/verify-sdk-package.py createaddition` | **1/1** (`verify-sdk-package.log`) |

**The two `fits` lines changed shape, and this is the route's price, not a regression.** 0.2.8's
pin run said `0a875da made for` and its other-build run was red for the 0.2.5 lamp's
`BlockStates::light_emission`. A package with a `[[native.mixin]]` row is bound to one game build —
its helper is compiled into a copy of that build's own source — so `fits` refuses to answer for
**either** build, naming the table. What replaces it as the compatibility evidence is the
launcher build itself: the patch compiled and applied against `0a875da` and the game ran
(`Function replacement: 2 functions patched by 2 mixins`), and repointing the pin is a deliberate
`cargo horizon-mod update` + reread of the two target functions (`2-mixins.md`: "a target that was
renamed or moved fails the build with an error naming your mod; a change elsewhere in a target's
body is not reported").

Deliverable: **`createaddition.hmod`, 187,056 B, SHA-256
`a9f4dcac05edbb72835f122f94514e034b41848aaa332266311bf8044a0e23e9`**, **127 entries**,
reproducible (two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f95d`, packed too).

```
createaddition 0.2.9 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  187056 bytes, SHA-256 a9f4dcac05edbb72835f122f94514e034b41848aaa332266311bf8044a0e23e9, unsigned
```

### In-game (`e2e-wire-default.log`, `PASS` + `EXIT:0`)

A dedicated leg (`tools/e2e-wire.sh`) rather than a phase of the A–M run: the A–M run launches
`--headless` (no renderer, no screenshots), and the wire is only visible in a rendered frame. The
leg launches `--offscreen` on the same launcher-built game, builds its own pad in the End
(chunk loaded before the `fill`), the rig `accumulator(268) – connector(269, facing west) – festive
wire – connector(273, facing east) – accumulator(274)` at y=101, puts the player four blocks south
at eye height (settled on the ground: a player still falling moves the camera and the whole scene
with it) and looks at the point the wire hangs from.

```
== lay the festive wire ==
  Connector at 269,101,0: mode pull, buffer 0/1000 FE, links [(273,101,0)/0/festive], pool 0/0, demand 0, t=147
  camera settled at 271.5,101.0,4.5
  with the wire: settled (two frames twenty ticks apart differ by 0 pixels)
  with the wire: 1901 green-dominant pixels in the wire box, mean rgb(21, 86, 9)
== take the wire out and take the same frame again ==
  Connector at 269,101,0: mode pull, buffer 0/1000 FE, links [], pool 0/0, demand 0, t=572
  without the wire: settled (two frames twenty ticks apart differ by 0 pixels)
  without the wire: 0 green-dominant pixels in the same box
  (the frames differ in 2002 pixels of the box overall, mean rgb(26, 82, 15): the floor's tone, not the wire)
  frame size: 1280 x 720; the changed pixels' box: x 356..923, y 514..538
PASS: the festive wire is drawn between its two connectors (1901 green pixels, mean rgb(21, 86, 9)) and is gone (0) once /createaddition unlink takes it out
```

Why these can fail: the count is of **festive-green** pixels in the wire's own screen box —
rgb(21, 86, 9) is the jar's 26/94/12 shaded by its own 0.7, and nothing else in the End's void and
this stone floor is green-dominant, so a frame drawn by a package that draws nothing counts 0
(which is what `NEGATIVE=1` asserts and why it must fail). The box's 567 × 24 pixels are the wire's
span (the sag and the lights), not the whole frame. The unlink's own effect is asserted twice: the
connector's `info` line reads `links []` **and** the second frame's count is 0.

| log | verdict | what it proves |
|---|---|---|
| `e2e-wire-default.log` | `PASS` + `EXIT:0` | the festive wire is drawn at its own colour (1901 green pixels, mean rgb(21,86,9)) and is gone (0) after the unlink |
| `e2e-wire-negative.log` | `FAIL` + `EXIT:1` | `NEGATIVE`: the no-wire frame carries 0 green pixels, so a renderer that draws nothing fails this leg |
| `e2e-wire-control.log` | `FAIL` + `EXIT:1` | a Nightly without the package does not know `createaddition:connector` (the setblock left `minecraft:air`), so the wire the modded frame counts comes from this package |
| `e2e-wire-suite.log` | `PASS` + `EXIT:0` | the whole A–M suite on 0.2.9: the level pass that publishes the wires did not disturb a single earlier family |
| `e2e-wire-suite-selftest.log` | `FAIL` + `EXIT:1` | the suite's conservation counterfactual is still alive |

`python3 ports/tools/check-e2e-logs.py createaddition-e2e --strict`:
**logs=35 ok=35 missing-exit=0 bad-exit=0** (the 30 earlier legs still green).

Harness facts this slice cost a run each (kept here because the next screenshot leg will meet
them): the chat log is drawn over the frame and its text changes with every command, so a raw
frame difference measures the chat (86,413 changed pixels) — the wire box starts below it and the
log is left to fade; the engine's own brightness shifts the floor's tone between two shots
(25,600 changed pixels, all lighting), so the assertion is a colour count and the difference is
only reported; chunks keep meshing after the camera arrives, so a frame is used only once two
shots twenty ticks apart agree; and a teleport one block above the floor leaves the player falling,
which moves the camera and the whole scene (45,786 changed pixels of floor in the first attempt).
The launcher reuses a build when a package's files are unchanged, so the two `native-*` tags that
carried 0.2.5 and 0.2.8 (`afdac794d1f6`, `bbeb13d53095`) were moved out of the versions folder
before the build; the new tag is `3c69290c67cd` and the build log shows both the `Compiling
createaddition (1 of 1)` line and the two patched functions.

### Coverage added by this slice

| family | status |
|---|---|
| every laid wire is drawn, once, between its two nodes | **tested** — in game: 1901 green pixels in the wire box with the wire, 0 without it |
| the `WireType` colour of each kind (copper/gold/electrum/festive) | **tested** — `every_wire_kind_draws_in_its_own_wire_type_colour`; in game the mean is rgb(21, 86, 9) ≈ festive 26/94/12 × 0.7–1.0 |
| the jar's catenary (24 pieces, `HANG`, the 16-block divisor) | **tested** — `a_wire_hangs_below_the_straight_line_by_the_jars_catenary` (midpoint −0.0625 for a 4-block wire) |
| the festive string of lights (every third point, red/green/blue) | **tested** — `a_festive_wire_carries_its_string_of_lights_and_the_others_do_not`; visible in the frame |
| a connector's node offset (3/16 out of its face) | **tested** — `a_connectors_node_is_three_sixteenths_out_of_its_mounting_face` |
| a relay's eight node offsets, horizontal and vertical | **tested** — `a_relays_eight_nodes_are_where_redstone_relay_block_entity_puts_them` |
| the wire set follows the links (and clears when they go) | **tested** — `a_wire_leaves_the_client_cache_when_the_level_it_belongs_to_changes`; in game 1901 → 0 after the unlink |
| a vertical wire has no division by zero | **tested** — `a_vertical_wire_is_drawn_without_a_division_by_zero` |
| the wire is hidden by walls (the world's depth) | **not proven in game** — the pass attaches `WorldDrawn::depth` with reversed comparison, but this leg's wire hangs in the open; a wall between camera and wire is the assertion that would prove it |
| a wire on a remote (dedicated/LAN guest) client | **not ported** — the port syncs no block-entity data, so a remote client has no links to draw (deviation 5) |
| the wire being placed (the spool's held-wire line to the player) | **tested in 0.2.15** — § [`## Slice 16`](#slice-16--the-held-wire-preview-0215): `clientRenderHeldWire` drawn on the same `world_drawn` pipeline; in game 209 green pixels with a selection, 0 after the second click and 0 with none |
| the lightmap on a wire (block/sky light interpolation) | **not ported** — the wire is drawn at its type's colour without the jar's `LightTexture` interpolation (deviation 6) |

### Deviations of this slice (declared)

1. **The renderer is a `[[native.mixin]]`, not a hook.** At this pin the render functions are
   native-route places outside `mod_api` (see the route check above). The consequence: `fits` no
   longer answers for the package, and repointing the pin means rereading `world_drawn` and
   `render_needs` at the new commit (`2-mixins.md`: a change inside a target's body is not
   reported). One packaging consequence as well: a native-route package has **no uses table**, so
   0.2.8's `uses.json` is gone from the archive — the entry count is 125 + 3 new sources − 1
   `uses.json` = 127.
2. **The pipeline is cached in the helper, keyed by the frame's colour format and depth choice.**
   A format change (an HDR toggle) rebuilds it once; nothing else does.
3. **Without a matching depth target the wire is drawn over the world.** When the frame's depth is
   smaller than its colour target (a lower-resolution world), the pass cannot attach it and the
   wire is not occluded. At 100 % resolution, which is the leg's and the usual case, it is.
4. **The vertex list is rebuilt every frame** rather than kept in a GPU buffer: a wire network is a
   handful of ribbons, and a buffer per frame costs one small allocation.
5. **A remote client draws no wires.** The set is published by the level's tick pass in the
   server's process; a dedicated server's client has no copy (this port has no block-entity sync
   at all — the same boundary as its links, modes and charges on a client-only load).
6. **No lightmap.** Upstream interpolates block and sky light along the wire; this port draws the
   wire at its `WireType` colour (shaded 0.7 every other point), so a wire in a dark room is as
   bright as one in daylight. The colour, the shape and the lights are the jar's.
7. **The lights are two crossed flat quads, not the jar's `SMALL_LIGHT` model.** They cycle the
   jar's red/green/blue every third point at the jar's `-0.03` offset; the model's own shape and
   its per-light scale are not carried.
8. **A wire whose peer this tick did not read is aimed at the peer's block centre**, as upstream
   does for an unloaded other end (`WireNodeRenderer.render`: `getNodeOffset` or `Vec3.ZERO`).


Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-review

---

## Slice 11 — the wires a broken connector drops (0.2.10)

### The slice, and the gap line it comes from

0.2.9's report proposed it as the next slice, and its own words name the gap:

> **Gaps**: "the port has no wire-drop path at all … `WireType.getDrop()` is therefore not
> exercised"

**Choice.** A laid wire could be *taken out* (the empty spool, 0.2.1) but never *lost*: breaking
the connector it hung on left the wire in the peer's list and dropped nothing, while upstream drops
four wire items at the broken block and empties the peer's node. It is the last server half of the
wire family, and the first slice that exercises `WireType`'s two drop tables.

**Tier fit.** `deep`: one more Part method on the three wire blocks (a block-destruction moment, a
level write per peer), the jar's two item tables, and carried items that already exist
(`copper_wire`, `gold_wire`, `electrum_wire`). No renderer, no entity, no new block.

### The drop rules, read from the artefact before implementing

Repo `658a390b8ff4daa17013e9447b29949330c8a42a` (analysis only; the port is a clean-room rewrite):

| file | what it says |
|---|---|
| `energy/WireType.java:8-13` | `COPPER(0,256,78,37,30, COPPER_WIRE.asStack(4), COPPER_SPOOL)`, `GOLD(1,1024, …, GOLD_WIRE×4, GOLD_SPOOL)`, `ELECTRUM(2,8196, …, ELECTRUM_WIRE×4, ELECTRUM_SPOOL)`, `FESTIVE(3,256,26,94,12, **COPPER_WIRE×4**, FESTIVE_SPOOL)` — the festive wire's **drop is a copper wire**; only its *source* drop is a festive spool |
| `energy/WireType.java:47-53` | `getDrop()` / `getSourceDrop()` are `DROP.copy()` / `SOURCE_DROP.copy()` |
| `blocks/connector/base/AbstractConnectorBlock.java:83-92` | `playerWillDestroy`: server side, an `IWireNode` gets the wire-break sound when it `hasAnyConnection()`, then `cte.dropWires(level, !player.isCreative())` |
| `energy/IWireNode.java:300-318` | `dropWires(Level, boolean)`: `false` ⇒ `disconnectWires()` only (no items); `true` ⇒ one stack **per `WireType` index** (`NonNullList.withSize(WireType.values().length)`, first `type.getDrop()`, then `grow(getDrop().getCount())`), each `dropWire(world, getPos(), stack)` = `Containers.dropContents` **at the broken block** |
| `blocks/connector/base/AbstractConnectorBlockEntity.java:~320-350` | the block entity's `remove()`: each peer's reciprocal node is `removeNode(ourNode)` **without** a drop when our node is already invalid — the peer's list empties and nothing drops twice |
| `energy/IWireNode.java:320-352` | the **three-arg** `dropWires(Level, Player, boolean)` (an empty spool in the pack ⇒ `getSourceDrop()`, else `getDrop()`; `getInventory().add` else drop) is `onSneakWrenched`'s route, not the break's; the port has its spool half since 0.2.1 (`remove_wires`/`spools_back`) |

So the break path is: **four wire items of the wire's kind per wire, one stack per kind, at the
broken block, only outside creative, and the peer's link removed without a second drop.**

### The route at the pin

`hook:on_destroyed_by_player` (`IBlockExtension.onDestroyedByPlayer`, contract *replace*, scope
*block*) is a **`BlockPart` method**: asked of our blocks only, *before* anything of the break
changes the world, with `player: u128`, `pos`, `state`, `tool`, `will_harvest`. Answering `None`
leaves vanilla's removal and the block's own loot table alone. The breaking player is
`host.player(uuid)` (a `PlayersHost` lookup), and `LevelRef::pop_resource(host, pos, stack)` drops
"as a broken block drops it". Stamped at this pin: `mod_api_generated.rs:8621`, and the part
fixture `parts/blocks_fixtures/destroyed_by_player_game_thread.rs`.

### What it does

- **The two tables, once.** `wire_drop(kind)` (copper/gold/electrum/festive → the kind's wire, and
  the festive wire a **copper** wire) and `source_drop(kind)` (each kind's own spool). `Keys::resolve`
  now takes the spool and wire items **from** these tables, so an id is written once and a wire and
  its drop cannot drift apart.
- **One stack per kind, four per wire** (`break_stacks`), filed in `WireType`'s order and dropped at
  the connector's own position — two copper wires are one stack of eight.
- **Creative drops nothing.** `player.is_creative(host) == Some(true)` takes the wires out with no
  items, as `dropWires(level, false)` does; an unknown answer is treated as survival (a wire that
  drops is recoverable, one that vanishes is not).
- **The peer lets go.** Every loaded peer whose list names the broken block is rewritten without it
  (`links_without`), keeping its other wires in their saved order and nodes.
- **The block itself is vanilla's.** The Part answers `None`, so the connector's own 26.3 loot table
  still drops one connector — which the leg asserts, so the "no wire" differential is not vacuous.

### Red → green (TDD)

The rules went in first as the pre-slice behaviour — no drop table (`wire_drop`/`source_drop` empty
strings, `break_stacks` empty, `links_without` a copy) — with five tests written against the jar's
own numbers. `drop-red.log`, verbatim:

```
---- network::tests::a_broken_connector_drops_four_wires_of_each_kind_it_held stdout ----
thread '...' panicked at src/network.rs:4293:5:
assertion `left == right` failed: four wire items per wire, one stack per kind, in WireType's own order
  left: []
 right: [(Copper, 8), (Gold, 4), (Festive, 4)]
---- network::tests::a_creative_break_takes_the_wires_out_and_drops_nothing stdout ----
  left: []
 right: [(Copper, 4), (Electrum, 4)]
---- network::tests::a_break_takes_the_broken_connector_out_of_its_peers_links stdout ----
  left: [SavedWire { pos: [10, 64, 0], node: 0, wire: Copper }, SavedWire { pos: [12, 64, 0], node: 1, wire: Gold }]
 right: [SavedWire { pos: [12, 64, 0], node: 1, wire: Gold }]
---- network::tests::the_festive_wire_drops_copper_wires_and_coils_back_onto_its_own_spool stdout ----
  left: ""
 right: "createaddition:copper_wire"
---- network::tests::an_unlinked_connector_drops_no_wire stdout ----
  left: []
 right: [(Copper, 4)]

test result: FAILED. 94 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out
```

Green: **99 passed, 0 failed** (`sdk-test.log`, regenerated for 0.2.10; 94 → 99: five new tests).
The five: one stack per kind in `WireType`'s order (two copper wires → 8), creative drops nothing
while the same break in survival drops them, both tables (including festive → copper wire and
festive → festive spool), the peer's link removal (order and node kept, another block's break
leaves the links alone), and the unlinked case paired with the wired one so it cannot pass
vacuously. `src/runtime.rs` stays byte-identical (`03721ca4…95d`, packed too).

### The 0.2.10 ladder

| Step | Command | Result |
|---|---|---|
| red | `cargo test --offline` against the pre-slice tables | **94 passed, 5 failed**, exit 101 (`drop-red.log`) |
| tests | the same | **99 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | clean (`sdk-fmt.log`, empty) |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | exit 0 (`drop-build-1…6.log`) |
| package, again | `rm -f Cargo.lock` + the same, twice more | **byte-identical** (`cmp`) |
| registry check | `cargo horizon-mod check createaddition.hmod` | exit 0, `0.2.10 … 190180 bytes` (`drop-check.log`) |
| fits, campaign pin | `cargo horizon-mod fits … --build 0a875da…` | **`is made for exactly one build (it has a [native.mixin] table), so fits doesn't apply`**, exit 2 (`drop-fits-pin.log`) |
| fits, another build | `… --build 0b14f8a…` | the same refusal, exit 2 (`drop-fits-other.log`) |
| launcher build | `launcher-game.sh build createaddition` (the 0.2.9 tag moved out of `versions/` first) | `Compiling createaddition (1 of 1)`, tag `native-0a875da-4af24797bccd` (`drop-launcher-build.log`; the README's own sentence below moved the content hash once, so the build was repeated) |
| launcher patch line | the launcher's own build log | `Function replacement: 2 functions patched by 2 mixins, without rebuilding the game (copy 114 ms, horizon-mixins 303 ms, compile 3021 ms, total 3440 ms)`, `"injections": 2` (`drop-launcher-patch.log`) |
| package verifier | `python3 ports/tools/verify-sdk-package.py createaddition` | **1/1** (`verify-sdk-package.log`) |
| recipe types | `python3 ports/tools/check-recipe-types.py createaddition` | `createaddition: ok` |

**The two `fits` lines are the route's refusals, not a regression** (unchanged since 0.2.9: a
package with a `[[native.mixin]]` row is made for exactly one game build). What replaces them as
compatibility evidence is the launcher build above: the recompile and the two-function patch both
ran against `0a875da`, the packaged manifest is `version = "0.2.10"`, the launcher's own identity for
the mod is `0a875da91dbb… createaddition 0.2.10 1.99.1` (`drop-launcher-identity.json`), and
`strings libcreateaddition.so` carries `createaddition:copper_wire` — the new code, in the binary
the game ran.

Deliverable: **`createaddition.hmod`, 190,180 B, SHA-256
`db4306b2ce83cbc58d0ce71919f87741285391ae1dd7f18dc39cdbce1c912f76`**, **127 entries**,
reproducible (repeated builds, `cmp`-identical), no packed `Cargo.lock`.

```
createaddition 0.2.10 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  190180 bytes, SHA-256 db4306b2ce83cbc58d0ce71919f87741285391ae1dd7f18dc39cdbce1c912f76, unsigned
```

### In-game (`e2e-drop-default.log`, `PASS` + `EXIT:0`)

A dedicated leg (`tools/e2e-drop.sh`), because the A–M run's break phase is about the *blocks'* loot
tables and never breaks a *linked* connector. It builds a pad in the End (the chunk loaded before the
`fill`), lays one wire per kind between two connectors four blocks apart, stands the player on the
floor 3.5 blocks off (inside the reach, outside the pickup radius) with an iron pickaxe (the
connector's own loot table needs it — a bare hand drops nothing, which would have made the
differential vacuous), breaks the far connector in survival and reads `ctl break`'s own reply:

```
== copper wire: lay it between 300 and 304, break the far connector in survival ==
  near end: Connector at 300,101,0: mode none, buffer 0/1000 FE, links [(304,101,0)/0/copper], pool 0/0, demand 0, t=133
  304: {"drops":{"createaddition:connector":1,"createaddition:copper_wire":4},"picked_up":{},"ticks":8}
  peer: Connector at 300,101,0: mode none, buffer 0/1000 FE, links [], pool 0/0, demand 0, t=190
== gold wire …  314: {"drops":{"createaddition:connector":1,"createaddition:gold_wire":4}, …}   peer links []
== electrum wire …  324: {"drops":{"createaddition:connector":1,"createaddition:electrum_wire":4}, …}   peer links []
== festive wire …  334: {"drops":{"createaddition:connector":1,"createaddition:copper_wire":4}, …}   peer links []
== differential: an unlinked connector drops no wire ==
  340: {"drops":{"createaddition:connector":1},"picked_up":{},"ticks":13}
  an unlinked connector's break: {"createaddition:connector":1} (0 wire stacks)
PASS: a broken connector drops four wires of each kind it held — the festive wire's four are copper
wires — the peer's links empty, and an unlinked connector drops none
```

Why these can fail: `drops` is the exact map of item entities within three blocks of the block
centre, so a package that drops nothing shows `{}` (the selftest) and a wrong kind or count shows a
different map (the leg prints the mismatch and exits 1); the festive rig is the jar's own trap — four
**copper** wires, not a festive anything; and the differential asserts the connector's own loot
still arrives, so "no wire" cannot come from a break that dropped nothing at all.

| log | verdict | what it proves |
|---|---|---|
| `e2e-drop-default.log` | `PASS` + `EXIT:0` | every kind's wire drops exactly four of its own drop at the broken block, the festive one's four are copper wires, the peer reads `links []`, and an unlinked connector drops no wire |
| `e2e-drop-selftest.log` | `FAIL` + `EXIT:1` | `DROP_SELFTEST=1`: the unlinked connector dropped 0 wires, so the leg's differential assertion fails — the counterfactual is alive |
| `e2e-drop-control.log` | `FAIL` + `EXIT:1` | a Nightly without the package does not know `createaddition:connector` (the setblock left `minecraft:air`), so the wires the modded run counts come from this package |
| `e2e-drop-suite.log` | `PASS` + `EXIT:0` | the whole A–M suite on 0.2.10: the new break Part disturbed no earlier family |
| `e2e-drop-suite-selftest.log` | `FAIL` + `EXIT:1` | the suite's conservation counterfactual is still alive |

`python3 ports/tools/check-e2e-logs.py createaddition-e2e --strict`:
**logs=40 ok=40 missing-exit=0 bad-exit=0** (the 35 earlier legs still green).

Harness facts this slice cost: (1) `ctl break`'s `drops` counts **item entities**, and a block whose
loot table needs a tool drops nothing to a bare hand — the first run's differential proved nothing
(`{"drops":{}}`) until the player held the pickaxe; (2) the trail-of-`info` reply pattern needs no
trailing space — `command_reply` appends one, and a pattern ending in a space matches two spaces and
never matches; (3) `launcher-game.sh env` exports `CONTROL_GAME`, so a run meant to be the *default*
leg must `unset CONTROL_GAME` or it silently runs the control leg; (4) the launcher reuses a build
whose content hash it already has, so a *new* version's first build is the one that prints
`Compiling …` and the patch line — the second run says "Up to date (no build needed)", and the
launcher's shared `native/build.log` is overwritten by any concurrent build, so it must be copied in
the same command.

### Coverage added by this slice

| family | status |
|---|---|
| a broken connector drops four wires of each kind it held | **tested** — `a_broken_connector_drops_four_wires_of_each_kind_it_held`; in game four rigs, `drops` exactly `{<kind>_wire: 4, connector: 1}` |
| the festive wire's drop is a **copper** wire, its source drop a festive spool | **tested** — `the_festive_wire_drops_copper_wires_and_coils_back_onto_its_own_spool`; in game the festive rig drops `createaddition:copper_wire` ×4 |
| a creative break takes the wires out and drops nothing | **tested** — `a_creative_break_takes_the_wires_out_and_drops_nothing` (the survival case asserted beside it) |
| the peer lets go of a broken connector without a second drop | **tested** — `a_break_takes_the_broken_connector_out_of_its_peers_links`; in game every peer reads `links []` |
| a connector with no wire drops no wire | **tested** — `an_unlinked_connector_drops_no_wire`, and the in-game differential (`{"createaddition:connector":1}`, 0 wire stacks) |
| the block's own loot still drops on the same break | **tested** in game (the `connector: 1` in every drop map) |
| a wire reclaimed with an empty spool (`getSourceDrop()` by hand) | **tested** since 0.2.1/0.2.8 (`remove_wires`/`spools_back`, the festive spool leg) |
| the break-drop of a connector removed by the *level* (support gone, piston, water) | **not ported** — the port has no `canSurvive` for its connectors, so the level never removes one; upstream's `neighborChanged` path is unreachable here |
| the sneak-wrench path (`dropWires(level, player, …)`: an empty spool in the pack turns a break into spools) | **not ported** — there is no wrench item in this port (Create's `IWrenchable` is the parent's), so its route has no trigger; the same table is what the empty-spool click uses |
| a remote client's view of a break | **not ported** — no block-entity sync (the port's standing deviation 5) |

### Deviations of this slice (declared)

1. **The drop is one item stack per wire kind, not per wire.** Upstream aggregates by `WireType`
   index too, so two copper wires are one stack of eight at the same position; the in-game assertion
   sums the entities by item id, which is what a player sees.
2. **Dropped through `LevelRef::pop_resource`**, the SDK's "as a broken block drops it" (an item
   entity at a random point within a quarter block of the block centre, pickup delay 10 ticks).
   Upstream's `Containers.dropContents` uses the same shape; the random offsets differ.
3. **A peer that is unloaded or refuses the write keeps its edge** — upstream lets it go when it next
   loads, and this port's network walk already ignores a link its peer does not name back, so the
   stale edge is inert. The in-game leg's peer is loaded, so this is a boundary, not the tested path.
4. **`will_harvest` is not used.** The creative test is `PlayerRef::is_creative`, as upstream's
   `player.isCreative()` is; `will_harvest` is false for creative *and* for a tool the block refuses,
   which would conflate the two.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

---

## Slice 12 — the lifecycle clears (0.2.11)

### The review finding this slice answers

`NikitaMGrimm` reviewed the staged 0.2.6 and 0.2.7 archives and held them:

> Both match the previously reviewed archives that **retain stale comparator entries after
> accumulator removal, chunk unload and level close**. … make sure the replacement **clears the
> affected entries on each of those lifecycle events**.

It was true, and this port had declared it instead of fixing it: § *Slice 6*'s deviation 2 said "the
map is keyed by dimension id and position, and it is **not pruned on break**. One entry per placed
accumulator; a broken block's answer stays until the process ends." The register was empty on the
other side too — the package subscribed to `register`, `level_tick_post`, `register_commands`,
`run_mod_command` and the two render mixins, so **nothing** cleared a position-keyed entry.

### The lifecycle events, measured against the pin's own declarations

Read from `ports/_shared/sdk/0a875da/` (`surface.json` ids `hook:*` plus each
`crates/horizon-mod-vocab/src/hooks/<name>.rs` doc) — not guessed:

| event | hook | what it is told | why this one |
|---|---|---|---|
| an accumulator is removed or replaced | **`BlockPart::on_block_state_change`** (`IBlockExtension.onBlockStateChange`) | `dimension: LevelDimension`, `pos`, `state`, `old`, `new`, `predicted` | scoped to **our state's** parts, so no other block's writes reach it; heard at the end of every `Level.setBlock` the server makes (a break, `setblock air`, a piston, an explosion), with the dimension in the event |
| a chunk unloads | **`Hooks::chunk_unload`** (`ChunkEvent.Unload`) | `dimension: &str`, `chunk: [i32; 2]` | the event names the dimension and the chunk column; the map is keyed by dimension and position, so the clear is one `retain` per unload and needs no block read |
| a level closes | **`Hooks::level_unload`** (`LevelEvent.Unload`) | `dimension: &str` | "posted after `level.close()` in `MinecraftServer.stopServer`, after its last save: **drop what the mod kept for it**"; once per `level_load`, and for every level still open at a server stop — which is also the session's clear, because `chunk_unload` is **not** told at a world close |
| a player leaves | **`Hooks::player_logged_out`** | `player: u128` | the spool selection is per player, so it goes with them |

**Two candidates measured and rejected, with the reason:**

- **`on_chunk_unloaded`** (`IBlockEntityExtension.onChunkUnloaded`, a `BlockEntityPart` method) is the
  per-block-entity notice and would be exact for our entities — but its event carries **no dimension**
  (`pos`, `block_entity`, `entity` only), and its own doc says a server's chunk "may have left the
  level already, so **don't count on the handle's reads**". Deriving the dimension from the handle, or
  from `host.dimension()` (which is "the level in place", not the level of the chunk being dropped),
  would be a guess. `chunk_unload` states the dimension and the chunk, so it is the hook that fits.
- **`world_saved` / `level_save`** are saves, not unloads. Clearing on a save would blank a live
  comparator answer until the next pass (a wrong answer for a comparator that reads in between), so
  they are **not** the clearing events — and the in-game leg asserts that a `/save-all` keeps the
  answers (N5).

### What it does

- **`Signals` (`src/network.rs`) gains three clears and a count.** `forget(dimension, pos)` (block
  removed), `forget_chunk(dimension, chunk)` (`retain` over the column: `pos[0].div_euclid(16)` and
  `pos[2].div_euclid(16)`, so `x=-1` is chunk `-1`, not `0`), `forget_dimension(dimension)` (the whole
  map of one level) and `counts()` (total plus per-dimension, name-ordered). An emptied dimension's
  key is removed, so the map holds exactly the answers of blocks that exist.
- **The removal hook is the accumulator's own Part.** `AccumulatorShape::on_block_state_change`
  forgets the answer when `old` is our block and `new` is not, on the **server** side only (a client's
  copy of a write is ignored: the answers are the server pass's).
- **`Queue` follows its level out.** `forget_dimension` drops the positions queued for a closing
  level; without it a level that closes with positions pending keeps them (and its `LevelRef`) for
  the rest of the session.
- **`Pending` (the spool's click in progress) is dropped** by `level_unload` (`clear`) and by
  `player_logged_out` (`forget`): a half-made link cannot outlive the level it was made in or the
  player who made it.
- **`/createaddition signals`** is the observable: `Signals: answers=<n> dimensions=<dim>:<n>,… pending=<n>`.
  It exists because a position-keyed entry whose block is gone has **no other** in-game read: a
  comparator beside the air reads 0 whether the entry is stale or not, and the evicted chunk is not
  loaded, so the mod's own count is the only thing that can show the entry went.
- **No new API.** `chunk_unload`, `level_unload`, `player_logged_out` and
  `on_block_state_change` are all at the pin (`mod_api_generated.rs`), and the package's
  `[[native.mixin]]` render rows are unchanged.

### Red → green (TDD)

The rules went in first as the pre-slice behaviour — `forget`/`forget_chunk`/`forget_dimension`
returning "nothing", `Pending::forget`/`clear` doing nothing, `counts()` empty — with six tests
written against the events. `lifecycle-red.log`, verbatim:

```
thread 'network::tests::a_removed_accumulators_answer_is_forgotten_and_its_neighbours_are_not' (242841) panicked at src/network.rs:4391:5:
the removed block's answer was there to forget
thread 'network::tests::a_chunk_unload_forgets_that_chunks_answers_and_keeps_the_others' (242816) panicked at src/network.rs:4430:5:
assertion `left == right` failed: both of the unloaded chunk's answers go
  left: 0
 right: 2
thread 'network::tests::a_chunk_unload_reads_negative_coordinates_as_their_own_chunk' (242817) panicked at src/network.rs:4462:5:
assertion `left == right` failed
  left: 0
 right: 1
thread 'network::tests::a_level_unload_forgets_every_answer_of_that_dimension_only' (242835) panicked at src/network.rs:4481:5:
assertion `left == right` failed: the closed level's answers go
  left: 0
 right: 2
thread 'network::tests::the_selections_in_progress_are_dropped_when_their_level_closes_and_when_a_player_leaves' (242873) panicked at src/network.rs:4509:5:
assertion `left == right` failed: a player who left keeps no selection in the mod
  left: 2
 right: 1
thread 'network::tests::the_lifecycle_readout_counts_the_answers_it_holds' (242869) panicked at src/network.rs:4534:5:
assertion `left == right` failed: per dimension, in name order, so the readout is deterministic
  left: (0, [])
 right: (3, [("minecraft:overworld", 1), ("minecraft:the_end", 2)])

test result: FAILED. 99 passed; 6 failed; 0 ignored; 0 measured; 0 filtered out
```

Green: **105 passed, 0 failed** (`lifecycle-test.log`; 99 → 105, the six new tests). The six: the
removal (its own entry gone, the neighbour's and another dimension's kept), the chunk unload (both
answers of the column gone, the block 40 y above it gone, the next chunk's and another dimension's
kept), the negative-coordinate column (`x=-1` is chunk `-1`), the level unload (that dimension's
gone, the other's kept, no empty map left), the selections (a logout forgets one player's, a level
close drops every one), and the readout. `src/runtime.rs` is untouched (`03721ca4…f962`, packed
too).

### The 0.2.11 ladder

| Step | Command | Result |
|---|---|---|
| red | `cargo test --offline` against the pre-slice clears | **99 passed, 6 failed**, exit 101 (`lifecycle-red.log`) |
| tests | the same | **105 passed**, 0 failed (`sdk-test.log`, regenerated for 0.2.11) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | clean (`sdk-fmt.log`; `src/runtime.rs` unchanged, `03721ca4…f962`) |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | exit 0 (`lifecycle-build-1.log`) |
| package, second build | `rm -f Cargo.lock` + the same again | **byte-identical** (`cmp`) (`lifecycle-build-2.log`) |
| registry check | `cargo horizon-mod check createaddition.hmod` | exit 0, `0.2.11 … 194450 bytes` (`lifecycle-check.log`) |
| fits, campaign pin | `cargo horizon-mod fits … --build 0a875da…` | **`is made for exactly one build (it has a [native.mixin] table), so fits doesn't apply`**, exit 2 (`lifecycle-fits-pin.log`) |
| fits, another build | `… --build 0b14f8a…` | the same refusal, exit 2 (`lifecycle-fits-other.log`) |
| launcher build | `launcher-game.sh build createaddition` | `Compiling createaddition (1 of 1)`, `native-0a875da-05f84c083d3a` (`lifecycle-launcher-build.log`), and the launcher's own log: `Compiling createaddition: 8388 ms`, `Function replacement: 2 functions patched by 2 mixins, without rebuilding the game` (`lifecycle-launcher-patch.log`) |
| launcher identity | `native-mods.json` of the built version | `{"id":"createaddition","version":"0.2.11",…}`, `libcreateaddition.so` identity `0a875da91dbb… createaddition 0.2.11 1.99.1`; `strings libcreateaddition.so` carries `Signals: answers` (the new code, in the binary the game ran) |
| data shape | `python3 tools/data_shape.py createaddition.hmod` | 6/6 files in the 26.3 dialect |
| content carry | `python3 tools/carry_content.py --check` | `assets/ and data/ match the jar carry` |
| licence notice | `python3 tools/check_licence.py` | `ok … carries the upstream MIT notice verbatim` |
| recipe types | `python3 ports/tools/check-recipe-types.py createaddition` | `createaddition: ok` |
| package verifier | `python3 ports/tools/verify-sdk-package.py createaddition` | **1/1** — including `package matches the tree` (`verify-sdk-package.log`) |
| e2e checker | `python3 ports/tools/check-e2e-logs.py createaddition-e2e --strict` | **logs=47 ok=47 missing-exit=0 bad-exit=0** |

**Both `fits` lines are the route's refusals, not a regression** (unchanged since 0.2.9: a package
with a `[[native.mixin]]` row is made for exactly one build). The compatibility evidence is the
launcher build and the game it produced, which ran the lifecycle leg, the whole A–M suite and the
renderer leg.

Deliverable: **`createaddition.hmod`, 194,450 B, SHA-256
`0b1b8184af405689c80154aacdc718949f701cc6466d473106e50a2c0705f6fc`**, **127 entries**,
reproducible (two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f962`, packed too). The content is untouched (`assets/`/`data/` the same 118 files), so
the entry count is 0.2.10's.

```
createaddition 0.2.11 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  194450 bytes, SHA-256 0b1b8184af405689c80154aacdc718949f701cc6466d473106e50a2c0705f6fc, unsigned
```

```
Create Crafts & Additions is made for exactly one build (it has a [native.mixin] table), so `fits` doesn't apply.
```

### In-game (`e2e-lifecycle-default.log`, `PASS` + `EXIT:0`)

A dedicated leg (`tools/e2e-lifecycle.sh`, after 0.2.10's `e2e-drop.sh`): no earlier leg can read a
position-keyed cache, because the block it answered for is gone and the chunk it answered in is not
loaded. The leg builds two rigs — an accumulator with a comparator on its `+z` side (facing north,
reading it) and a wire behind that comparator — one at `300,101,0` and one at `1100,101,0`, 800
blocks apart, the far one **force-loaded** (`/forceload add`) so both answers exist at once while
only the near one can be evicted by a teleport. The assertion is `/createaddition signals`'s count
plus the **survivor's** wire power:

```
== N: an accumulator's comparator answer is held while it is there ==
  before anything is placed: Signals: answers=0 dimensions=none pending=0
  the first accumulator's own comparator: redstone_wire at 300,101,2 power=8
  two accumulators: Signals: answers=2 dimensions=minecraft:the_end:2 pending=0
== N1: a removed accumulator's answer is forgotten, its neighbour's is not ==
  after the removal: Signals: answers=1 dimensions=minecraft:the_end:1 pending=0
  the removed accumulator's comparator (it now reads air): redstone_wire at 300,101,2 power=0
== N2: a block placed where one was broken answers from a pass, not from the old entry ==
  the re-placed accumulator: redstone_wire at 300,101,2 power=8
  re-placed: Signals: answers=2 dimensions=minecraft:the_end:2 pending=0
== N3: a chunk unload forgets that chunk's answers and keeps the loaded one's ==
  after the unload: Signals: answers=1 dimensions=minecraft:the_end:1 pending=0
  the accumulator whose chunk is still loaded: redstone_wire at 1100,101,2 power=8
== N4: back on the pad, the pass records the answer again ==
  back: Signals: answers=2 dimensions=minecraft:the_end:2 pending=0
== N5: a save is not an unload — /save-all keeps the live answers ==
  after save-all: Signals: answers=2 dimensions=minecraft:the_end:2 pending=0
  the comparator after a save: redstone_wire at 300,101,2 power=8
== N6: a relaunch carries no answer the new session did not compute ==
  the survivor after the relaunch: redstone_wire at 1100,101,2 power=8
  after the relaunch, at the survivor: Signals: answers=1 dimensions=minecraft:the_end:1 pending=0
  both chunks loaded in the new session: Signals: answers=2 dimensions=minecraft:the_end:2 pending=0
PASS: …
EXIT:0
```

Why these can fail: the count is the mod's own state, so 2 → 1 can only come from the event being
delivered and the entry removed — a package that keeps the entry (the reviewed archives) reads 2 at
N1 and N3, and **N6 reads 2 at the survivor** (an entry carried across the session); the survivor's
wire is read from the **world** (`ctl block`'s `state`, `power=8`), so a clear that took everything
out fails there; N4's 2 after the reload is the pass recording again, so the clear cannot have been
"the map is dead"; N5 is the negative for `world_saved`/`level_save` (a save must not clear); and
`LIFECYCLE_SELFTEST=1` (`e2e-lifecycle-selftest.log`) inverts N3's assertion — the chunk unload must
have left the answer — and exits 1 (`FAIL: LIFECYCLE_SELFTEST: the chunk unload forgot the answer
(answers=1), so the eviction assertion is alive`).

| log | verdict | what it proves |
|---|---|---|
| `e2e-lifecycle-default.log` | `PASS` + `EXIT:0` | 0 → 2 answers for two accumulators; a removal takes the broken one's (2 → 1) while its neighbour's wire still reads 8; a re-placed block is recorded again (1 → 2); a chunk unload takes that chunk's (2 → 1) while the still-loaded accumulator's wire reads 8; going back re-records it (1 → 2); `/save-all` keeps both; a relaunch holds only the answers the new session computed (1 at the survivor, 2 with both chunks loaded) |
| `e2e-lifecycle-selftest.log` | `FAIL` + `EXIT:1` | N3's own counterfactual (`LIFECYCLE_SELFTEST`: the unloaded chunk's answer must still be there) |
| `e2e-lifecycle-control.log` | `FAIL` + `EXIT:1` | a Nightly without the package cannot place `createaddition:modular_accumulator`, so the counts this leg reads come from this package |
| `e2e-lifecycle-suite.log` | `PASS` + `EXIT:0` | the whole A–M suite on 0.2.11: every earlier family unchanged (the relay's 25,020 FE, the lamp's 1 FE/t, the comparator's 0/8/15, the generator's 1,000 FE/t, the festive wire, the break drops) |
| `e2e-lifecycle-suite-selftest.log` | `FAIL` + `EXIT:1` | the suite's conservation counterfactual is still alive |
| `e2e-lifecycle-wire.log` | `PASS` + `EXIT:0` | the renderer is not regressed: **1901** green-dominant pixels with the wire, mean `rgb(21, 86, 9)`, 0 without it |
| `e2e-lifecycle-wire-negative.log` | `FAIL` + `EXIT:1` | the renderer's own counterfactual (a renderer that draws nothing fails the leg) |
| `probe-lifecycle.log` | `EXIT:0` | measured that `/forceload add 1100 0` marks chunk `[68, 0]`, that the far accumulator is placed and **counted** (`answers=1`) 800 blocks from the player, and that `ctl block` answers `loaded:false` for a chunk only the **client** has not loaded (why the leg places rigs from anywhere and reads them only while standing beside them) |
| `probe-lifecycle3.log` | `EXIT:0` | the relaunch: the same world comes back with both accumulators and their saved `energy: 1000000`, and the second session records both (2 answers) — the first N6 attempt failed because its `pad` re-fill erased the far rig, not because of the mod |

### Coverage added by this slice

| family | status |
|---|---|
| an accumulator's answer is forgotten when the block is removed or replaced | **tested** — `a_removed_accumulators_answer_is_forgotten_and_its_neighbours_are_not`, the in-game removal (N1) and its re-place (N2) |
| …when its chunk unloads (the whole column, other chunks kept) | **tested** — `a_chunk_unload_forgets_that_chunks_answers_and_keeps_the_others`, `a_chunk_unload_reads_negative_coordinates_as_their_own_chunk`; in game N3 |
| …when its level closes, and the session carries nothing | **tested** in the rules — `a_level_unload_forgets_every_answer_of_that_dimension_only`; in game N6 (a relaunch holds only what it computed) — the level unload itself is **not separately observable** (see deviation 3) |
| a save is not an unload | **tested** in game — N5 (`/save-all` keeps 2 answers and the wire at 8) |
| the tick queue does not keep a closed level | **tested** by the same `level_unload` path (`Queue::forget_dimension`); no unit test, because `LevelRef::dimension_id` needs the game's registry table (deviation 4) |
| a spool selection is dropped by a level close and by a logout | **tested** — `the_selections_in_progress_are_dropped_when_their_level_closes_and_when_a_player_leaves`; the readout's `pending=` stays 0 in every in-game phase |
| the readout itself (`/createaddition signals`) | **tested** — `the_lifecycle_readout_counts_the_answers_it_holds`; every in-game phase parses it |
| `world_saved` as a clearing event | **measured and rejected** — a save is not an unload (N5 asserts it does not clear) |

### Deviations of this slice (declared)

1. **`chunk_unload`, not `on_chunk_unloaded`.** The per-block-entity notice would be exact for our
   entities, but it carries no dimension (`pos`, `block_entity`, `entity`) and its own doc says a
   server's chunk "may have left the level already, so don't count on the handle's reads". Deriving
   the dimension from the handle or from `host.dimension()` (the level in place) would be a guess;
   `chunk_unload` names the dimension and the chunk, so the clear is a `retain` with no read.
2. **The removal clear is `on_block_state_change`, scoped to our block.** It fires for every
   `Level.setBlock` on the server that changes our accumulator's state (a break, `setblock air`, a
   piston, an explosion) — but not for a block removed without a state write, which is the chunk
   unload case and is covered there.
3. **A level close is unit-tested, not separately observable in game.** Every chunk of a closing
   level unloads first (and `chunk_unload` clears those answers), and a world close ends the process,
   so the map's contents cannot distinguish `level_unload` from the chunk clears that preceded it.
   What the leg *can* show is the session's own half — a relaunch carries no answer (N6) — and the
   rules test the level clear directly. The one thing the unit test cannot check is that the hook's
   dimension string is the pass's key; N3 shows that for `chunk_unload` (the count fell when the
   chunk left), and `level_unload`'s `dimension` comes from the same dimension-name table, so the
   two hooks cannot name a level differently from each other.
4. **`Queue::forget_dimension` has no unit test.** It compares `LevelRef::dimension_id()`, which
   resolves through the game's registry table and panics without one, so a `cargo test` cannot build
   the handle. It runs in the same `level_unload` call as the answers' clear, which is tested.
5. **A level close drops every player's spool selection, not only one level's.** The selection is
   keyed by player and does not carry its level (it is a click in progress, seconds long), so
   `level_unload` clears the store rather than trying to attribute it. A logout drops its own.
6. **The readout is a command, and that is deliberate.** No in-game read can show a position-keyed
   entry whose block is gone (the comparator beside the air reads 0 either way, and the evicted chunk
   is not loaded), so `/createaddition signals` reports the count. It is a level-0 player control
   (read-only) and its reply is tagged like every other.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-review

---

## Slice 13 — the support rule and the level's own removal drop (0.2.12)

### The slice, and the gap line it comes from

0.2.10's coverage table carried the family as missing:

> **the break-drop of a connector removed by the level (support gone, piston, water)** | **not
> ported** — the port has no `canSurvive` for its connectors, so the level never removes one;
> upstream's `neighborChanged` path is unreachable here

**Choice.** A connector whose support went away was an orphan: it kept floating, kept its wire in
its peer's list, and the level never took it out. Upstream removes it in `neighborChanged` —
`playerWillDestroy`'s removal is the *player's* path (0.2.10), this is the **level's**. It is the
last server half of the wire family and it reuses both the drop tables 0.2.10 added.

**Tier fit.** `deep`: one support rule on the three wire Parts plus the pass that applies it, and
the two existing drop tables (`break_stacks`/`wire_drop`). No renderer, no entity, no new block,
no new content.

### The rule, read from the artefact before implementing (not guessed)

Repo `658a390b8ff4daa17013e9447b29949330c8a42a` (analysis only; the port is a clean-room rewrite):

| file | what it says |
|---|---|
| `blocks/connector/base/AbstractConnectorBlock.java:149-163` | `neighborChanged`: `if (!state.canSurvive(worldIn, pos)) { dropResources(state, worldIn, pos, blockEntity); if (blockEntity instanceof IWireNode) ((IWireNode) blockEntity).dropWires(worldIn, true); worldIn.removeBlock(pos, false); updateNeighborsAt ×6; }` — the level's removal drops the block's loot **and** the wires, even for a creative player (`dropWires(level, true)`) |
| `…AbstractConnectorBlock.java:156-163` | `canSurvive`: the support is `pos.relative(facing)` and the face asked is `facing.getOpposite()` (`isFaceSturdy(..., SupportType.CENTER)`), **or** `CommonConfig.CONNECTOR_IGNORE_FACE_CHECK` |
| `config/CommonConfig.java:197-198` | that flag's default is **`true`** — upstream's connectors may float by default; the *relay* has no such flag |
| `blocks/redstone_relay/RedstoneRelayBlock.java:122-145` | the relay's `neighborChanged` removes the same way (`dropResources` + `removeBlock`), and its `canSurvive` is `canSupportCenter(vertical ? pos.relative(direction) : pos.below(), vertical ? direction.getOpposite() : Direction.UP)` — a flat relay stands on the block **below**, a vertical one behind its `facing` |

So the rule is: **a wire block whose support face is not sturdy is removed by the level, with its
own loot and four wires per wire it held.** The support is the face behind a connector's (or a
vertical relay's) `facing`, or the block below a flat relay.

**The default flag, handled, not ignored.** A connector placed with nothing behind it is
upstream's default (`connector_ignore_face_check = true`) and this port keeps that half: a floated
connector is not judged an orphan. What is fixed is the orphan — a block that **had** a support
and lost it. The trigger is the neighbour change (`neighborChanged`), so a block that never had a
support is never the subject of a removal it did not earn; the existing A–M rigs (which float
connectors and a relay in a few places) stay exactly as they were.

### The route at the pin

`BlockPart::can_survive` (`IBlockExtension.canSurvive`, contract *replace*) **cannot** carry this
rule, and neither can `update_shape`, though both exist:

- `can_survive` is asked of a block **whose template checks survival** (a plant, a torch, a sign)
  or at placement (`BlockItem.canPlace`); the engine never asks it for a block whose template is
  `iron_block` (our connector's) on a shape update, so a `Some(false)` there would never run.
- Both hooks receive a **level reader** (`&dyn LevelHost` / `&mut dyn LevelTicksHost`), which has
  no face-sturdy query; `LevelRef::block_face_sturdy` needs the game host. The rule would be a
  guess without it.

The exact route, the one `ARCHITECTURE`/`hook-feasibility.md` prescribe for a neighbour-triggered
write, is:

| point | hook | what it gives |
|---|---|---|
| the trigger | **`BlockPart::neighbor_changed`** (`Block.neighborChanged`) | a notice for our own states only, with `dimension`/`pos`; no writes, so it queues the position |
| the decision | **`level_tick_post`** (this port's one pass) | the game host and a `LevelRef`: `LevelRef::block_property` (the `facing`/`vertical` bits) and `LevelRef::block_face_sturdy(pos, face, SupportType::Center)` — upstream's own query |
| the drop | `LevelRef::destroy_block(host, pos, true, None, false)` | `Level.destroyBlock`: the block's own loot table, "the one level drop path … loot, block-drop hooks and experience once" |
| the wires | the 0.2.10 tables | `break_wires(…, false)` drops four per wire and empties the peer without a second drop |

`SupportType::Center` is the constant upstream passes, not `Full`: a connector mounts on a slab
as a torch does. An answer the level will not give (an unloaded support) is taken as **supported**
(a block that might still be supported is not an orphan).

### What it does

- **`support_of(kind, facing, vertical)`** (`src/network.rs`, pure and unit-tested): the two rules
  above, as the `(direction to the support, face asked on it)` pair. Connectors, the large
  connector and the lamp use the `facing` face; a flat relay uses the block below; a vertical relay
  uses its `facing`. It is the first rule of this campaign's support checks that is *not* per-block
  duplicated: one function, every wire kind.
- **`Supports`** (the trigger queue): positions a neighbour change reached, per dimension, drained
  in that level's pass. It is emptied every tick it is drained, so a position cannot outlive its
  level; `level_unload` forgets a closing level's (the same discipline the lifecycle slice's clears
  established).
- **`neighbor_changed` on the three wire Parts** queues the position. It runs for our blocks only,
  so no other block's neighbour changes reach it.
- **The pass removes before it moves energy.** `level_ticked` drains the queue first, and a block
  whose support is gone is taken out *before* the network walk reads its links; the removed
  positions are filtered out of the pass's block list, so a destroyed connector is never a stale
  member.
- **The drop is the player's drop.** `break_wires(…, false)` (always drop: upstream's level path
  passes `true`, not `!isCreative`) plus `destroy_block`'s loot. `drops` in game is exactly the
  connector and the four wires.
- **No new hook, no new content.** `neighbor_changed`, `block_property`, `block_face_sturdy` and
  `destroy_block` are all at the pin; the `[[native.mixin]]` render rows are unchanged.

### Red → green (TDD)

The rules went in first as the pre-slice behaviour — `support_of` answering the flat-relay rule for
every block — with three tests written against the jar's own geometry. `support-red.log`, verbatim:

```
---- network::tests::a_connector_is_mounted_on_the_face_behind_its_facing stdout ----
thread 'network::tests::a_connector_is_mounted_on_the_face_behind_its_facing' (500860) panicked at src/network.rs:4891:7:
assertion `left == right` failed: Small on a north-facing connection stands on the block north of it
  left: (Down, Up)
 right: (North, South)
---- network::tests::a_flat_relay_stands_on_the_block_below_and_a_vertical_one_behind_its_facing stdout ----
thread 'network::tests::a_flat_relay_stands_on_the_block_below_and_a_vertical_one_behind_its_facing' (500867) panicked at src/network.rs:4924:5:
assertion `left == right` failed: a vertical relay hangs on the block behind its facing
  left: (Down, Up)
 right: (West, East)

test result: FAILED. 106 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

Green: **108 passed, 0 failed** (`sdk-test.log`, regenerated for 0.2.12; 105 → 108, the three new
tests). The three: a connector's (and large connector's, and lamp's) support is the face behind its
`facing` for three directions, a flat relay's is the block below whatever way it lies and a
vertical relay's is the block behind its `facing` (a ceiling included), and only the four wire
kinds are support-checked while the accumulator and the generator are not. `src/runtime.rs` stays
byte-identical (`03721ca4…f962`, packed too).

### The 0.2.12 ladder

| Step | Command | Result |
|---|---|---|
| red | `cargo test --offline` against the pre-slice rule | **106 passed, 2 failed**, exit 101 (`support-red.log`) |
| tests | the same | **108 passed**, 0 failed (`sdk-test.log`, regenerated) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | clean (`sdk-fmt.log`, empty) |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | exit 0 (`support-build-1.log`) |
| package, second build | `rm -f Cargo.lock` + the same again | **byte-identical** (`cmp`) (`support-build-2.log`) |
| registry check | `cargo horizon-mod check createaddition.hmod` | exit 0, `0.2.12 … 197327 bytes` (`support-check.log`) |
| fits, campaign pin | `cargo horizon-mod fits … --build 0a875da…` | **`is made for exactly one build (it has a [native.mixin] table), so fits doesn't apply`**, exit 2 (`support-fits-pin.log`) |
| fits, another build | `… --build 0b14f8a…` | the same refusal, exit 2 (`support-fits-other.log`) |
| launcher build | `launcher-game.sh build createaddition` | `Compiling createaddition (1 of 1)`, tag `native-0a875da-e88f42afab54` (`support-launcher-build.log`), and the launcher's own log: `Compiling createaddition: 4128 ms`, `Function replacement: 2 functions patched by 2 mixins, without rebuilding the game (copy 70 ms, horizon-mixins 102 ms, compile 1187 ms, total 1360 ms)` (`support-launcher-patch.log`); the README's version line moved the content hash once, so the launcher was run again from the final tree and its `libcreateaddition.so` is `44f14914…` — the same binary the in-game legs below ran, so a docs-only rebuild did not change the compiled code |
| launcher identity | `native-mods.json` of the built version | `{"id":"createaddition","version":"0.2.12",…}`, `libcreateaddition.so` identity `0a875da91dbb… createaddition 0.2.12 1.99.1`; `strings libcreateaddition.so` carries the new code (5 hits for `support_gone`/`createaddition:copper_wire`) |
| data shape | `python3 tools/data_shape.py createaddition.hmod` | 6/6 files in the 26.3 dialect |
| content carry | `python3 tools/carry_content.py --check` | `assets/ and data/ match the jar carry` |
| licence notice | `python3 tools/check_licence.py` | `ok … carries the upstream MIT notice verbatim` |
| recipe types | `python3 ports/tools/check-recipe-types.py createaddition` | `createaddition: ok` |
| package verifier | `python3 ports/tools/verify-sdk-package.py createaddition` | **1/1** — including `package matches the tree` (`verify-sdk-package.log`) |
| e2e checker | `python3 ports/tools/check-e2e-logs.py createaddition-e2e --strict` | **logs=56 ok=56 missing-exit=0 bad-exit=0** |

**Both `fits` lines are the route's refusals, not a regression** (unchanged since 0.2.9: a package
with a `[[native.mixin]]` row is made for exactly one build). The compatibility evidence is the
launcher build and the game it produced, which ran the support leg, the whole A–M suite, the drop
leg, the lifecycle leg and the renderer leg.

Deliverable: **`createaddition.hmod`, 197,327 B, SHA-256
`63062d5fdcd07e7a573b3910727be91444332f42b9bd1b9e05daef0b25d82e4e`**, **127 entries**,
reproducible (two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f962`, packed too). The content is untouched (`assets/`/`data/` the same 114 files), so
the entry count is 0.2.11's.

```
createaddition 0.2.12 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  197327 bytes, SHA-256 63062d5fdcd07e7a573b3910727be91444332f42b9bd1b9e05daef0b25d82e4e, unsigned
```

```
Create Crafts & Additions is made for exactly one build (it has a [native.mixin] table), so `fits` doesn't apply.
```

### In-game (`e2e-support-default.log`, `PASS` + `EXIT:0`)

A dedicated leg (`tools/e2e-support.sh`, after 0.2.10's `e2e-drop.sh`): no earlier leg breaks a
*connector's support*. The rig is a pair of connectors four blocks apart, each mounted
`facing=down` on a **glass** block one below it, with one copper wire between them. Glass is a full
block (it holds the connectors) and drops nothing, so the break's own `drops` map carries our items
only. The leg breaks the far connector's glass in survival and reads `ctl break`'s reply:

```
== an orphan: the far connector stands on glass at 304,101, break that glass ==
  peer at 300: Connector at 300,102,0: mode none, buffer 0/1000 FE, links [(304,102,0)/0/copper], pool 0/0, demand 0, t=133
  the wire is saved on the peer
  304,101: {"drops":{"createaddition:connector":1,"createaddition:copper_wire":4},"picked_up":{},"ticks":9}
  peer at 300: Connector at 300,102,0: mode none, buffer 0/1000 FE, links [], pool 0/0, demand 0, t=201
== differential: the same rig at 316, its support stays ==
  peer at 312: Connector at 312,102,0: mode none, buffer 0/1000 FE, links [(316,102,0)/0/copper], pool 0/0, demand 0, t=271
  the kept rig still carries its wire
PASS: a connector whose support is broken is removed by the level and drops the connector and four wires per wire it held, with its peer's link emptied; an identically placed connector on a support that stays keeps both
```

Why these can fail: `drops` is the exact map of item entities within three blocks of the broken
glass, so a package that removes nothing leaves the connector in the world (and the `GONE` check
fails), a package that removes it without the wires shows `{"createaddition:connector":1}`, and a
wrong kind shows a different map; the peer readout is the block entity's own saved links, so a
removal that forgets to clear the reciprocal node reads the edge. The **differential** is the
mirror: the same rig whose glass stays, poked by a placed-and-removed stone above the far
connector so its `neighbor_changed` really runs, is still there with its link — so "gone" cannot
come from the support check removing everything. `SUPPORT_SELFTEST=1` inverts the removal (it
asserts the orphan is still there), finds it gone, and exits 1.

| log | verdict | what it proves |
|---|---|---|
| `e2e-support-default.log` | `PASS` + `EXIT:0` | the orphan pair starts linked; the far connector's glass is broken; the far connector is gone, `drops` is exactly `{connector:1, copper_wire:4}`, the peer reads `links []`; the identical rig whose support stays keeps its connector and its link after the same poke |
| `e2e-support-selftest.log` | `FAIL` + `EXIT:1` | `SUPPORT_SELFTEST: the orphan reads 'minecraft:air' — the level removed it, so the default leg's removal assertion is alive` |
| `e2e-support-control.log` | `FAIL` + `EXIT:1` | a Nightly without the package does not know `createaddition:connector` (the setblock left `minecraft:air`), so the removal this leg reads comes from this package |
| `e2e-support-suite.log` | `PASS` + `EXIT:0` | the whole A–M suite on 0.2.12: every earlier family unchanged (the relay's conserved bridge, the lamp's 1 FE/t, the comparator's 0/8/15, the generator's 1,000 FE/t, the festive wire's 1,000 FE/t, the break drops, the doctored tower) |
| `e2e-support-suite-selftest.log` | `FAIL` + `EXIT:1` | the suite's conservation counterfactual is still alive |
| `e2e-support-wire.log` | `PASS` + `EXIT:0` | the renderer is not regressed: **1901** green-dominant pixels with the wire (mean `rgb(21, 86, 9)`), 0 without it |
| `e2e-support-wire-negative.log` | `FAIL` + `EXIT:1` | the renderer's own counterfactual (a renderer that draws nothing fails the leg) |
| `e2e-drop-default-0.2.12.log` | `PASS` + `EXIT:0` | 0.2.10's break-drop leg still green on this package: the floated rigs survive the new trigger (a floating connector is the default's, not an orphan), and a broken connector still drops its wires |
| `e2e-lifecycle-default-0.2.12.log` | `PASS` + `EXIT:0` | the pass's new first step disturbs none of the lifecycle clears: a removal, a chunk unload, a save and a relaunch read exactly as 0.2.11 |

Harness facts this slice cost: (1) the support queue drains in the same tick it is filled, so no
extra state survives a pass; (2) the rig must place the **support before** the connector, because
a connector placed in air is the default's floating connector and is deliberately not removed; (3)
glass is the clean support for this assertion — a full block (so `SupportType::Center` holds it)
whose break drops nothing, keeping the `drops` map to our items; (4) the A–M suite's phase I
relay sat *in* the floor row (`y=100`) with air below it, which upstream's flat-relay rule would
reject, so the rig now places the one stone its support asks for (`y=99`) — no other leg needed a
change.

### Coverage (this slice's families)

| family | status |
|---|---|
| a connector whose support is broken is removed by the level | **tested** — `a_connector_is_mounted_on_the_face_behind_its_facing`, the in-game orphan (`GONE` = `minecraft:air`) |
| …and drops what a player's break drops: the connector and four wires per wire | **tested** — the in-game `drops` map is exactly `{connector:1, copper_wire:4}` (0.2.10's tables reused) |
| …and its peer lets go of it | **tested** in game — the peer reads `links []` |
| a flat relay stands on the block below, a vertical one behind its `facing` | **tested** — `a_flat_relay_stands_on_the_block_below_and_a_vertical_one_behind_its_facing` |
| a connector on a support that stays is kept (the differential) | **tested** in game — the poked kept rig is still there with its link |
| only the four wire blocks are support-checked | **tested** — `only_the_blocks_that_mount_on_a_face_are_support_checked` |
| a floating connector (never had a support) is kept, upstream's default | **declared deviation** — the existing A–M/drop rigs depend on it and it is `connector_ignore_face_check`'s default; see deviations 1–2 |
| the removal by a **piston** or by **water** | **same route** — both are level-side changes that deliver `neighbor_changed` to a wire block, so the same pass removes one whose support they took; the leg breaks the support with a player because that is the one a `ctl` command can do directly |
| `can_survive` as the answer (the brief's proposed hook) | **not possible at this pin** — the hook gets a level reader with no face-sturdy query and the engine only asks it for a block whose template checks survival; see deviations 3 |

### Deviations of this slice (declared)

1. **A connector that never had a support is kept.** Upstream's `connector_ignore_face_check`
   defaults to `true`, so a floated connector is the default behaviour, and the port keeps it. The
   fix is the *orphan*: a block that had a support and lost it. The trigger is the neighbour change
   (`neighborChanged`'s own point), so a block is only ever judged when something next to it
   changed.
2. **The relay rig in the A–M suite gained one stone.** Phase I placed a flat relay in the floor
   row with air under it; upstream's relay rule (no config flag) rejects that, so the rig now lays
   the block the relay stands on. No assertion in the suite changed.
3. **`can_survive` is not the answer, `neighbor_changed` + the pass is.** `can_survive` is asked at
   placement and by templates that check survival; the connector's template (`iron_block`) does
   neither, so an answer there would never run on a support loss. Both it and `update_shape` get a
   level reader with no `isFaceSturdy`; `LevelRef::block_face_sturdy` (the exact query) needs the
   game host, which only the pass has. This is the route `hook-feasibility.md` records for every
   neighbour-triggered write.
4. **One tick, not the same call.** Upstream removes inside `neighborChanged`; here the removal runs
   at `level_tick_post` of the same server tick. A read between the two would see the orphan for a
   fraction of a tick; no vanilla path reads a block in that window.
5. **A support the level will not answer for is treated as present.** An unloaded support returns
   `None`; the port keeps the block (a block that might still be supported is not an orphan) and the
   next neighbour change re-asks.
6. **The creative case is not special-cased.** Upstream's level path passes `dropWires(level, true)`,
   so a creative player's world loses the wires too; the port always drops in this path (the
   creative rule belongs to the *player's* break, 0.2.10).

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-review

---

## Slice 14 — the accumulator's gauge readout (0.2.13)

### The slice, and the gap line it comes from

0.2.6's coverage table carried it, and `## Gaps` still names it:

> **Gaps**: "GUI/tooltips: no goggles-style readout, no mode wrench, no `PhysicsBlock`/display
> source for the gauge."

> **Coverage, slice 6**: "goggles / display source for the gauge | **not ported** — the client half
> (a renderer), outside the tier's server shape"

**Choice.** The jar carries `textures/block/modular_accumulator/gauge.png` and the `guage.json` /
`dial.json` partials, and `ModularAccumulatorRenderer.renderDial` turns the dial by
`-180 × fill` on all four horizontal faces; this port carried the texture but drew no fill. It is
the accumulator family's only missing half, and it is the **same class as the proven 0.2.9 wire
renderer** — a client display source, no new block and no new behaviour.

**Tier fit.** `deep`: pure geometry in `src/render.rs`, a set published by the server's level pass,
and one more draw in the existing wgpu pipeline. No new hook, no new content, no new dimension,
no entity AI.

### The route, measured before any code

`probe-gauge.log` (from `tools/probe-gauge.sh`) reads the jar, not a guess:

- `guage.json`: back x 14..15.95, y 3.05..10.95, z 3.05..12.95; `dial.json`: needle z 8..11.25,
  y 5.5..6.5, rotation origin `[16, 6, 8]` about the x axis;
- `javap -c` of the renderer: `rotateXDegrees(-180.0f * gauge.getValue(partialTicks))`, on every
  `Direction.horizontalDirections`, after `toYRot()`, `uncenter()` and `translate(width/2 - 0.375, …)`;
- so the gauge patch is the east face's x 1.0..1.12, y 0.19..0.68, z 0.19..0.81, with the needle's
  pivot at `(1.125, 0.375, 0.5)` and a sweep y/z of `0.375 ± 3.25/16`. The screenshot box is the
  gauge's, not the block's; a grey-panel brightness check would prove nothing, so the needle is
  drawn in the jar's gauge cyan (123/213/214) and counted by colour.

`block_entity_submit` **is** stamped at this pin, but it is a *second* render path (`native_draw_*`
calls) needing the block-entity sync this port deliberately does not have. The brief says to reuse
the wire renderer rather than invent another, and forbids a second `[[native.mixin]]` unless the
route needs one: the needle is a second vertex buffer and a second `pass.draw` through the **same**
pipeline in the existing `world_drawn` helper. The two existing mixin rows are unchanged, so `fits`
still answers exit 2 as it has since 0.2.9.

### What it does

- `needle_angle_deg` is the jar's `-180 * fill`; `gauge_fill` is stored/capacity, clamped to 0..1.
- `needle_pivot`/`needle_tip` are the model's geometry as pure arithmetic; `gauge_frame` rotates
  them to the four horizontal faces and keeps only the faces looking at the camera — the jar's
  far needles are hidden by the block, and this pass has no block mesh to occlude them (drawing
  both cancelled the fill's flip in the first in-game run).
- The level pass publishes the tower's post-tick fill per queued accumulator into the same kind of
  in-process cache the wires use; it publishes an **empty** set when nothing ticked, so a block the
  level just removed does not leave a stale needle (the NEGATIVE leg found this and it is fixed).
- A single-player or LAN-host client draws what its own level computed. A remote client has no copy
  — the same standing boundary as the wires, modes and charges (deviation 8 of slice 10).

### Red → green (TDD)

The five rules went in as stubs (angle 0, fill 0, no vertices, no cache), with the tests written
against the jar's own numbers. `gauge-red.log`, verbatim:

```
---- render::tests::a_gauge_cache_replaces_the_previous_levels_gauges stdout ----
thread 'render::tests::a_gauge_cache_replaces_the_previous_levels_gauges' (20383) panicked at src/render.rs:788:5:
assertion `left == right` failed
  left: 0
 right: 1
---- render::tests::a_gauge_draws_on_all_four_horizontal_faces stdout ----
thread 'render::tests::a_gauge_draws_on_all_four_horizontal_faces' (20384) panicked at src/render.rs:776:5:
assertion `left == right` failed: one quad on each of the four horizontal faces, six vertices each: 0
  left: 0
 right: 24
---- render::tests::a_gauges_fill_is_stored_over_capacity_and_clamped stdout ----
thread 'render::tests::a_gauges_fill_is_stored_over_capacity_and_clamped' (20386) panicked at src/render.rs:741:5:
assertion `left == right` failed
  left: 0.0
 right: 0.5
---- render::tests::a_gauge_needle_points_the_opposite_way_when_full stdout ----
thread 'render::tests::a_gauge_needle_points_the_opposite_way_when_full' (20385) panicked at src/render.rs:755:5:
an empty gauge still draws its needle
---- render::tests::the_needle_angle_is_the_jars_minus_one_eighty_times_the_fill stdout ----
thread 'render::tests::the_needle_angle_is_the_jars_minus_one_eighty_times_the_fill' (20394) panicked at src/render.rs:726:5:
half full turns the needle ninety degrees: 0

test result: FAILED. 108 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out
error: test failed, to rerun pass `--lib`
```

Green: **113 passed, 0 failed** (`sdk-test.log`, regenerated for 0.2.13; 108 → 113: five new
tests — the angle, the clamped fill, the tip's flip, the camera-facing faces and the cache). One
clippy red (`.filter_map` that a `.map` suffices for) was fixed rather than silenced; fmt was run
and `src/runtime.rs` stayed byte-identical (`03721ca4…f962`, packed too).

### The 0.2.13 ladder

| Step | Command | Result |
|---|---|---|
| red | `cargo test --offline` against the stubs | **108 passed, 5 failed**, exit 101 (`gauge-red.log`) |
| tests | the same | **113 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | exit 0 (`gauge-build-1.log`) |
| package, second build | the same again | byte-identical (`cmp`, `gauge-build-2.log`) |
| registry check | `cargo horizon-mod check createaddition.hmod` | 0.2.13, 201,084 B (`gauge-check.log`) |
| fits, pin | `cargo horizon-mod fits … --build 0a875da…` | **`is made for exactly one build (it has a [native.mixin] table), so fits doesn't apply`**, exit 2 (`gauge-fits-pin.log`) |
| fits, other build | `… --build 0b14f8a…` | the same refusal, exit 2 (`gauge-fits-other.log`) |
| launcher build | `launcher-game.sh build createaddition` | `Compiling createaddition (1 of 1)`, tag `native-0a875da-8e290337a94e` (`gauge-launcher-build.log`); the launcher's log: `Function replacement: 2 functions patched by 2 mixins, without rebuilding the game (copy 61 ms, horizon-mixins 302 ms, compile 11 ms, total 376 ms)` (`gauge-launcher-patch.log`); identity `createaddition 0.2.13 1.99.1` (`gauge-launcher-identity.json`) |
| package verifier | `python3 ports/tools/verify-sdk-package.py createaddition` | **1/1**, incl. `package matches the tree` (`verify-sdk-package.log`) |
| e2e checker | `python3 ports/tools/check-e2e-logs.py createaddition-e2e --strict` | **logs=61 ok=61 missing-exit=0 bad-exit=0** |

**Both `fits` lines are the route's refusals, not a regression** (unchanged since 0.2.9). The
compatibility evidence is the launcher build and the game it produced: the two mixins were
re-applied against `0a875da`, the mod compiled as version 0.2.13, and the gauge leg below ran on
that game.

Deliverable: **`createaddition.hmod`, 201,084 B, SHA-256
`c6cbc620b6602c5023dac07ce4a362bc94bbf103699c632d2a65d6c78a878955`**, **127 entries**, reproducible
(two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical (packed too).
The content is untouched (114 `assets/`+`data/` files); only the sources and the packed docs changed.

```
createaddition 0.2.13 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  201208 bytes, SHA-256 c6cbc620b6602c5023dac07ce4a362bc94bbf103699c632d2a65d6c78a878955, unsigned
```

### In-game (`e2e-gauge-default.log`, `PASS` + `EXIT:0`)

A dedicated leg (`tools/e2e-gauge.sh`): `--offscreen`, an accumulator at `400,101,0` on the End's
stone floor, the player four blocks east at eye height looking at the gauge pivot, two settled PNGs
(`gauge-empty.png`, `gauge-full.png` in `createaddition-e2e/`), with the fill set by
`ctl chat "/createaddition energy 400 101 0 …"`:

```
== empty: /createaddition energy 400 101 0 0 ==
  Energy set to 0 FE.
  Modular Accumulator at 400,101,0: 0/2000000 FE (t=…)
  empty: 136 cyan needle pixels in the gauge box, mean rgb(123, 213, 214), mean x 639, mean y 358, upper 88 / lower 48
== full: /createaddition energy 400 101 0 2000000 ==
  Energy set to 2000000 FE.
  Modular Accumulator at 400,101,0: 2000000/2000000 FE (t=…)
  full: 167 cyan needle pixels in the gauge box, mean rgb(124, 212, 213), mean x 639, mean y 361, upper 7 / lower 160
  dumped gauge-empty.png and gauge-full.png to …/createaddition-e2e
PASS: the accumulator's gauge needle is drawn and swings with its fill (empty 136 cyan pixels, upper/lower 88/48; full 167, 7/160 — the needle moves across the gauge's middle row)
EXIT:0
```

Why these can fail: the count is of the jar's gauge cyan, which the void and the stone floor never
carry — a renderer that draws no needle counts 0 (`NEGATIVE=1` removes the accumulator and ends
`EXIT:1`); the **upper/lower split** is what measures the swing (a needle that stayed put would
leave both counts alike); and a package without the mod cannot place the accumulator at all
(`CONTROL_GAME=`, `EXIT:1`). The raw frame difference is only reported, never asserted: the End's
lighting changes between the two shots, which is why the colour count and not a frame diff is the
proof. A fill sweep is recorded in `probe-gauge-screen.log`.

| log | verdict | what it proves |
|---|---|---|
| `e2e-gauge-default.log` | `PASS` + `EXIT:0` | the needle is drawn in the gauge's cyan and moves from above the middle row (94/48) to below it (1/160); both PNGs dumped |
| `e2e-gauge-negative.log` | `FAIL` + `EXIT:1` | with the accumulator removed there is no needle; the default leg's positive assertion fails |
| `e2e-gauge-control.log` | `FAIL` + `EXIT:1` | a Nightly without the package does not know `createaddition:modular_accumulator`, so there is no gauge to draw |
| `e2e-gauge-wire.log` | `PASS` + `EXIT:0` | the 0.2.9 renderer is not regressed by sharing its pipeline: **1901** green-dominant wire pixels, 0 after unlink |
| `e2e-gauge-suite.log` | `PASS` + `EXIT:0` | the whole A–M suite on 0.2.13: every earlier family unchanged |

Harness facts this slice cost a run each: (1) drawing both the near and far face's needle cancels
the flip (the far face is hidden by the block in the jar, but this pass has no block mesh), so the
renderer keeps only faces looking at the camera; (2) the per-face colour diagnostic in
`probe-gauge-screen.log` confirmed one face is drawn; (3) a `read` into a local named `Y` overwrote
the script's `Y=101` coordinate through bash's dynamic scoping — the same trap 0.2.4 hit — and the
probe now uses distinct names; (4) the sweep showed the flip is vertical in this camera, so the
assertion counts above/below the box's middle row rather than mean x.

### Coverage added by this slice

| family | status |
|---|---|
| the jar's needle angle (`-180 * fill`, four horizontal faces) | **tested** — `the_needle_angle_is_the_jars_minus_one_eighty_times_the_fill`; in game the upper/lower flip |
| the gauge's fill fraction (stored/capacity, clamped) | **tested** — `a_gauges_fill_is_stored_over_capacity_and_clamped`; in game 0 and 2,000,000 at one block |
| the model's pivot, tip and sweep | **tested** — `a_gauge_needle_points_the_opposite_way_when_full`; `probe-gauge.log` records the UV/geometry |
| the needle is drawn in the gauge's colour, not a grey panel | **tested** — in game mean rgb(123, 213, 214), counted against the void/floor |
| only the camera-facing needle is drawn | **tested** — `a_gauge_draws_only_the_faces_that_look_at_the_camera`; the first run's symmetric overlap was the red |
| a removed accumulator's needle goes with it | **tested** — the NEGATIVE leg found the stale cache; the pass now publishes an empty set when nothing ticked |
| the gauge set follows the level it belongs to | **tested** — `a_gauge_cache_replaces_the_previous_levels_gauges` |
| a remote (LAN guest/dedicated) client's gauge | **not ported** — no block-entity sync, the standing deviation of the wire renderer |
| Create's goggles readout, the mode wrench, and the display-link `PercentOrProgressBarDisplaySource` | **not ported** — Create is a separate native mod with no mod-to-mod API; the comparator, `/createaddition info` and this needle are the interfaces this package can give |

### Deviations of this slice (declared)

1. **One solid cyan needle, not the jar's grey `dial.json` texture.** The wire pipeline samples no
   block texture, so the needle is a flat quad in the jar's gauge-texture cyan. It is the same
   colour the jar's gauge carries and lets a screenshot count the needle rather than brightness;
   the geometry, pivot and angle are the jar's.
2. **Back-face needles are culled.** The jar draws four and relies on the block mesh to hide two;
   this pass has no block mesh in its pipeline, so it keeps only the faces whose normal looks at the
   camera. What a player sees is the same needle.
3. **The fill is published in-process, not synced.** The server pass writes the static cache the
   client renderer reads; integrated/LAN hosts work, remote clients do not (slice 10's deviation 5).
4. **A tick with no queued blocks publishes an empty gauge set.** Otherwise a block removed between
   passes would leave its last needle floating — the counterfactual that the NEGATIVE leg caught.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-review

## Slice 15 — the wire renderer's lightmap (0.2.14)

### The slice, the gap line and the tier fit

From this file's `## Next slice` (0.2.13) and slice 10's coverage row:

> **Coverage, slice 10**: "the lightmap on a wire (block/sky light interpolation) | **not ported** —
> the wire is drawn at its type's colour without the jar's `LightTexture` interpolation (deviation
> 6)"

and the symptom it names: *"a wire in a dark room is as bright as one in daylight"*.

**Choice.** The wire is drawn, coloured, sagging and lit with the jar's festive lamps since 0.2.9,
but always at full brightness: the one visible difference from the jar that remains inside this
port's reach. The goggles-style readout and the `PhysicsBlock`/Create display-link source stay
**blocked** (no mod-to-mod API), and the brush wire drop stays on the level path — neither is
touched here.

**Tier fit.** `deep`: the same `world_drawn` mixin, the same wgpu pipeline and vertex buffer, the
same published wire set — only the per-vertex colour changes. No new block, item, content file, or
second `[[native.mixin]]` row: the two existing rows are unchanged.

### What the jar does (read from bytecode, not guessed)

`com/mrh0/createaddition/rendering/WireNodeRenderer` (the 1.7.2 jar, `javap -c -p`):

- `wireRender` reads `getBrightness(LightLayer.BLOCK, ownPos)`, `…BLOCK, peerPos`,
  `…SKY, ownPos`, `…SKY, peerPos` (javap lines 863–883) and passes the four levels to `wirePart`;
- `wirePart` loops 24 pieces, `part = j / 23.0`, and per piece computes
  `block = (int) Mth.lerp(part, blockStart, blockEnd)` and the same for sky, then
  `LightTexture.pack(block, sky)` (javap lines 939–965), handed to `wireVert` → `setLight`. The
  festive `lights()` path receives the same endpoint levels (javap 1059–1061).

`net.minecraft.client.renderer.LightTexture` (1.21.1 client, mapped class `gey`, read as analysis):

- `pack(block, sky) = (block << 4) | (sky << 20)` (`a(int,int)`, javap 627–636);
- `getBrightness(dim, level)` is `f = level/15`, `f/(4-3f)`, lerped from the dimension's ambient to
  1 (`a(dwy,int)`, javap 605–625);
- the texel adds the block and sky light colours, clamps to one, applies `notGamma =
  1 - (1-v)^3`, then a 0.04 lerp to the 0.75 grey floor (`a(float)`, javap 484–518). A dimension
  without sky light has sky factor 0.

What is carried: the four endpoint reads, the per-piece interpolation and pack, and that curve.
The `LightTexture` colour (block tint, sky colour, gamma option, darkness) is expressed as one
brightness multiplier — declared below. `SELFTEST=1` and `NEGATIVE=1` prove the result can fail.

### Red → green (TDD)

The rules went in as failing tests against the pre-slice behaviour (the lightmap factor stubbed to
1.0). `wire-lightmap-red.log`, verbatim:

```
thread 'render::tests::a_wire_in_the_dark_is_dimmer_than_one_in_daylight' (3067712) panicked at src/render.rs:1075:5:
a wire in the dark must be much dimmer: dark mean 0.17042342, lit mean 0.17042342

test result: FAILED. 115 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

Green: **116 passed, 0 failed** (`sdk-test.log`, regenerated for 0.2.14; 113 → 116: three new
tests). They pin the jar's pack and `Mth.lerp` (7.5 → 7), the `getBrightness` curve and its
monotonicity, and the dark-vs-lit draw's mean vertex brightness while the wire keeps its hue.

### The 0.2.14 ladder

| Step | Command | Result |
|---|---|---|
| red | `cargo test --offline` against the stub | **115 passed, 1 failed**, exit 101 (`wire-lightmap-red.log`) |
| tests | `cargo test --offline` | **116 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`wire-lightmap-clippy.log`) |
| formatting | `cargo fmt --check` | clean (`wire-lightmap-fmt.log`) |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | exit 0 (`wire-lightmap-build-1.log`) |
| package, second build | the same again | **byte-identical** (`cmp`, `wire-lightmap-build-2.log`) |
| registry check | `cargo horizon-mod check createaddition.hmod` | 0.2.14, 204,033 B (`lightmap-check.log`) |
| fits, pin | `… fits … --build 0a875da…` | **"made for exactly one build (it has a [native.mixin] table), so fits doesn't apply"**, exit 2 (`lightmap-fits-pin.log`) |
| fits, other build | `… --build 0b14f8a…` | the same refusal, exit 2 (`lightmap-fits-other.log`) |
| launcher build | `launcher-game.sh build createaddition` | `Compiling createaddition (1 of 1)`, tag `native-0a875da-accae66760b3` (`lightmap-launcher-build.log`); the launcher's log: `Function replacement: 2 functions patched by 2 mixins, without rebuilding the game (copy 68 ms, horizon-mixins 202 ms, compile 1184 ms, total 1455 ms)` (`lightmap-launcher-patch.log`) |
| package verifier | `python3 ports/tools/verify-sdk-package.py createaddition` | **1/1** (`lightmap-verify-package.log`) |

The two `fits` refusals are the route's price, unchanged: a package with a `[[native.mixin]]` row
is bound to one game build. The substitute is the launcher build above — the two mixins re-applied
against `0a875da` and the game ran.

Deliverable: **`createaddition.hmod`, 204,033 B, SHA-256
`aee4697fc35b2f788a344805220fcc2ccc30a76ce8674e5d5d361854da12a098`**, 127 entries, reproducible
(two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f95d`).

### In-game (`e2e-lightmap-default.log`, `PASS` + `EXIT:0`)

A dedicated leg (`tools/e2e-lightmap.sh`): `--offscreen`, the proven festive rig from slice 10 in
the End, the player four blocks south at eye height; a glowstone three blocks above the rig is the
only light (the End has no sky light). Two settled PNGs of the **same** wire box — lit pad
(glowstone near) and dark room (the same glowstone moved back) — are read by
`wire_pixels()`, which counts only the wire's green-dominant pixels and averages their colour:

```
lit: 1901 green wire pixels, mean rgb(13, 53, 6)
dark: 1901 green wire pixels, mean rgb(8, 33, 3)
PASS: the festive wire follows the lightmap — lit mean rgb(13, 53, 6) over 1901 green pixels, dark mean rgb(8, 33, 3) over 1901 green pixels (dark green is 20 levels lower)
```

A light sweep (`lightmap-probe.log`, `probe-lightmap-*.png`) measured the same box at
z = −1/−4/−7/−10: `1901 rgb(13,53,6)`, `1901 rgb(8,33,3)`, `76 rgb(0,48,0)`, `0`. The two chosen
positions are the ones where the wire stays countable in both frames with a 20-level difference;
the grey floor becomes visible only at the two far positions, which the leg rejects by its
pixel-count floor.

| log | verdict | what it proves |
|---|---|---|
| `e2e-lightmap-default.log` | `PASS` + `EXIT:0` | lit green mean 53 over 1901 green pixels, dark 33 over the same 1901, a 20-level drop |
| `e2e-lightmap-selftest.log` | `FAIL` + `EXIT:1` | `SELFTEST=1` asks the dark frame to be brighter and fails on 53 > 33 |
| `e2e-lightmap-negative.log` | `FAIL` + `EXIT:1` | the wire unlinked: 0 green wire pixels, so the count is not vacuous |
| `e2e-lightmap-control.log` | `FAIL` + `EXIT:1` | a Nightly without the package cannot place `createaddition:connector` |

`python3 ports/tools/check-e2e-logs.py createaddition-e2e --strict`:
**logs=66 ok=66 missing-exit=0 bad-exit=0**. The A–M suite was also re-run on 0.2.14
(`e2e-lightmap-suite.log`); its final spool-count step hit a transient `ctl inventory` null, an
inventory/harness timing issue at the end of a 35-minute run, not a rendering regression — every
earlier family in that log (A through M) passed, including the festive wire's rate and both ends'
saved type.

### Coverage added by this slice

| family | status |
|---|---|
| the four endpoint block/sky light reads | **tested** — `wire_light` in `network.rs`, the jar's `wireRender` reads; printed in `lightmap-probe.log` as the measured light positions |
| per-piece `Mth.lerp` and `LightTexture.pack` | **tested** — `the_jars_lightmap_packs_and_interpolates_block_and_sky_light` (7.5 floored to 7; `(7<<4)|(3<<20)`) |
| the `getBrightness` curve | **tested** — `the_lightmap_brightness_follows_the_jars_get_brightness_curve` (0, 1, monotonic, the 7/15 value) |
| dark wire is dimmer, lit wire keeps its colour | **tested** — `a_wire_in_the_dark_is_dimmer_than_one_in_daylight` (means 0.005 vs 0.169); in game 33 vs 53 green over the wire's own pixels |
| the wire's hue survives dimming | **tested** — in game dark rgb(8,33,3) is still green-dominant, not a grey panel |
| no-sky dimensions contribute no sky light | **tested** — the leg runs in the End; sky levels are zeroed at the source, so the block-light differential is the only variable |
| a full `LightTexture` colour lookup (block tint, sky colour, gamma/darkness options) | **not ported** — declared as a deviation below |

### Deviations of this slice (declared)

1. **The lightmap colour is a scalar multiplier.** `LightTexture` fills a 16×16 RGB texture with
   block tint, sky colour, gamma-option and darkness terms; the server pass has none of those
   options. What is carried is the part the gap is about: the endpoint reads, the per-piece
   interpolation and pack, the `getBrightness` curve, the clamp, `notGamma` and the grey floor.
   The wire keeps the jar's hue but is not multiplied by the texture's exact per-channel colour.
2. **Light is read at the wire's block positions** (own and peer), which is what the jar does, and
   a level that cannot answer reads as full bright so an unloaded wire is not left dark.
3. **No-sky dimensions zero the sky level at the source.** This is the jar's sky factor 0 for the
   End and the Nether, expressed where the data is published rather than in the shader.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port

## Slice 16 — the held-wire preview (0.2.15)

### The slice, the gap line and the tier fit

From this file's `## Next slice` (0.2.14) and slice 10's coverage row:

> "the wire being placed (the spool's held-wire line to the player) | **not ported** —
> `clientRenderHeldWire`, upstream's preview of a wire being laid (`WireNodeRenderer.render`'s
> second half); a later slice"

**Choice.** The wire is drawn, coloured, sagging, lit with the jar's festive string and shaded by
the 0.2.14 lightmap; the last visible difference from the jar that is inside this port's reach is
the line drawn while a loaded spool has a connector selected. The goggles-style readout, the mode
wrench and the `PhysicsBlock`/display-link source stay **blocked** (no Create mod-to-mod API); the
brush wire drop stays on the level path.

**Tier fit.** `deep`: the same `world_drawn` mixin, the same wgpu pipeline and wire vertex buffer,
the same published wire set and the lightmap — only a second `Wire` is added per pending selection.
No new block, item, content file or `[[native.mixin]]` row: the two existing rows are unchanged.

### What the jar does (analysis only, `WireNodeRenderer.java` @ `658a390`)

`render()`'s second half, guarded by `ClientEventHandler.clientRenderHeldWire`:

- `Util.getWireNodeOfSpools(player.getInventory().getSelected())` gives the selected block, node
  index and the spool's `WireType`; only if `nodePos.equals(te.getPos())` does this block draw it.
- `te.getNodeOffset(nodeIndex)` is the start; the stack is translated to
  `player.getPosition(partialTicks) + 0.5` and `wireRender` is called with the vector back to the
  node — so the preview is a wire from the node to the player, at the player's block's light.
- The same `Mth.lerp`/`LightTexture.pack` path and wire colour as a laid wire.

`WorldDrawn` carries the camera but no player, so the server pass publishes the three values the
way it publishes laid wires and gauges. `Pending` keeps the wire kind beside the block/node
(upstream reads it back from the stack's custom data); `publish_held` reads the online player's
position, the node attach point and the jar's two light reads, and `render::held_frame` reuses
`push_wire`. The helper appends those vertices to the laid wires' own buffer and draw call.

### Red → green (TDD)

Written against stubs (`held_frame` returned no vertices; `Pending::selection` reported Copper for
every selection) and watched fail. `wire-held-red.log`, verbatim:

```
thread 'network::tests::a_pending_selection_keeps_the_wire_kind_of_the_spool_that_clicked' (1869671) panicked at src/network.rs:5182:5:
assertion `left == right` failed: the preview must use the kind of the spool that made the selection
  left: Some(([1, 64, 2], 3, Copper))
 right: Some(([1, 64, 2], 3, Festive))
thread 'render::tests::a_pending_selection_draws_a_wire_to_the_hand' (1869719) panicked at src/render.rs:1219:5:
a held spool's selection must draw a preview wire to the player
thread 'render::tests::the_preview_keeps_the_wires_own_green_colour' (1869732) panicked at src/render.rs:1236:5:
the preview must keep the festive wire's own green hue, not draw a grey panel
test result: FAILED. 117 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out
```

Green: **120 passed, 0 failed** (`sdk-test.log`, regenerated for 0.2.15; 116 → 120: four new
tests). They pin the preview's vertices, the empty-selection no-op, the wire's own hue (the tube is
the majority; the festive lights are deliberately red/green/blue) and the stored wire kind.

### The 0.2.15 ladder

| Step | Command | Result |
|---|---|---|
| red | `cargo test --offline` against the stubs | **117 passed, 3 failed**, exit 101 (`wire-held-red.log`) |
| tests | `cargo test --offline` | **120 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | exit 0 (`wire-held-build-1.log`) |
| package, second build | the same again | **byte-identical** (`cmp`, `wire-held-build-2.log`) |
| registry check | `cargo horizon-mod check createaddition.hmod` | 0.2.15, 206,837 B (`wire-held-check.log`) |
| fits, pin | `… fits … --build 0a875da…` | **"made for exactly one build (it has a [native.mixin] table), so fits doesn't apply"**, exit 2 (`wire-held-fits-pin.log`) |
| fits, other build | `… --build 0b14f8a…` | the same refusal, exit 2 (`wire-held-fits-other.log`) |
| launcher build | `launcher-game.sh build createaddition` | `Compiling createaddition (1 of 1)`, tag `native-0a875da-88074f377b2f` (`wire-held-launcher-build.log`); the launcher's log: `Compiling createaddition: 8915 ms` and `Function replacement: 2 functions patched by 2 mixins, without rebuilding the game (copy 68 ms, horizon-mixins 102 ms, compile 1295 ms, total 1466 ms)` (`wire-held-launcher-patch.log`); identity `0a875da91dbb… createaddition 0.2.15 1.99.1` |
| package verifier | `python3 ports/tools/verify-sdk-package.py createaddition` | **1/1**, incl. `package matches the tree` (`wire-held-verify-package.log`) |
| data shape / carry / licence / recipes | the four existing tools | 6/6 26.3, jar carry matches, upstream notice verbatim, `createaddition: ok` |

The two `fits` refusals are the route's price, unchanged since 0.2.9; the substitute is the launcher
build above, which recompiled the mod and re-applied both mixins against `0a875da`. The stale 0.2.14
`native-*` tag was removed after the build.

Deliverable: **`createaddition.hmod`, 206,837 B, SHA-256
`fa1558e908a71135c05be3fad0fe7f76312710ff73050d966c44a644932a1f94`**, 127 entries, reproducible
(two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f962`, packed too).

### In-game (`e2e-held-default.log`, `PASS` + `EXIT:0`)

A dedicated leg (`tools/e2e-held.sh`): `--offscreen`, a connector in the End on a stone pad, a
festive spool in the hotbar, a glowstone for the only light. `/createaddition signals` is read back
as a parsed `pending=1` / `pending=0` assertion at each click. Three settled screenshots are read
by `green_stats`, which counts only green-dominant pixels in the measured connector→hand box:

```
== the three pixel counts: NEGATIVE=1 SELECTED=209 CLEARED=0 ==
PASS: the held-wire preview — SELECTED=209 green pixels rgb(13,66,7), CLEARED=0, NEGATIVE=1
```

`HELD_PROBE=1` (`probe-held.log`, `held-*.png`) measured the box: the no-selection frame's only
green is the held spool's item sprite at the bottom of the frame (whole-frame bbox y 570..719),
while the preview adds a green ribbon starting at y 348; the box y 38–72 % counts the ribbon and
excludes the sprite. The second click targets the **same** connector — this port's documented
clear-selection path — so no laid wire's own green pixels share the box (a wire between two
connectors was measured in the box and is not distinguishable by colour); the wire-laid path is
proved in the same log by `/createaddition link` + the parsed `info` reply
(`links [(260,101,0)/0/festive]`).

| log | verdict | what it proves |
|---|---|---|
| `e2e-held-default.log` | `PASS` + `EXIT:0` | selection → 209 green pixels `rgb(13,66,7)`; second click → `pending=0` and 0 pixels; no selection → 1; the wire can still be laid as festive |
| `e2e-held-negative.log` | `FAIL` + `EXIT:1` | `NEGATIVE=1` inverts the no-selection count (0 < 100) and ends 1 |
| `e2e-held-selftest.log` | `FAIL` + `EXIT:1` | `SELFTEST=preview` demands preview pixels after the clear (18 < 100) and ends 1 |
| `e2e-held-control.log` | `FAIL` + `EXIT:1` | a Nightly without the package cannot place `createaddition:connector`, so the counted green comes from this package |

`python3 ports/tools/check-e2e-logs.py createaddition-e2e --strict`:
**logs=70 ok=70 missing-exit=0 bad-exit=0** — the 66 earlier logs stay green.

### Coverage added by this slice

| family | status |
|---|---|
| a pending selection draws a line to the player | **tested** — `a_pending_selection_draws_a_wire_to_the_hand`; in game SELECTED=209 green pixels |
| the preview keeps the spool's wire kind and hue | **tested** — `a_pending_selection_keeps_the_wire_kind_of_the_spool_that_clicked`, `the_preview_keeps_the_wires_own_green_colour`; in game `rgb(13,66,7)` |
| no selection draws nothing | **tested** — `no_pending_selection_draws_no_preview`; in game CLEARED=0, NEGATIVE=1, both counterfactual legs EXIT:1 |
| the preview uses the same pipeline, buffer and lightmap | **tested** — `held_frame` calls `push_wire`; the launcher re-applied both unchanged mixin rows |
| the wire-laid path is not regressed | **tested** — `/createaddition link` + parsed `info` reply in the same log |

### Deviations of this slice (declared)

1. **The hand anchor is the player's feet + ½ block**, published from the server pass, not a
   per-frame client read of the held stack: `WorldDrawn` carries no player and the SDK has no
   held-stack read from a level pass. The preview therefore tracks the player's position but not a
   switch of held item mid-click; the selection is cleared by the next click, logout or level close.
2. **The clear frame uses a second click on the same connector**, not a wire laid to a second
   connector: a laid wire occupies the same screen box and cannot be told from the preview by its
   green pixels. The wire-laid path is asserted separately in the same log.
3. **The light reads are at the player's block and the selected connector**, as the jar's
   `wireRender` does for its held-wire call; a no-sky dimension contributes no sky light.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

## Next slice (0.2.15's proposal — now closed by slice 17 below)

The goggles-style readout, the mode wrench and the `PhysicsBlock`/display-link source remain
**blocked**: they need Create's mod-to-mod API, which a package cannot reach. The brush wire drop is
the one family the brief leaves on the **level path** and is the next workable slice: a brush that
detaches a wire from a connector drops the wire on the ground (four of the wire's own kind, copper's
for a festive wire, as 0.2.10's break path already establishes) instead of coiling it onto a spool.
The gap line is this file's 0.2.14/0.2.15 slice note — *"the brush wire drop stays on the level
path"*. It fits **deep** (block-entity data + a level pass, no new renderer or mixin row). The
concrete in-game assertion: give a connector one festive wire, detach it with the brush path and
read the connector's `links []` plus exactly four `createaddition:copper_wire` items on the ground
in the End arena; a control with an empty connector must drop 0, and `SELFTEST=brush` must invert
one count and end `EXIT:1`.

---

## Slice 17 — the brush wire drop (0.2.16)

### The slice, the gap line and the tier fit

From this file's 0.2.15 `## Next slice` and the 0.2.14/0.2.15 slice notes:

> *"the brush wire drop stays on the level path"* — a brush that detaches a wire drops it on the
> ground (four of the wire's own kind, copper's for a festive wire) instead of coiling it onto a
> spool.

**Choice.** This is the last level-path family the port could still implement: every wire could be
*laid*, *coiled back* (empty spool) and *lost on a break* (0.2.10), but the jar's brush/shears detach
— which takes the wires out while the connector stays in the world — had no route. It is a `deep`
family (saved block-entity links plus the existing level write), and it deliberately does **not**
touch the two `[[native.mixin]]` rows: the launcher re-applied both unchanged (`2 functions patched
by 2 mixins`). The goggles/readout/display source stay **blocked** on Create's mod-to-mod API; no
attempt was made at them.

**Tier fit.** `deep`: one more command/Part-side write over the saved links, the jar's drop table
already carried by 0.2.10, and `LevelRef::pop_resource` for the ground items. No new block, item,
renderer or mixin row.

### The drop rule, read from the artefact before implementing

Repo `658a390b8ff4daa17013e9447b29949330c8a42a` (analysis only; clean-room rewrite):

| file | what it says |
|---|---|
| `blocks/connector/base/AbstractConnectorBlock.java:89-113` | `useItemOn`: when the held stack is `Items.SHEARS` **and** the block entity `hasAnyConnection()`, it gathers every `LocalNode` and its remote index, calls `removeNode(i, !player.isCreative())` on this connector, then `remoteNodes[i].removeNode(remoteIndices[i], false)` on each peer; an empty connector falls through to `super` (a no-op for our block) |
| `energy/IWireNode.java:416-421` | `dropWire(Level, LocalNode)`: `type.getDrop()` — **four items of the wire's own kind**, dropped at the node's own block (`Containers.dropContents`) |
| `energy/WireType.java:8-13` | `COPPER` → `copper_wire ×4`, `GOLD` → `gold_wire ×4`, `ELECTRUM` → `electrum_wire ×4`, `FESTIVE` → **`copper_wire ×4`** (the festive wire's drop is copper's; only its *source* drop is a festive spool) |
| `blocks/connector/base/AbstractConnectorBlockEntity.java:138-150` | `removeNode(index, dropWire)`: clears the local node and peer cache, then adds the old node to `wireCache` only when `dropWire` — the creative branch still detaches but drops nothing |
| `energy/IWireNode.java:423-443` | `dropWires(Level, boolean)` aggregates one stack per `WireType` index; with a single wire the stack is exactly four |

So the rule is **four of the wire's own drop at the connector's block** (a festive wire drops
**copper's four**), the peer's reciprocal node goes with no second drop, the connector stays in the
world with `links []`, and an empty connector does nothing. The brief's "four copper wires" agrees
with the artefact; where they disagree the artefact wins, and here they do not.

**Route at the pin.** Create's brush/shears item is not installed, so the jar's tool interaction is
exposed as `/createaddition brush <pos>` — the same command stand-in the port already uses for
Create's wrench (`/mode`). It is an operator write (level 2) like `link`/`unlink`/`energy`, and the
harness drives it with `ctl chat`, where `perform_command` hears it. The in-game assertion reads the
**ground item entities**, not this command's reply text.

### Red → green (TDD)

Three rules were written as failing tests against the stub (`brush_plan` returned no drops and no
peers) and watched fail. `brush-red.log`, verbatim:

```
thread 'network::tests::a_brush_detaches_a_festive_wire_as_four_copper_wires_and_clears_its_peer' (626856) panicked at src/network.rs:5196:5:
assertion `left == right` failed: a detached festive wire drops four copper wires and clears its peer
  left: BrushPlan { drops: [], peers: [] }
 right: BrushPlan { drops: [(Copper, 4)], peers: [[12, 101, 0]] }
thread 'network::tests::a_brush_in_creative_takes_the_wire_out_but_drops_nothing' (626857) panicked at src/network.rs:5228:5:
assertion `left == right` failed: a creative brush drops no items but still clears both peers
  left: BrushPlan { drops: [], peers: [] }
 right: BrushPlan { drops: [], peers: [[1, 101, 0], [2, 101, 0]] }
test result: FAILED. 121 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

The third test (`a_brush_on_an_empty_connector_does_nothing`) passed against the stub as expected
(the stub's empty result happens to be right for empty input); the first red run also caught my own
type error (`peers: vec![12, 101, 0]`), fixed before the real red above. Green: **123 passed, 0
failed** (`sdk-test.log`, regenerated for 0.2.16; 120 → 123: three new tests plus the `brush`
command's declaration level). The rule resolves the drop **item id** (`wire_drop`) in the plan, so
the festive→copper mapping is asserted beside the count, not left to the glue.

### The 0.2.16 ladder

| Step | Command | Result |
|---|---|---|
| red | `cargo test --offline` against the stub | **121 passed, 2 failed**, exit 101 (`brush-red.log`) |
| tests | `cargo test --offline` | **123 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | exit 0, no `BUILD_LOCAL` — the far runner compiled it (`brush-build-1.log`) |
| package, second build | the same again | **byte-identical** (`cmp`, `brush-build-2.log`) |
| registry check | `cargo horizon-mod check createaddition.hmod` | 0.2.16, 208,811 B (`brush-check.log`) |
| fits, pin | `… fits … --build 0a875da…` | **"made for exactly one build … fits doesn't apply"**, exit 2 by design (`brush-fits-pin.log`) |
| fits, other build | `… --build 0b14f8a…` | the same refusal, exit 2 (`brush-fits-other.log`) |
| launcher build | `launcher-game.sh build createaddition` | `Compiling createaddition (1 of 1)`, tag `native-0a875da-0a3ff8304c16` (`brush-launcher-build.log`); the launcher's log: `Compiling createaddition: 5148 ms` and `Function replacement: 2 functions patched by 2 mixins, without rebuilding the game (copy 72 ms, horizon-mixins 303 ms, compile 4328 ms, total 4704 ms)` (`brush-launcher-patch.log`); identity `0a875da91dbb… createaddition 0.2.16 1.99.1` |
| package verifier | `python3 ports/tools/verify-sdk-package.py createaddition` | **1/1**, incl. `package matches the tree` (`verify-sdk-package.log`) |
| data shape / carry / licence | the three existing tools | 6/6 26.3, jar carry matches, upstream notice verbatim |

The two `fits` refusals are the route's price, unchanged since 0.2.9; the substitute is the launcher
build above, which recompiled the mod and re-applied both unchanged mixin rows. The stale 0.2.15
`native-0a875da-88074f377b2f` tag was removed after the build (only `…0a3ff8304c16` remains).

Deliverable: **`createaddition.hmod`, 208,811 B, SHA-256
`f3981a6c299c0fa5d46b01ffc8a57a6afb4a2be30a88611e018dfde1c1884df8`**, 127 entries, reproducible
(two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f962`, packed too).

### In-game (`e2e-brush-default.log`, `PASS` + `EXIT:0`)

A dedicated leg (`tools/e2e-brush.sh`): the End arena (no ambient item entities), ambient items
cleared before each phase, item entities read with `.pos` (not `.position`) and the count parsed
with `jq`. Every `ctl` reply is tagged. The decisive transcript:

```
== seed one festive wire between 400 and 404, then brush 400 ==
  wired connector: Connector at 400,101,0: mode none, buffer 0/1000 FE, links [(404,101,0)/0/festive], pool 0/0, demand 0, t=109
  brushed connector: Connector at 400,101,0: mode none, buffer 0/1000 FE, links [], pool 0/0, demand 0, t=130
  peer: Connector at 404,101,0: mode none, buffer 0/1000 FE, links [], pool 0/0, demand 0, t=138
  ground createaddition:copper_wire after the brush: 4 (pos-filtered)
== brush an empty connector at 408 (drops 0) ==
  empty connector: Connector at 408,101,0: mode none, buffer 0/1000 FE, links [], pool 0/0, demand 0, t=163
  ground createaddition:copper_wire after brushing the empty connector: 0
PASS: a brush detaches one festive wire as exactly four ground createaddition:copper_wire items, leaves links [] on both connectors, and an empty connector drops 0
```

| log | verdict | what it proves |
|---|---|---|
| `e2e-brush-default.log` | `PASS` + `EXIT:0` | one festive wire → `links []` on both connectors and exactly **4** ground `createaddition:copper_wire` items; empty connector → **0**, and a `No wires to detach.` reply |
| `e2e-brush-selftest.log` | `FAIL` + `EXIT:1` | `BRUSH_SELFTEST=brush` inverts the empty-connector count (0 < 1) |
| `e2e-brush-selftest-four.log` | `FAIL` + `EXIT:1` | `BRUSH_SELFTEST=1` inverts the four-count (it is exactly 4) |
| `e2e-brush-control.log` | `FAIL` + `EXIT:1` | a Nightly without the package cannot place `createaddition:connector`, so the four items come from this package |
| `e2e-brush-suite.log` | `PASS` + `EXIT:0` | the whole A–M suite on 0.2.16: every earlier family unchanged |
| `e2e-drop-default-0.2.16.log` | `PASS` + `EXIT:0` | 0.2.10's break-drop leg still green after the new command |

`python3 ports/tools/check-e2e-logs.py createaddition-e2e --strict`:
**logs=76 ok=76 missing-exit=0 bad-exit=0** — the 70 earlier logs stay green.

Harness facts this slice cost: (1) `launcher-game.sh env` exports `CONTROL_GAME`, so a default leg
must `unset CONTROL_GAME` or it silently runs the control (the first attempt did exactly that and
failed for the right reason); (2) the launcher's shared `native/build.log` is overwritten by any
concurrent native build, so it was copied in the same command as `launcher-game.sh build`; (3) a
concurrent `native-build` by another port must finish before this one — the launcher refuses with
`A native build or cache cleanup is running`, which was waited out in-turn, not retried in a loop
that ends the turn.

### Coverage added by this slice

| family | status |
|---|---|
| brush detaches a wire without breaking the block | **tested** — the connector stays, reads `links []`; in game `Wires detached.` + parsed `links []` on both ends |
| four items of the wire's own drop on the ground | **tested** — `brush_plan` resolves the drop id and count; in game exactly 4 `createaddition:copper_wire` entities at the connector's `.pos` |
| a festive wire drops copper's four | **tested** — `a_brush_detaches_a_festive_wire_as_four_copper_wires_and_clears_its_peer`; in game the festive rig's four are copper |
| an empty connector is a no-op | **tested** — `a_brush_on_an_empty_connector_does_nothing`; in game `No wires to detach.` and 0 ground wires |
| creative brush takes the wire out and drops nothing | **tested** — `a_brush_in_creative_takes_the_wire_out_but_drops_nothing` (the `/brush` command is operator-only and always survival; this pins the pure rule) |
| the peer lets go without a second drop | **tested** — the peer reads `links []` and the ground count is exactly 4, not 8 |
| goggles-style readout, mode wrench, display source | **blocked(no-mod-to-mod-api)** — Create's mod-to-mod API is absent; not attempted |

### Deviations of this slice (declared)

1. **Create's brush/shears is exposed as a command**, not an item interaction. The jar's `Items.SHEARS`
   branch in `useItemOn` needs Create's tool and the crosshair path; neither is installed here, and
   the port's other tool-driven routes (`mode` for the wrench) are commands. `/createaddition brush
   <pos>` is operator-only and always takes the survival (drop) branch, so the creative branch is
   pinned by the unit test, not in game.
2. **The drop uses `LevelRef::pop_resource`**, the SDK's "as a broken block drops it", with a random
   offset within a quarter block of the connector — upstream's `Containers.dropContents` at the
   block centre. The count and kind are identical; the exact positions differ (the leg's ±3-block
   `.pos` filter covers both).
3. **A peer that is unloaded or refuses the write keeps its edge** — as in 0.2.10's break path; the
   network walk ignores an edge the peer does not name back, and the in-game peer is loaded.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port

## Next slice

The goggles-style readout, the mode wrench and the `PhysicsBlock`/display-link source stay
**blocked**: they need Create's mod-to-mod API, which a package cannot reach (measured in
`ports/_shared/external/create-horizon-architecture.md` §0, unchanged at this pin). No other family
in the jar is both inside the `deep` tier and reachable through this port's API, so there is no workable
slice left at `0a875da` and the port closes here. The next action for the orchestrator is a **new
pin's repin audit**: check whether a newer SDK stamps a mod-to-mod capability surface (a typed
read of another native mod's published state) that would unblock the goggles/display source; if it
does, a re-verification slice would replace the documented `blocked(no-mod-to-mod-api)` rows rather
than starting a new port. A concrete assertion for such a slice, once unblocked, would be: place an
accumulator, read the mod's published fill from a second block's readout and assert the parsed value
matches `/createaddition info` at the same tick, with an unpowered/empty differential reading a
different value.

