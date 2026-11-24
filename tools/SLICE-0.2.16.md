# Slice 0.2.16 — the brush wire drop

## Choice and gap line

From `notes.md` § `## Next slice` (0.2.15) and the 0.2.14/0.2.15 slice notes:

> "the brush wire drop stays on the level path — a brush-detached wire drops four of its own kind
> on the ground instead of coiling onto a spool"

Tier **deep** (the ledger's row): saved block-entity links plus the existing level write and the
0.2.10 drop table. **No `[[native.mixin]]` change**: the two existing rows stay byte-for-byte
unchanged, and the launcher re-applies both (`2 functions patched by 2 mixins`). The
goggles/readout/display source stays **blocked** on Create's mod-to-mod API — not attempted.

## What the jar does (analysis only, `658a390b8ff4daa17013e9447b29949330c8a42a`)

`AbstractConnectorBlock.useItemOn`: when `Items.SHEARS` is held **and** the node
`hasAnyConnection()`, every `LocalNode` is removed with `removeNode(i, !player.isCreative())` and
each peer's reciprocal node with `removeNode(peer, false)`; an empty connector falls through to
`super` (a no-op). `IWireNode.dropWire(Level, LocalNode)` drops `type.getDrop()` at the node's block
— **four items of the wire's own kind**, one stack per `WireType`. `WireType.FESTIVE`'s drop is
**`copper_wire ×4`** (only its source drop is a festive spool). The connector stays in the world.

## Plan

1. **TDD red:** pure tests in `network.rs` (a festive wire drops four copper items and clears its
   peer; an empty connector does nothing; a creative brush drops nothing but still clears). Stub
   `brush_plan` so the first and third fail with assertion diffs. Carry the artefact's rule, not the
   brief's number, if they disagree (they do not here).
2. **Glue:** `brush_detach` clears the saved links on the connector, drops `pop_resource` stacks at
   its position, and clears each peer's reciprocal link without a second drop. `/createaddition
   brush <pos>` is the operator command stand-in for Create's absent brush/shears item; the reply
   text is not the assertion.
3. **Ladder** through `build-slot.sh` (test, clippy, fmt, two `cmp`-identical builds, check, both
   `fits`, gate, launcher build with the patch line on the new sha). `sdk-test.log` regenerated.
   No `BUILD_LOCAL` — the far runner compiles the crate and the package.
4. **In-game:** `tools/e2e-brush.sh` in the End arena, ambient items cleared before each phase,
   item entities read with `.pos` (not `.position`): one festive wire, brush, parse `links []` on
   both ends and exactly **4** ground `createaddition:copper_wire`; an empty connector drops **0**.
   `BRUSH_SELFTEST=brush` inverts the zero count, `BRUSH_SELFTEST=1` inverts the four; a
   `CONTROL_GAME` leg without the package fails.
5. Bump to 0.2.16, **docs before the final build**, remove the stale `native-*` tag, evidence,
   notes, VALIDATION. The 70 earlier e2e logs stay green.
