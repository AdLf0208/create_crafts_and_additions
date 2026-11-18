# What was carried from the jar, and what was written

`tools/carry_content.py` re-derives the whole tree from
`ports/_shared/licence-cache/createaddition-1.7.2.jar`; `--check` rebuilds it in
a temporary folder and exits 1 if `assets/` or `data/` differs. **68 files** (the jar's
`assets/` + `data/` for the slice) at the **package root**, 151,691 B unpacked — carried into
`assets/createaddition/` and `data/createaddition/` of the SDK package, which packs them with the
code (no separate content build; the retired `content/createaddition.hmod` and its own manifest are
gone). `python3 tools/carry_content.py --check` prints `assets/ and data/ match the jar carry`.

## Carried verbatim

| What | From the jar | Note |
|---|---|---|
| Block models | `assets/createaddition/models/block/connector/{small,large}/{none,pull,push,item,block}.json`, `…/modular_accumulator/{block_single,item}.json` | elements and UVs untouched |
| Item models | `…/models/item/{connector,large_connector,modular_accumulator,copper_wire,gold_wire,electrum_wire,spool,copper_spool,gold_spool,electrum_spool}.json` | `item/generated` where the jar has it |
| Textures | 22 PNGs, every texture a carried model names, plus the bulb and the accumulator gauge | the small connector's `gold_none` is real: the **large** connector's idle model names it |
| Loot tables | `data/createaddition/loot_table/blocks/{connector,large_connector,modular_accumulator}.json` | the jar already uses the singular `condition` 26.3 shape |
| Lang | 26 keys from `assets/createaddition/lang/en_us.json` | names, the spool tooltips, the wire status lines, the energy tooltips |

## Patched

1. **Particle texture.** Every connector and accumulator block model names
   `create:block/andesite_casing` as its particle. Create is not installed, so
   the carry rewrites any `create:` texture reference to
   `createaddition:block/connector/connector`.
2. **Blockstates.** The jar's `connector.json`/`large_connector.json` are
   multiparts over `facing` × `mode` × `variant` (the girder mount); this port
   declares `facing` and `mode` only, so the carry writes those 18 (×2) parts
   and the jar's own rotations (`down` y=90, `up` x=180 y=90, `north` x=90
   y=180, `south` x=90, `west` x=90 y=90, `east` x=90 y=270). The accumulator
   is a single variant on `block_single`, because the multiblock is a gap.
3. **Block declarations** (`data/createaddition/horizon/block/*.json`) are
   written: `minecraft:iron_block` template, hardness 1.5/2.0, copper sounds,
   `occludes: false` for the connectors, the six `facing` values and the three
   `mode` values, a 4×4-pixel nub shape per facing, and `item: true`.
4. **Item declarations** (`…/horizon/item/*.json`) use `minecraft:copper_ingot`
   as their template (stack size and look only; the carried model overrides the
   look).
5. **26.3 item definitions** (`assets/createaddition/items/*.json`) are written
   for the ten items, as the pin's schema requires.

## Not carried

- **Recipes.** Every upstream recipe for these blocks needs `create:` items
  (`andesite_alloy`, `brass_casing`) or `c:` tags that do not exist without the
  parent mod, so the recipes could never match. The blocks and items are
  creative-obtainable; crafting is a gap (see `notes.md`).
- **The `charging/` recipe type** and its 40+ oxidisation recipes: they belong
  to the rotation half (the electric motor powers them).
- **`small_light_connector`, `creative_energy`, `portable_energy_interface`,
  `redstone_relay`, `energy_meter`, `electric_pump`, `digital_adapter`**, the
  cables/spools of iron and festive wire, and the electrum/brass/copper metal
  items: other slices.
- **Advancements and tags** the jar ships for these blocks (its
  `data/…/advancement/` entries name `create:` items).
