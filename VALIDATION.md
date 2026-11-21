# Validation — Create Crafts & Additions slices 1–3

## 0.2.3 — the three review fixes (PR #1526)

Pin: Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` (nightly 2026-11-20), mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`; every cargo step inside one
`ports/tools/build-slot.sh` lease (`CARGO_INCREMENTAL=0`, no `--target-dir`, `BUILD_LOCAL=1`).
The in-game legs use the launcher-built game (`ports/tools/launcher-game.sh`, launcher 0.2.92);
the script now honours `E2E_TMP` instead of hardcoding `/tmp`.

Logs: `ports/_shared/evidence/createaddition/` (`tower-red.log`,
`doctored-controller-red.log`, `licence-notice*.log`, `sdk-*.log`,
`verify-sdk-package.log`, `probe-doctored.log`) and
`ports/_shared/evidence/createaddition-e2e/` (`e2e-review-*.log`).

```
red, quadratic remainder scan   left: 8256, right: 4160          (tower-red.log)
red, doctored saved 4x1          Tower { width: 4, height: 1 }    (tower-red.log)
red, controller at i32::MAX      attempt to add with overflow     (doctored-controller-red.log)
red, upstream notice absent      no LICENSE-UPSTREAM in the hmod  (licence-notice-red.log)
cargo test --offline             56 passed, 0 failed              (sdk-test.log)
cargo clippy --all-targets --offline -- -D warnings   exit 0, 0 warnings (sdk-clippy.log)
cargo fmt --check                clean                            (sdk-fmt.log)
rm -f Cargo.lock && cargo horizon-mod build           exit 0       (sdk-build.log)
cargo horizon-mod build (again)  byte-identical (cmp)             (sdk-build-second.log)
cargo horizon-mod check createaddition.hmod           exit 0       (sdk-check.log)
fits --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a  "0a875da  made for" (sdk-fits-pin.log)
fits --build 0b14f8acfd97fa2c750e42795249dcc981178ca4  "0b14f8a  fits"     (sdk-fits-other.log)
python3 tools/check_licence.py    upstream notice verbatim        (licence-notice.log)
python3 ports/tools/verify-sdk-package.py createaddition   1/1     (verify-sdk-package.log)
```

Deliverable: `createaddition.hmod`, **133,587 B**, SHA-256
`1b29b731068bc6a2f21281fc88f60b7643b39f819a4a2f8885d0ad0818fe53d6`, **83 entries**, reproducible
(two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f962`, packed too).

```
createaddition 0.2.3 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  133587 bytes, SHA-256 1b29b731068bc6a2f21281fc88f60b7643b39f819a4a2f8885d0ad0818fe53d6, unsigned
```

```
Create Crafts & Additions:
0a875da  made for
Create Crafts & Additions:
0b14f8a  fits
```

| log | verdict | what it proves |
|---|---|---|
| `e2e-review-default.log` | `PASS` + `EXIT:0` | A–H as 0.2.2, plus **G6**: a doctored `tw:4,th:1` and `tw:1000,th:1000` both read `123/2000000`, and the next pass re-forms the 1×1 |
| `e2e-review-selftest.log` | `FAIL` + `EXIT:1` | the flipped conservation assertion trips |
| `e2e-review-control.log` | `FAIL` + `EXIT:1` | the same item assertion on the package-less control game |
| `probe-doctored.log` | `EXIT:0` | the saved compound is the block-entity NBT root (`/data merge block` works) and `/tick freeze` holds the pass off |

`python3 ports/tools/check-e2e-logs.py createaddition createaddition-e2e --strict`:
**logs=12 ok=12 missing-exit=0 bad-exit=0**.

## 0.2.2 — the accumulator multiblock, and the carried data's dialect

Pin: Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` (nightly 2026-11-20), mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, SDK cached in `.horizon/sdk/` from
`ports/_shared/sdk/0a875da`. Every cargo step ran inside one `ports/tools/build-slot.sh` lease
(`CARGO_INCREMENTAL=0`, no `--target-dir`); `stage`/`check`/`build` locally (`BUILD_LOCAL=1`: the
second machine has no `cargo-horizon-mod`), `test`/`clippy`/`fmt` on the slot runner. The in-game
leg is a **launcher-built** game (`ports/tools/launcher-game.sh build createaddition`, launcher
0.2.92), the `e2e` step through the runner.

Logs quoted here live in `ports/_shared/evidence/createaddition/` (`tower-*.log`) and
`ports/_shared/evidence/createaddition-e2e/` (`e2e-*.log`).

```
red (the tower rules as stubs)     46 passed, 6 failed  (red-tower.log)
cargo test --offline               53 passed, 0 failed  (sdk-test.log)
cargo clippy --all-targets --offline -- -D warnings   exit 0, 0 warnings (tower-clippy.log)
cargo fmt --check                  clean              (tower-fmt.log)
rm -f Cargo.lock && cargo horizon-mod build           exit 0 -> target/horizon-mod/createaddition.hmod (tower-build-1.log)
cargo horizon-mod build (again)    byte-identical (cmp) (tower-build-2.log)
cargo horizon-mod check createaddition.hmod           exit 0 (tower-check.log)
cargo horizon-mod fits createaddition.hmod --build 0a875da   "0a875da  made for" (tower-fits-pin.log)
cargo horizon-mod fits createaddition.hmod --build 0b14f8a   "0b14f8a  fits"      (tower-fits-0b14f8a.log)
python3 tools/data_shape.py createaddition.hmod       red: 9 problems (loot-red.log) -> green (loot-green.log)
python3 tools/carry_content.py --check                "assets/ and data/ match the jar carry"
python3 ports/tools/verify-sdk-package.py createaddition   1/1 (tests 53, clippy 0, check, fits)
```

Deliverable: `createaddition.hmod`, **129,771 B**, SHA-256
`3f64f672d86bf90da7d43643399e565d4851b280f43c8b4c3923d62f88dc137c`, **82 entries**, reproducible
(two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f962`, packed too).

```
createaddition 0.2.2 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  129771 bytes, SHA-256 3f64f672d86bf90da7d43643399e565d4851b280f43c8b4c3923d62f88dc137c, unsigned
```

```
Create Crafts & Additions:
0a875da  made for
Create Crafts & Additions:
0b14f8a  fits
```

### In-game legs (launcher-built game, `horizon-ctl`)

| log | verdict | what it proves |
|---|---|---|
| `e2e-tower.log` | `PASS` + `EXIT:0` | A–F (the 0.2.1 scenario, unchanged) + **G** the 3-high tower forms (`tower 1x3 at 220,100,0`, `0/6000000`), **G2** a 5,000,000 seed into the top reads from every part, **G3** breaking the top re-forms at `4000000/4000000`, **G4** the tower's pool serves a wire and FE is conserved at 4,000,000, **G5** three separated blocks stay singles at 2,000,000, **H** each of the three blocks breaks and drops 1 |
| `e2e-tower-selftest.log` | `FAIL` + `EXIT:1` | the flipped conservation assertion trips |
| `e2e-tower-control.log` | `FAIL` + `EXIT:1` | the same package assertion on a game **without** the package: no such item |
| `e2e-loot-prefix.log` | `FAIL` + `EXIT:1` | the **pre-fix** package (1.21.1 loot tables): a real break drops 0 |
| `e2e-launcher.log`, `e2e-launcher-control.log` | `EXIT:0` / `EXIT:1` | the 0.2.1 run at `0b14f8a` (historical) |

`python3 ports/tools/check-e2e-logs.py createaddition createaddition-e2e --strict`:
**logs=9 ok=9 missing-exit=0 bad-exit=0**.

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
