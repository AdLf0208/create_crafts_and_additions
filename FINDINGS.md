# Create Crafts & Additions — findings (FE half)

Upstream: **Create Crafts & Additions** `1.7.2`, MIT, by MRH0
(`https://github.com/mrh0/createaddition`, analysed at commit
`658a390b8ff4daa17013e9447b29949330c8a42a`, 2026-11-02, `--depth 1`; the
clone was analysis scratch and is deleted after this record, per AGENTS.md).
Jar evidence: `ports/_shared/licence-cache/createaddition-1.7.2.jar`
(1,124 entries). Analysis is behaviour only; the port is a clean-room rewrite.

## What the artefact says (not the catalogue)

### Licence and dependencies — read from `META-INF/neoforge.mods.toml`

| Field | Value |
|---|---|
| `license` | `MIT` (rank 0, permissive; still local-only until publishing approval, ARCHITECTURE §8) |
| `modId` | `createaddition` |
| `version` | `1.7.2` |
| required | `neoforge` `[21.1.248,)`, `create` `[6.0.7,6.1.0)`, `minecraft` `[1.21.1]` |
| optional | `jei`, `computercraft`, `simulated` |

`computercraft` is **optional** (lines 56–67 of the jar's `mods.toml`), so the
only non-provided half of the mod is `digital_adapter`; the FE slice does not
need it. `create` is required, and the parent is **upstream**
(`Delete228/create-horizon` @ `236c87a`, `create` 0.7.0-alpha.2, MIT) — there
is **no mod-to-mod API** (`ports/_shared/external/create-horizon-architecture.md`
§0), so this port cannot link, produce or join Create's kinetic network. It
does not try. The `create:` items its recipes need are likewise absent.

## Jar inventory (measured)

- **1,124 entries.** 26 blockstate files, 45 item models, 18 `en_us`-style lang
  files, 20 block loot tables, 79 recipes (`crafting/`, `charging/`,
  `mixing/`, `crushing/`), plus block/item textures.
- **Blocks with a block entity:** `connector` (small), `large_connector`,
  `small_light_connector`, `modular_accumulator`, `electric_motor`,
  `alternator`, `rolling_mill`, `tesla_coil`, `electric_pump`,
  `portable_energy_interface`, `redstone_relay`, `liquid_blaze_burner`,
  `creative_energy`, `energy_meter`, `digital_adapter`, `servo_motor`,
  `modular_accumulator` parts.
- **Items:** `copper_wire`, `gold_wire`, `electrum_wire`, `iron_wire`, `spool`,
  `copper_spool`, `gold_spool`, `electrum_spool`, `festive_spool`, capacitor,
  electrum ingot/nugget/sheet/rod, brass/copper/gold/iron/zinc rods, biomass
  and pellets, seed-oil/bioethanol buckets, cakes, sandpaper, amulets,
  `digital_adapter`.
- **The FE half is:** `connector`, `large_connector`, `modular_accumulator`,
  `redstone_relay`, `small_light_connector`, the wire and spool items, and (not
  in this slice) `creative_energy`, `portable_energy_interface`, `energy_meter`,
  `electric_pump`, `digital_adapter`.
- **The rotation half is:** `electric_motor`, `alternator`, `rolling_mill`,
  `tesla_coil`, `servo_motor`, and the `charging/` recipe type they power.

## Behaviour extracted (with the numbers this slice enforces)

### Connectors — `blocks/connector/base/AbstractConnectorBlockEntity.java`

- The block state is `facing` + `mode` + `variant` + `rotation`; `facing` is the
  side the connector is mounted on. Placement sets
  `facing = clickedFace.getOpposite()`.
- **Placement mode** (`ConnectorMode.test`, `ConnectorMode.java:74-88`): ask the
  block on the mounted side for its energy capability — if it `canExtract()`
  the connector is **Pull**, else if it `canReceive()` it is **Push**, else
  **None**. An accumulator (both) therefore places as **Pull**; players cycle
  it with Create's wrench (`getNext`: None → Pull → Push → None; `Passive`
  exists in the enum but is unreachable from `getNext`).
- **Transfer per tick** (`networkTick`, `:283-307`):
  - **Pull**: `otherStorage.extractEnergy(network.push(getMaxIn(), true), false)`
    — extract from the mounted block, limited by `maxIn` and the network's
    free buffer.
  - **Push**: `network.pull(network.demand(otherStorage.receiveEnergy(getMaxOut(), true)))`
    then `otherStorage.receiveEnergy(pulled, false)` — insert into the mounted
    block, limited by `maxOut` and what the network holds.
  - The connector's own `Capabilities.Energy.BLOCK` handler is a window on the
    network: `getEnergyStored() = min(capacity, network.getBuff())`, and the
    capability is only reachable from the mounted side
    (`isEnergyInput||isEnergyOutput` = `FACING == side`). Capacity is
    `min(maxIn, maxOut)`.
  - Config (`CommonConfig.java:170-201`): small connector **1000 FE/t in and
    out**, wire length **16 blocks**; large connector **5000 FE/t in and out**,
    wire length **32**; network internal buffer ceiling **80,000 FE**.
  - Node counts: small **4**, large **6** (`getNodeCount`).
- **Wires** (`energy/IWireNode.java:565-593`): `connect` refuses when
  `pos1.distSqr(pos2) > maxLength²`, where `maxLength = min(both connectors'
  length)`; refuses a second wire on the same node, refusals carry
  `LONG / EXISTS / COUNT / INVALID / REQUIRES_HIGH_CURRENT`, and **copper (or
  festive) between two large connectors is refused** (`REQUIRES_HIGH_CURRENT`).
- `WireType` (copper/gold/electrum/festive) carries a `transfer` value
  (256/1024/8196) that **nothing in 1.7.2 reads**: the throughput limit is the
  connector's, not the wire's. Recorded, not enforced.
- **Wires are linked with a spool item** (`item/WireSpool.java`): the first
  right click stores `{x,y,z,node}` in the stack's custom data, the second
  connects, consumes one spool and gives the empty spool back; the empty spool
  removes a link. Node choice comes from the click location
  (`getAvailableNode(clickLocation)`).
- **Energy network** (`energy/network/EnergyNetwork.java`): one buffer per
  wire graph, **double-buffered** — `tick()` swaps `inBuff` and `outBuff`, so
  energy pushed in one tick can only be pulled in the next (a 1-tick latency);
  `getMaxBuff() = min(members * (outDemand + 2*inDemand + 10), 80000)`.

### Accumulator — `blocks/modular_accumulator/`

- Config (`CommonConfig.java:205-218`): **2,000,000 FE per block**, input
  **5000/t**, output **5000/t**, multiblock at most 5 high × 3 wide.
- `tickOutput()` pushes at `ACCUMULATOR_MAX_OUTPUT` into the blocks on its
  **UP and DOWN** faces only, when the `top`/`bottom` state bits are set (both
  true on a single block); every other side is a passive capability
  (`receiveEnergy` up to 5000/t, `extractEnergy` up to 5000/t).
- One accumulator block is a controller and its energy lives in
  `EnergyContent { energy }` in its saved compound.

### Redstone relay — `blocks/redstone_relay/`, `energy/network/WireGraph.java`

- `RedstoneRelayBlockEntity.NODE_COUNT = 8`; `WireNodeKind.RELAY` gives node 0–3 port 0 (input) and
  4–7 port 1 (output), `portCount() = 2`, `isBridge() = true`, and
  `isPowered(state) = state.getValue(POWERED)`.
- `getMaxWireLength()` is `SMALL_CONNECTOR_MAX_LENGTH` (16) and `getConnectorType()` is
  `ConnectorType.Small`; a relay is a low-current node.
- `getAvailableNode(click)`: the click's half of the block picks the port (horizontal: the `facing`
  side is the output; vertical: the halves swap), and a full half returns -1 rather than using the
  other.
- `RedstoneRelayBlock.updateState` schedules a 2-tick tick when `calculateInputStrength` crosses 0:
  horizontal reads `getSignal`/`getDirectSignal` on the two sides across the long axis (and redstone
  wire power), vertical reads `getSignal` above and below. `setBlockState` tells the wire graph.
- `WireGraph.tickBridges`: for a loaded bridge, `throughput = 0` and it `continue`s unless
  `powered`; otherwise `out.push(in.pull(bridge.demand))` and `demand = in.demand(out.getDemand())`
  — the input network's buffer feeds the output network's, and the output network's demand sizes the
  input buffer.
- No energy capability: the relay is a wire node, not a storage block.

### Creative generator — `blocks/creative_energy/`, `energy/CreativeEnergyStorage.java`

- `CreativeEnergyStorage` (the block entity's registered `Capabilities.Energy.BLOCK` handler,
  for **every** context): `receiveEnergy` → **0**, `extractEnergy(maxExtract)` → **maxExtract**
  unchanged, `getEnergyStored`/`getMaxEnergyStored` → **`Integer.MAX_VALUE`**, `canExtract` →
  true, `canReceive` → false.
- `CreativeEnergyBlockEntity.tick`: after `super.tick()`, for each of the six directions, the
  cached neighbour capability (created on the first tick, refreshed by `neighborChanged`) gets
  `ies.receiveEnergy(Integer.MAX_VALUE, false)` — an unlimited offer, which the neighbour's own
  handler caps.
- `CreativeEnergyBlock` extends Create's `CrateBlock` **only for the block's look**;
  `getShape` is `CREATIVE_ENERGY_SHAPE = CAShapes.shape(1,0,1,15,16,15).add(0,2,0,16,14,16)`,
  and the block registers `CABlockEntities.CREATIVE_ENERGY` for its entity.
- Content: `blockstates/creative_energy.json` (one variant, `""`),
  `models/block/creative_energy/{block,item}.json`, `models/item/creative_energy.json`, a 26.3
  loot table for the jar's 1.21.1 one, and three `en_us` keys (`Creative Generator`, the tooltip
  title and the summary "Generates an infinite supply of _⚡_."). The model's three texture slots
  name Create's `crate_creative` / `creative_fluid_tank`, which are **not in this jar**.

## Where this port differs (declared)

1. **No kinetic half.** `electric_motor`, `alternator`, `rolling_mill`,
   `tesla_coil`, `servo_motor` and the `charging/` recipe type are a gap: the
   parent Create is a separate native mod with no mod-to-mod API. Rotation
   could only be *consumed* by reading a neighbour's published
   `Speed/Capacity/Stress/NetworkStatus` NBT (`native_with_mod_block_entity`,
   API 18) and this slice does not need it.
2. ~~**Single-block accumulator.**~~ **Closed in 0.2.2**: the multiblock (up to 5 high × 3 wide,
   `getTotalAccumulatorSize()`) forms, one pool behind every block, split on a broken block — see
   `notes.md` `## Slice 2`.
3. ~~**`small_light_connector`** (1 FE/t consumption, festive lamp) is a gap.~~ **Closed in
   0.2.5**: the lamp, its 1 FE/t network draw, its position-seeded hysteresis (period
   `10 + (|x*31+y*45+z*33| % 7) * 3`, clamped both ways) and its `powered`-gated block light 15 —
   `notes.md` `## Slice 5`.
4. **Node choice from the click point is not reproduced**: the pin's
   `use_item_on` gives the block face, not the click location, so a link takes
   the lowest free node index (upstream: `getAvailableNode(clickLocation)`).
   The node *count* (4 / 6) is enforced.
5. **Wire through-put.** `WireType.transfer` is dead in 1.7.2; this port keeps
   the connector limit and the length rule only.
6. **No Create wrench.** The mode is set with `/createaddition mode` (upstream:
   wrench cycles None → Pull → Push); the placed mode is auto-detected from the
   mounted block exactly as upstream does.
7. **Network buffer contents are booked in the network's controller connector**
   (the member with the lowest position), in its saved compound's
   `netReady`/`netNext` ints, instead of in an in-memory wire graph. Upstream's
   graph is rebuilt on load and starts at 0; this port keeps the energy, which
   is strictly more conservative (no FE disappears across a reload).
8. **Recipes not carried.** Every upstream recipe for these blocks needs
   `create:` items (`andesite_alloy`, `brass_casing`) or `c:` tags that do not
   exist without the parent, so the slice carries loot tables, models,
   textures and lang only. The blocks/items are creative-obtainable.
9. **`Passive` mode** is in upstream's enum but unreachable; this port models
   `None/Pull/Push` only.
10. **The creative generator's supply is a registered handler, not a saved charge** (0.2.7).
    `Registrar::block_capability_before_game` answers `horizon:energy` with the port's
    `CreativeSupply` (amount and capacity `u32::MAX`, insert 0, extract what is asked), which is
    upstream's `CreativeEnergyStorage` on its block entity. The block declares **no**
    `block_energy`, so there is no compound amount to keep and `/createaddition info` says
    `infinite FE`; `/createaddition energy` refuses it.
11. **The generator re-reads its six neighbours every pass** (0.2.7). Upstream caches the six
    `BlockCapabilityCache`s and refreshes them on `neighborChanged`; a package's pass already
    walks the block's tick queue, so it asks `LevelRef::capability` per side each tick and there
    is no cache to invalidate. The offer is the same `Integer.MAX_VALUE`, and the neighbour's
    own insert caps it (1,000 FE into a connector's window, 5,000 into an accumulator).
12. **Two generated textures for the creative generator** (0.2.7). The jar's model names
    Create's crate and tank art, which is not in this artefact; `carry_content.py` draws two
    deterministic 16×16 PNGs instead, so the block is not the missing-texture checker and no
    other mod's assets travel in the package.
