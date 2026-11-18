# Create Crafts & Additions — a Horizon port

**Create Crafts & Additions** is a Minecraft mod. Create Crafts & Additions, Addon to the Create mod. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [mrh0/createaddition](https://github.com/mrh0/createaddition) — licence **MIT**.

| | |
|---|---|
| Upstream mod | Create Crafts & Additions ([source](https://github.com/mrh0/createaddition)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `createaddition.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

- **`createaddition:connector`** (small) and **`createaddition:large_connector`**
- **Placement mode**, upstream's `ConnectorMode.test` on the first pass after it
- **Wire links**, upstream's `WireSpool.useOn` rules: a spool's first right
- **The network pass**, after the level's tick: every connector that ticked is
- **`createaddition:modular_accumulator`** — 2,000,000 FE, 5,000 FE/t in and
- **Commands** — the harness's and an admin's interface, because the slice has

## Not yet ported

- **The in-game e2e has not been run against this package.** `cargo horizon-mod build` packs;
- **The rotation half**: `electric_motor`, `alternator`, `rolling_mill`,
- **`small_light_connector`** (its 1 FE/t lamp consumption).
- **`creative_energy`**, the e2e's source of last resort: the harness seeds an
- **`portable_energy_interface`**, **`redstone_relay`**, **`energy_meter`**,
- **GUI/tooltips**: no goggles-style readout, no mode wrench, no comparator
- **Crafting**: no recipes (`CONTENT.md`) — the blocks are creative-only until
- **Festive wire and spool**, the iron wire, and the electrum/brass/copper metal

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `createaddition.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Create Crafts & Additions was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/mrh0/createaddition

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run createaddition` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/createaddition/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
