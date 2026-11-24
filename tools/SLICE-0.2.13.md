# Slice 14 plan — the accumulator's gauge readout (0.2.13)

## Choice and gap line

`notes.md` → **Gaps**: *"GUI/tooltips: no goggles-style readout, no mode wrench, no
`PhysicsBlock`/display source for the gauge."* Also the 0.2.6 coverage row:
*"goggles / display source for the gauge | not ported — the client half (a renderer),
outside the tier's server shape"*.

The jar carries `textures/block/modular_accumulator/gauge.png` and the `guage.json` /
`dial.json` partials; `ModularAccumulatorRenderer.renderDial` rotates the dial by
`-180 × fill` degrees about its pivot on all four horizontal faces. Nothing in this
port draws the fill. This slice is that client half only — no new block, no behaviour.

**Tier fit:** `deep`, the same class as the proven 0.2.9 wire renderer. No new hook,
no new content, no new `[[native.mixin]]`.

## Route (measured, not guessed)

Reuse the 0.2.9 machinery: pure geometry in `src/render.rs`, a static set published
by the server level pass, and the **same** wgpu pipeline in
`src/mixins/world_drawn.rs` (one extra `pass.draw` with a second vertex buffer).
`block_entity_submit` exists at the pin but is a *second render path* (native draw
calls) needing block-entity sync this port deliberately does not have; the brief
says reuse the wire path rather than invent one, and the trap forbids a second
`[[native.mixin]]` unless the route needs it. No new mixin is added; the two
existing rows stay, so `fits` still answers exit 2.

## Order

1. Probe the gauge UV/geometry + needle angle → `probe-gauge.log` (measure first).
2. TDD: needle-angle, gauge-vertices and cache tests red against the pre-slice code.
3. Glue: publish the tower fill per accumulator from `level_ticked`; draw the needle
   in `world_drawn.rs` with the wire pipeline.
4. Ladder, version bump 0.2.13, two `cmp`-identical builds, check, fits ×2, gate.
5. Launcher build (patch line + sha), `e2e-gauge.sh` (offscreen screenshot differential,
   two PNGs, pixel numbers, `NEGATIVE=1` → EXIT:1), strict e2e check.
6. Docs (notes/VALIDATION/README) BEFORE the final build; report.
