# Slice 11 (0.2.10) — the wire's break-drop path

Gap line (this port's own 0.2.9 report and `notes.md` § *Slice 10* → next slice):

> the port has no wire-drop path at all … `WireType.getDrop()` is therefore not exercised

## The rules, read from the artefact before implementing

Upstream: MRH0/createaddition `658a390b8ff4daa17013e9447b29949330c8a42a` (the ATM10 jar is
`createaddition-1.7.2.jar`, MIT). Analysis only; the port stays a clean-room rewrite.

| file | what it says |
|---|---|
| `energy/WireType.java:8-13` | `COPPER(0,256, …, COPPER_WIRE.asStack(4), COPPER_SPOOL)`, `GOLD(1,1024, …, GOLD_WIRE.asStack(4), GOLD_SPOOL)`, `ELECTRUM(2,8196, …, ELECTRUM_WIRE.asStack(4), ELECTRUM_SPOOL)`, `FESTIVE(3,256, …, COPPER_WIRE.asStack(4), FESTIVE_SPOOL)` — the festive wire's **drop is copper wire ×4**, only its *source* drop is a festive spool |
| `energy/WireType.java:47-53` | `getDrop()` / `getSourceDrop()` return `DROP.copy()` / `SOURCE_DROP.copy()` |
| `blocks/connector/base/AbstractConnectorBlock.java:83-92` | `playerWillDestroy`: server side, `IWireNode` ⇒ wire-break sound when `hasAnyConnection()`, then `cte.dropWires(level, !player.isCreative())` |
| `energy/IWireNode.java:300-318` | `dropWires(Level, boolean)`: `!dropItems` ⇒ `disconnectWires()` only (no items); else one `ItemStack` **per `WireType` index** — the first is `type.getDrop()`, later ones `grow(getDrop().getCount())` — and each is `dropWire(world, getPos(), stack)` = `Containers.dropContents` **at the broken block** |
| `blocks/connector/base/AbstractConnectorBlockEntity.java:320-350` | on block-entity `remove()` every peer's reciprocal node is `removeNode(ourNode)` **without** a drop when our node is already invalid ⇒ the peer's list empties, nothing drops twice |
| `energy/IWireNode.java:320-352` | the **three-arg** `dropWires(Level, Player, boolean)` (an empty spool in the player's inventory ⇒ `getSourceDrop()`, else `getDrop()`; `getInventory().add` else drop) is `onSneakWrenched`'s path — the port already has its spool half (`remove_wires`/`spools_back`) |

So the break path is: **four wire items of the wire's kind per wire, at the broken block, one
stack per kind, only outside creative, and the peer's link removed without a second drop.**

## The route at the pin

`hook:on_destroyed_by_player` (`IBlockExtension.onDestroyedByPlayer`, contract replace, scope
block) is a **`BlockPart` method** — asked for our blocks only, *before* anything of the break
changes the world, with `player: u128`, `pos`, `state`, `tool`, `will_harvest`; answering `None`
leaves vanilla's removal and loot alone. `host.players().player(uuid)` resolves the player for
`is_creative`. `LevelRef::pop_resource(host, pos, stack)` drops "as a broken block drops it".
Stamped: `mod_api_generated.rs:8621` and the part fixture `destroyed_by_player_game_thread.rs`.

## Files

- `src/network.rs` — the pure tables + the Part method (no `src/runtime.rs` change: it stays
  byte-identical).
- `tools/e2e-drop.sh` — the new leg (default / `DROP_SELFTEST=1` / `CONTROL_GAME=`).
- `horizon-mod.toml`, `Cargo.toml` — 0.2.10.
- `notes.md`, `VALIDATION.md` — the slice record.

## Tasks

1. **Red** — `wire_drop`, `source_drop`, `break_drops`, `links_after_break` in `network.rs` with
   the pre-slice bodies (no drops, no link pruning) + 5 unit tests; `cargo test` fails, quoted.
2. **Green** — fill the tables; add `on_destroyed_by_player` to `ConnectorShape`, `RelayShape`,
   `LightShape` (one shared `break_wires` fn): drop the aggregated stacks at `pos`, then write
   each loaded peer's list without `pos`. `cargo test` green.
3. **Ladder** — `cargo test`, `clippy --all-targets -- -D warnings`, `fmt --check`, all through
   `build-slot.sh run` with `CARGO_INCREMENTAL=0`.
4. **Package** — bump to 0.2.10, `rm -f Cargo.lock` + `cargo horizon-mod build` twice, `cmp`;
   `check`; two `fits` runs (expected: exit 2, "made for exactly one build"); `verify-sdk-package`
   1/1.
5. **In game** — `launcher-game.sh build createaddition` (a fresh `native-*` tag, no stale tag
   carrying 0.2.9), then `e2e-drop.sh`:
   - copper / gold / electrum / festive rigs: lay the wire, stand 3.5 blocks off, `ctl break` the
     far connector in survival, assert `ctl break`'s own `drops` map is exactly that kind's wire
     ×4 plus the connector ×1, and the peer's `info` reads `links []`;
   - differential: an unlinked connector broken in the same run drops **no** wire;
   - `DROP_SELFTEST=1` asserts the unlinked break *did* drop a wire ⇒ must fail, `EXIT:1`;
   - `CONTROL_GAME=` cannot place `createaddition:connector` ⇒ fails, `EXIT:1`.
6. **Suite** — the A–M run on the same build (`e2e-drop-suite.log` + its selftest), then
   `check-e2e-logs.py createaddition-e2e --strict`; `check-recipe-types.py createaddition`.
7. **Record** — `notes.md` slice section, coverage rows, `Skills:` line; `VALIDATION.md`.
