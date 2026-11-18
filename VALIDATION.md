# Validation — Create Crafts & Additions slice 1

## 0.2.0 — the SDK package (mod API 0.2)

Pin: SDK nightly `0b14f8acfd97fa2c750e42795249dcc981178ca4` (`package_format` 6, `content_api`
"0.2"), mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`. There is **no staging step** in this
format: the SDK crate is compiled in place against `.horizon/sdk/`. Every step still ran inside one
`ports/tools/build-slot.sh` lease (`CARGO_INCREMENTAL=0`, no `--target-dir`), locally
(`BUILD_LOCAL=1`: the second machine's cargo cache lacks the SDK's crates — see `notes.md` trap 2).

Logs quoted here live in `ports/_shared/evidence/createaddition/` (`sdk-*.log`).

```
cp + cmp src/runtime.rs            byte-identical, sha256 03721ca4…f962
cargo test --offline               30 passed, 0 failed     (skeleton: `src/lib.rs` = `pub mod runtime;`,
                                                            before any glue existed)
cargo test --offline               30 passed, 10 failed    (glue stubs red on todo!(), exit 101)
cargo test --offline               41 passed, 0 failed
cargo clippy --all-targets --offline -- -D warnings         exit 0, 0 warnings
cargo fmt --check                   clean
rm -f Cargo.lock && cargo horizon-mod build                 exit 0 -> target/horizon-mod/createaddition.hmod
cargo horizon-mod build (again)     byte-identical (cmp against the first build)
cargo horizon-mod check createaddition.hmod                 exit 0
cargo horizon-mod fits createaddition.hmod --build nightly  "0b14f8a  made for", exit 0
python3 tools/carry_content.py --check                      "assets/ and data/ match the jar carry", exit 0
```

Deliverable: `createaddition.hmod`, **119,430 B**, SHA-256
`28aee5dcc361ed04dddbdb4a7a5743165f47fe9b7fe4761124222fe94afa1c8b`, 78 entries, reproducible.
`unzip -p createaddition.hmod src/runtime.rs | sha256sum` equals `src/runtime.rs`'s, and the package
holds no `Cargo.lock`.

```
createaddition 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  119430 bytes, SHA-256 28aee5dcc361ed04dddbdb4a7a5743165f47fe9b7fe4761124222fe94afa1c8b, unsigned
  native source: unsandboxed; not executed by the registry check. Compile and test against the exact pinned Horizon and Mixins revisions before review.
```

```
Create Crafts & Additions:
0b14f8a  made for
```

**The in-game leg has not been run against this package.** `horizon-launcher` is not installed, so
no `0b14f8a` game exists here: `cargo horizon-mod build` packs, and the native compile is
`horizon-launcher native-build … --trust-native-code`. `tools/e2e.sh` is updated for a
launcher-built game (`GAME=`, optional `CONTROL_GAME=`, `SELFTEST=1` kept, assertions unchanged) and
is a recorded gap in `notes.md` rather than a claim.

## 0.1.0 — the retired `.hnmod` (historical evidence)

Pin: Horizon `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9` (432 hooks / 174
helpers), horizon-mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57` (schema 6).
Builds/tests/e2e all ran through `ports/tools/build-slot.sh` (one lease per
ladder step; `CARGO_INCREMENTAL=0`; no `--target-dir`).

> The format this section describes is **retired** (mod-API merge #16941): the `.hnmod` no longer
> loads on launcher ≥ 0.2.70, `tools/horizon.py`/`tools/package.py` are gone, and the files are
> kept at `ports/_shared/evidence/createaddition/retired-hnmod-0.1.0/`. The runs themselves were
> real and stay as the behaviour's in-game record, including the defect the conservation
> assertion caught.

## H.1 The port's own crate (no game)

```
cargo fmt --check                                  exit 0   (rustfmt.toml: tab_spaces = 2)
cargo clippy --all-targets -- -D warnings          exit 0   (0 warnings)
cargo test --offline                               30 passed, 0 failed
```

`red-leg.log` — the mutation check for the conservation test:

```
== red leg: the pool swap that drops the undrained half ==
test tests::energy_nobody_took_this_tick_stays_in_the_pool ... FAILED
test result: FAILED. 29 passed; 1 failed; …
== green leg: the jar-correct swap ==
test tests::energy_nobody_took_this_tick_stays_in_the_pool ... ok
test result: ok. 30 passed; 0 failed; …
```

The bug was real: the first in-game run lost 60 FE at the buffer swap, and the
conservation assertion caught it.

## H.2 The staged game (`horizon.py`)

```
stage   exit 0
check   exit 0   cargo_exit 0, 0 warnings (target/horizon-check.log)
test    exit 0   cargo_exit 0, game::createaddition::tests
                 test result: ok. 30 passed; 0 failed; 1240 filtered out
build   exit 0   cargo_exit 0, 0 warnings (target/horizon-build.log),
                 binary target/horizon-cargo/fast/horizon-client
```

(Reports: `stage.json`, `check.json`, `test.json`; cargo's own log was kept in
the port's `target/horizon-*.log` while the ladder ran.)

## H.3 In-game check (`tools/e2e.sh`, 0.1.0)

Three legs, each ending in `EXIT:<status>`. `check-e2e-logs.py createaddition
--strict`: **logs=3 ok=3 missing-exit=0 bad-exit=0**.

### default leg — `e2e-default.log` — `PASS` + `EXIT:0`

```
== A: the four blocks, their buffers and the placement mode ==
  Connector at 191,100,0: mode pull, buffer 0/1000 FE, links [], pool 0/0, demand 0, t=178
  Modular Accumulator at 190,100,0: 0/2000000 FE (t=185)
== B: 100,000 FE crosses the wire at 1,000 FE a tick ==
  mode 195 push: Mode set to push.
  link 191-195 (gold): Nodes linked.
  Modular Accumulator at 190,100,0: 96960/2000000 FE (t=228)
```

… and the rest of the run as recorded: source 100,000 → 96,960, sink 0 → 3,000 with the network
holding the difference, total conserved at 100,000; the unlinked control moves nothing; a 17-block
wire is refused; copper is refused between two large connectors and gold accepted; the accumulator
fills the 1,000 FE window of the block above it, both empty staying empty first.

### `EMBEDDED=1` leg — `e2e-embedded.log` — `PASS` + `EXIT:0`

No `--dev-mod`: the bundle wrote its embedded content to `.horizon-cache/`, byte-identical to
`content/createaddition.hmod`, and a control run with the cache made unusable had neither the item
nor the block. **This leg has no 0.2.0 equivalent**: one package carries the content, so there is
nothing to embed (a `CONTROL_GAME=` leg replaces it in the new script).

### `SELFTEST=1` leg — `e2e-selftest.log` — `FAIL` + `EXIT:1`

The flipped conservation assertion trips, so the scenario can fail.
