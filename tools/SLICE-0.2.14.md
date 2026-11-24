# Slice 0.2.14 — the wire renderer's lightmap

## Choice and gap line

From `notes.md` § `## Next slice` (0.2.13) and slice 10's coverage row:

> "the lightmap on a wire (block/sky light interpolation) | **not ported** — the wire is drawn at
> its type's colour without the jar's `LightTexture` interpolation (deviation 6)"

and the in-game symptom: *"a wire in a dark room is as bright as one in daylight"*.

Tier **deep** (the ledger's row): the same `world_drawn` mixin, the same wgpu pipeline and vertex
buffer, the same wire set — only the per-vertex colour changes. No new block, item, content file or
`[[native.mixin]]` row. The goggles readout and Create display-link stay blocked (no Create
mod-to-mod API); the brush wire drop stays on the level path.

## What the jar does (read from bytecode, not guessed)

`com/mrh0/createaddition/rendering/WireNodeRenderer` (1.7.2 jar):
- `wireRender` reads `Level.getBrightness(LightLayer.BLOCK, pos)` at both wire endpoints (4 reads:
  block/sky × start/end, javap lines 863–883) and passes them to `wirePart`.
- `wirePart` loops 24 pieces, `part = j / 23.0`, and per piece computes
  `Mth.lerp(part, start, end)` cast to `int` for block and sky, then
  `LightTexture.pack(block, sky)` (javap lines 939–965), passed to `wireVert` → `setLight`.
- The festive `lights()` path receives the same endpoint light values (javap 1059–1061).

`net.minecraft.client.renderer.LightTexture` (1.21.1 client jar, `pack` mapping `a(int,int)`):
`pack(block, sky) = (block << 4) | (sky << 20)`; `getBrightness(dim, level)` is
`f/(4-3f)` lerped from the dimension's ambient to 1, with `f = level/15`. The lightmap texel adds
block and sky contributions, clamps, applies `notGamma` (`1-(1-v)^3`) and a 0.04 lerp to the grey
floor. A no-sky dimension (End/Nether) has sky factor 0.

## Plan

1. TDD red: pure light tests (pack, lerp, curve, dark-vs-lit vertex brightness).
2. Glue: `Wire.light` filled by `publish_wires` from `level.brightness(…)`; `frame()` shades each
   segment (and each festive light) by the pack→lightmap factor; existing pipeline unchanged.
3. Ladder (test, clippy, fmt, two builds, check, both fits, gate).
4. In-game: `tools/e2e-lightmap.sh` — two `--offscreen` screenshots of the festive rig, lit pad vs
   dark room; count the wire's own green pixels in the wire box and print both pixel means; assert
   the dark mean is measurably lower. `SELFTEST=1` inverts; `NEGATIVE=1` removes the wire.
5. Bump to 0.2.14, docs before the final build, launcher build, evidence, notes/VALIDATION.
