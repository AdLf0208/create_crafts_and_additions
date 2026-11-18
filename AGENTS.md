# Working on this Horizon native mod

This is a **flat SDK package** (mod API 0.2): `horizon-mod.toml` + `Cargo.toml` + `src/lib.rs`
(`impl mod_api::Hooks`) + `src/runtime.rs` (the pure rules and their tests) + `assets/` + `data/`.
Horizon retired the `.hnmod` container in the mod-API merge (#16941), so there is no
`compatibility.json`, `mixins/<id>/`, `horizon-mixin.toml`, plan/injection schema or
`rust-toolchain.toml` here, and none of them should be recreated.

> **Build slots (v2):** every build/test/pack goes through the campaign runner
> (`ports/tools/build-slot.sh` from the campaign root, `../tools/build-slot.sh` from this port). It
> **blocks in the kernel** until a slot is free, so **never poll and never background a build**; a
> timeout exits **75**. It does **not** `cd` into the port — run the runner **from inside the port**
> with the bare command (`bash -c '…'`), never `--target-dir`. A `tee` in a pipe hides cargo's exit
> status: use `bash -o pipefail -c`.

```sh
cd ports/createaddition
../tools/build-slot.sh run --job createaddition-sdk --port ports/createaddition --step test \
    -- bash -o pipefail -c 'CARGO_INCREMENTAL=0 cargo test --offline 2>&1 | tee ../_shared/evidence/createaddition/sdk-test.log'
../tools/build-slot.sh run --job createaddition-sdk --port ports/createaddition --step build \
    -- bash -o pipefail -c 'rm -f Cargo.lock && cargo horizon-mod build'
../tools/build-slot.sh run --job createaddition-sdk --port ports/createaddition --step check \
    -- bash -o pipefail -c 'cargo horizon-mod check createaddition.hmod'
```

- `cargo horizon-mod` uses the pinned SDK in `.horizon/sdk/` (gitignored, never packed). `build`
  packs the `.hmod`; it does **not** compile against the game — that is
  `horizon-launcher native-build . --instance <instance> --trust-native-code`, which is not
  installed here. `fits` only answers "made for".
- **`src/runtime.rs` is byte-identical to the retired bundle's `runtime.rs` and must stay so**
  (`cmp` it, and check the packaged entry with `unzip -p createaddition.hmod src/runtime.rs |
  sha256sum`). New rules go in `src/network.rs` **with a test**, never in `runtime.rs`.
- `src/rustfmt.toml` holds the campaign style (`tab_spaces = 2`). It lives in `src/` because the
  packer refuses a root `rustfmt.toml`; without it `cargo fmt --check` fails on `runtime.rs` and
  `cargo fmt` would rewrite it. Use `cargo fmt --check`, never `--all` (that formats the fetched
  SDK).
- The energy moves go through the engine's `ENERGY` capability inside `host.transaction(…)`, a
  block's charge is the compound's `energy` int (`ModEnergyBuffer`'s own key), and the per-level
  pass is indexed by the positions `block_entity_tick` queues — a package cannot enumerate a level's
  block entities, and a block entity is out of the world during its own tick.
- Only the API the SDK publishes: `cargo horizon-mod hooks <text>` finds the exact method; never
  guess a name, and record an API gap in `notes.md` instead of faking one.
- The delivered package is `createaddition.hmod` **in the port root**, never in `target/`;
  `LICENSE`, `LICENSE-NOTICE.md` and `README.md` travel inside it, and `rm -f Cargo.lock` before a
  build (a packed lock changes the artifact).
- Keep the port separate from Horizon and horizon-mixins; never modify the pin checkouts, never
  commit binaries, exported sources or Minecraft assets, and never put Mojang files in the tree.
- `python3 ports/tools/check-skills.py --notes ports/createaddition` must stay green, and `notes.md`
  records every claim with its command and output.
