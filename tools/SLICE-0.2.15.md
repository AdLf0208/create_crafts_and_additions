# Slice 0.2.15 — the held-wire preview

## Choice and gap line

From `notes.md` § `## Next slice` (0.2.14) and slice 10's coverage row:

> "the wire being placed (the spool's held-wire line to the player) | **not ported** —
> `clientRenderHeldWire`, upstream's preview of a wire being laid (`WireNodeRenderer.render`'s
> second half); a later slice"

Tier **deep** (the ledger's row): the same `world_drawn` mixin, the same wgpu pipeline and wire
vertex buffer, the same published wire set and the lightmap carried in 0.2.14. **No second
`[[native.mixin]]`**: the two existing rows (`wire-world-drawn`, `wire-render-needs`) stay byte-for-byte
unchanged. The goggles/readout/display source stays blocked on Create's mod-to-mod API; the brush
wire drop stays on the level path.

## What the jar does (analysis only, `WireNodeRenderer.java` @ `658a390`)

`render()`'s second half, guarded by `ClientEventHandler.clientRenderHeldWire`:

- `Util.getWireNodeOfSpools(player.getInventory().getSelected())` →
  `Triple<BlockPos, Integer, WireType>`: the selected block, node and the spool's wire kind.
- Only if `nodePos.equals(te.getPos())`: `te.getNodeOffset(nodeIndex)` is the start; the stack is
  translated to `player.getPosition(partialTicks) + 0.5`, and `wireRender` is called with the vector
  from that anchor back to the node — so the preview is a wire from the node to the player.
- `wireRender` reads block/sky brightness at `te.getBlockPos()` and `player.blockPosition()`, the
  same `Mth.lerp`/`LightTexture.pack` path as a laid wire, and draws with the spool's `WireType`.

## Plan

1. **TDD red:** pure tests in `render.rs` (a pending selection produces vertices; none produces
   none; the preview keeps the wire's green hue) and `network.rs` (a pending selection keeps the
   spool's wire kind). Stub the three new functions so the tests fail with assertion diffs.
2. **Glue:** `render::{HeldWire, publish_held, held, held_frame}` reuse `push_wire` (geometry +
   0.2.14 lightmap). `network::publish_held` reads `Pending` + `Players::server_players` in the
   level pass, publishing node→(feet + ½) with the jar's two light reads; `level_tick_post` passes
   its `pending` store to `level_ticked`.
3. **Ladder** through `build-slot.sh` (test, clippy, fmt, two `cmp`-identical builds, check, both
   `fits`, gate, launcher build with the patch line on the new sha). `sdk-test.log` regenerated.
4. **In-game:** `tools/e2e-held.sh` — `--offscreen`, real spool clicks, `/createaddition signals`
   parsed for `pending=1/0`, three green-pixel counts in the connector→hand box: selection (≫floor),
   after the second click (0), `NEGATIVE=1` no selection (0). `SELFTEST=preview` inverts one count.
5. Bump to 0.2.15, **docs before the final build**, remove stale `native-*` tags, evidence, notes,
   VALIDATION. The 66 earlier e2e logs stay green.
