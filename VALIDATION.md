# Validation — Create Crafts & Additions slices 1–8

## 0.2.7 — the creative generator

Pin: Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` (nightly 2026-11-20), mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`; every cargo step inside one
`ports/tools/build-slot.sh` lease (`CARGO_INCREMENTAL=0`, no `--target-dir`, `BUILD_LOCAL=1`).
The in-game legs use the launcher-built game (`ports/tools/launcher-game.sh`, launcher 0.2.92,
`Compiling createaddition (1 of 1)`); the creative-generator build is
`native-0a875da-a4f96b049501`.

Logs: `ports/_shared/evidence/createaddition/` (`creative-red.log`, `creative-build-*.log`,
`creative-check.log`, `creative-fits-pin.log`, `creative-fits-other.log`, `creative-launcher-build.log`,
`sdk-*.log`, `verify-sdk-package.log`) and `ports/_shared/evidence/createaddition-e2e/`
(`e2e-creative-*.log`).

```
red, pre-slice generator rules   3 failed: 77 passed  (creative-red.log)
  stored/capacity                left: 0   right: 4294967295
  the wire rig's 20 ticks        left: [0; 20]   right: [1000; 20]
  the per-tick offer             left: 0   right: 4294967295
cargo test --offline             80 passed, 0 failed              (sdk-test.log, regenerated)
cargo clippy --all-targets --offline -- -D warnings   exit 0, 0 warnings (sdk-clippy.log)
cargo fmt --check                clean                            (sdk-fmt.log)
rm -f Cargo.lock && cargo horizon-mod build           exit 0       (creative-build-1.log)
cargo horizon-mod build (again)  byte-identical (cmp)             (creative-build-2/3.log)
cargo horizon-mod check createaddition.hmod           exit 0       (creative-check.log)
fits --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a  "0a875da  made for" (creative-fits-pin.log)
fits --build 0b14f8acfd97fa2c750e42795249dcc981178ca4  "does not fit: 1 item changed
                                                       the method `BlockStates::light_emission` (removed)"
                                                                    (creative-fits-other.log)
python3 tools/data_shape.py createaddition.hmod   6/6 26.3 loot tables
python3 tools/carry_content.py --check    assets/ and data/ match the jar carry
python3 tools/check_licence.py    upstream notice verbatim       (licence-notice.log)
python3 ports/tools/verify-sdk-package.py createaddition   1/1     (verify-sdk-package.log)
```

Deliverable: `createaddition.hmod`, **178,357 B**, SHA-256
`8387f865ed975963a9b7bd5b1d1d602434977c43a288dc0399e4cdbf962ad9b6`, **117 entries**, reproducible
(three builds, two `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f962`, packed too); 86 `assets/` + 20 `data/`.

```
createaddition 0.2.7 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  178357 bytes, SHA-256 8387f865ed975963a9b7bd5b1d1d602434977c43a288dc0399e4cdbf962ad9b6, unsigned
```

```
Create Crafts & Additions:
0a875da  made for
Create Crafts & Additions:
0b14f8a  does not fit: 1 item changed
    the method `BlockStates::light_emission` (removed)
```

**The second `fits` run is red and that is the measured finding**: the lamp's `light_emission`
surface (0.2.5) does not exist at `0b14f8a`. This slice adds no new API surface.

| log | verdict | what it proves |
|---|---|---|
| `e2e-creative-default.log` | `PASS` + `EXIT:0` | A–K unchanged, plus **L**: `createaddition:creative_energy` placed and read back, its readout `infinite FE`, a connector settled `pull` (state + readout), the sink gaining **exactly 1,000 FE/t** (30,000 over 30 ticks; 20,000 in the 20-tick window), **stone in its place moving nothing** over 31 ticks, and a generator beside an accumulator filling it at 5,000/t with no connector; H now drops one of all six blocks |
| `e2e-creative-selftest.log` | `FAIL` + `EXIT:1` | `CREATIVE_SELFTEST: the creative generator supplied 31000 FE, so the assertion is alive` |
| `e2e-creative-suite-selftest.log` | `FAIL` + `EXIT:1` | the suite's conservation counterfactual (`SELFTEST`) |
| `e2e-creative-control.log` | `FAIL` + `EXIT:1` | the package-less game has no accumulator item (`ctl give` exits 1); the default run's control leg also refuses the generator id |

`python3 ports/tools/check-e2e-logs.py createaddition createaddition-e2e --strict`:
**logs=29 ok=29 missing-exit=0 bad-exit=0**.

## The energy meter: blocked (not in the artefact) — the brief before this package

**No package was built and no version was bumped for it.** That brief's slice, `energy_meter`, is not in
the audited jar `createaddition-1.7.2.jar` (`71e9c3ac…3414`, 1,772,844 B): 0 entries under
`blocks/energy_meter`, no blockstate/model/item/loot/lang key, and only an orphan
`textures/item/multimeter.png`. On upstream `1.21.1` (`658a390`) all three `energy_meter` files are
commented out and nothing registers `EnergyMeter`. The transcript is
`ports/_shared/evidence/createaddition/energy-meter-artefact.log`; the decision is
`notes.md` → `## Slice 7 — the energy meter: blocked, not in the artefact`. The slice it proposed
instead is the 0.2.7 creative generator, above.

## 0.2.6 — the accumulator's comparator output

Pin: Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` (nightly 2026-11-20), mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`; every cargo step inside one
`ports/tools/build-slot.sh` lease (`CARGO_INCREMENTAL=0`, no `--target-dir`, `BUILD_LOCAL=1`).
The in-game legs use the launcher-built game (`ports/tools/launcher-game.sh`, launcher 0.2.92,
`Compiling createaddition (1 of 1)`); the comparator build is `native-0a875da-fa83417f8a7d`.

Logs: `ports/_shared/evidence/createaddition/` (`comparator-red.log`, `comparator-build-*.log`,
`comparator-check.log`, `comparator-fits-pin.log`, `comparator-fits-other.log`, `sdk-*.log`,
`verify-sdk-package.log`) and `ports/_shared/evidence/createaddition-e2e/`
(`e2e-comparator-*.log`).

```
red, pre-slice comparator rules 3 failed: 72 passed  (comparator-red.log)
  half full                     left: 15   right: 8
  a tower's own fraction        left: 15   right: 8
  a first answer is a change    "the first answer is a change, so the comparator is told"
red, f32 vs double fraction     left: 3    right: 4      (comparator-fraction-red.log,
                                                           taken by reverting the fix)
cargo test --offline             76 passed, 0 failed              (sdk-test.log, regenerated)
cargo clippy --all-targets --offline -- -D warnings   exit 0, 0 warnings (sdk-clippy.log)
cargo fmt --check                clean                            (sdk-fmt.log)
rm -f Cargo.lock && cargo horizon-mod build           exit 0       (comparator-build-1.log)
cargo horizon-mod build (again)  byte-identical (cmp)             (comparator-build-2.log)
cargo horizon-mod check createaddition.hmod           exit 0       (comparator-check.log)
fits --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a  "0a875da  made for" (comparator-fits-pin.log)
fits --build 0b14f8acfd97fa2c750e42795249dcc981178ca4  "does not fit: 1 item changed
                                                       the method `BlockStates::light_emission` (removed)"
                                                                    (comparator-fits-other.log)
python3 tools/data_shape.py createaddition.hmod   5/5 26.3 loot tables
python3 tools/carry_content.py --check    assets/ and data/ match the jar carry
python3 tools/check_licence.py    upstream notice verbatim       (licence-notice.log)
python3 ports/tools/verify-sdk-package.py createaddition   1/1     (verify-sdk-package.log)
```

Deliverable: `createaddition.hmod`, **172,461 B**, SHA-256
`9daebe6da2536041f42f76e2b7f169ee2d243ae60fbbc0312f719508b6e756f4`, **108 entries**, reproducible
(two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f962`, packed too).

```
createaddition 0.2.6 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  172461 bytes, SHA-256 9daebe6da2536041f42f76e2b7f169ee2d243ae60fbbc0312f719508b6e756f4, unsigned
```

```
Create Crafts & Additions:
0a875da  made for
Create Crafts & Additions:
0b14f8a  does not fit: 1 item changed
    the method `BlockStates::light_emission` (removed)
```

**The second `fits` run is red and that is the measured finding**: the lamp's `light_emission`
surface does not exist at `0b14f8a` or `6d765e0`. This slice adds no new API surface (the comparator
hooks and `NeighbourUpdate::OutputSignal` are at the pin), but the package still carries the lamp.

| log | verdict | what it proves |
|---|---|---|
| `e2e-comparator-default.log` | `PASS` + `EXIT:0` | A–J unchanged, plus **K**: a comparator on a formed 1×3 tower drives its own wire to `power=0` (empty), `power=8` (3,000,000 of 6,000,000 — the jar's fraction midpoint) and `power=15` (full); the same rig behind stone reads `power=0` while the tower reads 15; emptying the tower with `/createaddition energy` returns the wire to 0 |
| `e2e-comparator-selftest.log` | `FAIL` + `EXIT:1` | `COMPARATOR_SELFTEST: emptying the tower moved the wire from power=15 to power=0, so the assertion is alive` |
| `e2e-comparator-suite-selftest.log` | `FAIL` + `EXIT:1` | the suite's conservation counterfactual (`SELFTEST`) |
| `e2e-comparator-control.log` | `FAIL` + `EXIT:1` | the package-less game has no accumulator item |
| `e2e-comparator-probe.log` | `PASS` + `EXIT:0` | the comparator geometry: both mirror orientations read the tower and drive the wire behind them (`CMP_ONLY=1`) |

`python3 ports/tools/check-e2e-logs.py createaddition createaddition-e2e --strict`:
**logs=25 ok=25 missing-exit=0 bad-exit=0**.

## 0.2.5 — the small light connector

Pin: Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` (nightly 2026-11-20), mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`; every cargo step inside one
`ports/tools/build-slot.sh` lease (`CARGO_INCREMENTAL=0`, no `--target-dir`, `BUILD_LOCAL=1`).
The in-game legs use the launcher-built game (`ports/tools/launcher-game.sh`, launcher 0.2.92);
the lamp build is `native-0a875da-9e6e38b74abd`.

Logs: `ports/_shared/evidence/createaddition/` (`lamp-red.log`, `lamp-build-*.log`,
`lamp-check.log`, `lamp-fits-pin.log`, `lamp-fits-other.log`, `sdk-*.log`,
`verify-sdk-package.log`) and `ports/_shared/evidence/createaddition-e2e/`
(`e2e-lamp-*.log`).

```
red, pre-slice lamp rules      4 failed: 68 passed  (lamp-red.log)
  the period                  left: 10   right: 19
  the counter lights          left: (0, None)   right: (1, None)
  a lamp that never got FE    left: (0, None)   right: (-1, None)
  no network, no draw         left: (0, None)   right: (-1, None)
cargo test --offline             72 passed, 0 failed              (sdk-test.log)
cargo clippy --all-targets --offline -- -D warnings   exit 0, 0 warnings (sdk-clippy.log)
cargo fmt --check                clean                            (sdk-fmt.log)
rm -f Cargo.lock && cargo horizon-mod build           exit 0       (lamp-build-1.log)
cargo horizon-mod build (again)  byte-identical (cmp)             (lamp-build-2.log)
cargo horizon-mod check createaddition.hmod           exit 0       (lamp-check.log)
fits --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a  "0a875da  made for" (lamp-fits-pin.log)
fits --build 0b14f8acfd97fa2c750e42795249dcc981178ca4  "does not fit: 1 item changed
                                                       the method `BlockStates::light_emission` (removed)"
                                                                   (lamp-fits-other.log)
python3 tools/data_shape.py createaddition.hmod   5/5 26.3 loot tables
python3 tools/carry_content.py --check    assets/ and data/ match the jar carry
python3 tools/check_licence.py    upstream notice verbatim       (licence-notice.log)
python3 ports/tools/verify-sdk-package.py createaddition   1/1     (verify-sdk-package.log)
```

Deliverable: `createaddition.hmod`, **168,353 B**, SHA-256
`854e9d6ac9a8b9af0501310053c8e9c833215694663959501447a15199372b41`, **108 entries**, reproducible
(two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f962`, packed too), **79** `assets/` + **18** `data/` files.

```
createaddition 0.2.5 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  168353 bytes, SHA-256 854e9d6ac9a8b9af0501310053c8e9c833215694663959501447a15199372b41, unsigned
```

```
Create Crafts & Additions:
0a875da  made for
Create Crafts & Additions:
0b14f8a  does not fit: 1 item changed
    the method `BlockStates::light_emission` (removed)
```

**The second `fits` run is red and that is the measured finding**: the lamp's `light_emission` surface
(`dynamic_light`, the `BlockPart` hook and the `BlockStates::light_emission` service method) does not
exist at `0b14f8a` or at `6d765e0` — it is new at the `0a875da` pin. The pin run is therefore not a
tautology (the older builds show the check discriminates), and no build before the pin can carry this
family.

| log | verdict | what it proves |
|---|---|---|
| `e2e-lamp-default.log` | `PASS` + `EXIT:0` | A–I unchanged, plus **J**: the lamp is dark at 0 FE (`powered=false`, `ctl light` 0, counter `-25/25`); after seeding 5,000 FE and linking it is lit (`powered=true`, `ctl light` f, counter `25/25`); it draws exactly 1 FE/t (`source 4903->4861 over 42 ticks`, pool steady at 26); unlinked it goes dark and the source stays at 4,852 |
| `e2e-lamp-selftest.log` | `FAIL` + `EXIT:1` | `SELFTEST: the conservation assertion did not trip` (B's counterfactual) |
| `e2e-lamp-gate-selftest.log` | `FAIL` + `EXIT:1` | `LAMP_SELFTEST: the unlinked lamp drew nothing, so the assertion is alive` (J's own counterfactual) |
| `e2e-lamp-control.log` | `FAIL` + `EXIT:1` | the package-less game has no accumulator item and no lamp/relay block id |

`python3 ports/tools/check-e2e-logs.py createaddition createaddition-e2e --strict`:
**logs=20 ok=20 missing-exit=0 bad-exit=0**.

## 0.2.4 — the redstone relay

Pin: Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` (nightly 2026-11-20), mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`; every cargo step inside one
`ports/tools/build-slot.sh` lease (`CARGO_INCREMENTAL=0`, no `--target-dir`, `BUILD_LOCAL=1`).
The in-game legs use the launcher-built game (`ports/tools/launcher-game.sh`, launcher 0.2.92);
the relay build is `native-0a875da-39565764a7f0`.

Logs: `ports/_shared/evidence/createaddition/` (`relay-red.log`, `probe-relay.log`, `sdk-*.log`,
`verify-sdk-package.log`) and `ports/_shared/evidence/createaddition-e2e/` (`e2e-relay-*.log`).

```
red, pre-slice relay rules     5 failed: 61 passed  (relay-red.log)
  port choice                  left: Input  right: Output
  vertical port choice         left: Input  right: Output
  unpowered bridge             left: RelayMove { into: 200, out: 1000 }  right: RelayMove { into: 0, out: 0 }
  two networks                 left: one group  right: [[input],[output]]
  rig, unpowered               left: 18000  right: 0
cargo test --offline             66 passed, 0 failed              (sdk-test.log)
cargo clippy --all-targets --offline -- -D warnings   exit 0, 0 warnings (sdk-clippy.log)
cargo fmt --check                clean                            (sdk-fmt.log)
rm -f Cargo.lock && cargo horizon-mod build           exit 0       (sdk-build.log)
cargo horizon-mod build (again)  byte-identical (cmp)             (sdk-build-second.log)
cargo horizon-mod check createaddition.hmod           exit 0       (sdk-check.log)
fits --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a  "0a875da  made for" (sdk-fits-pin.log)
fits --build 0b14f8acfd97fa2c750e42795249dcc981178ca4  "0b14f8a  fits"     (sdk-fits-other.log)
python3 tools/check_licence.py    upstream notice verbatim        (licence-notice.log)
python3 tools/data_shape.py createaddition.hmod   4/4 26.3 loot tables
python3 tools/carry_content.py --check    assets/ and data/ match the jar carry
python3 ports/tools/verify-sdk-package.py createaddition   1/1     (verify-sdk-package.log)
```

Deliverable: `createaddition.hmod`, **150,070 B**, SHA-256
`8b5b1c2250b1d884d0b35fdc90a1cff5d46e3de7354dd95640ed34572783eacd`, **94 entries**, reproducible
(two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f962`, packed too), **83** carried `assets/`+`data/` files.

```
createaddition 0.2.4 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  150070 bytes, SHA-256 8b5b1c2250b1d884d0b35fdc90a1cff5d46e3de7354dd95640ed34572783eacd, unsigned
```

```
Create Crafts & Additions:
0a875da  made for
Create Crafts & Additions:
0b14f8a  fits
```

| log | verdict | what it proves |
|---|---|---|
| `e2e-relay-default.log` | `PASS` + `EXIT:0` | A–I: the relay's unpowered 20 ticks move the sink not at all, `powered=true` from a redstone block makes it gain 24,020 FE from a 100,000 FE source with source + sink + both pools + the relay's buffer conserved, and all **four** loot tables drop one of themselves |
| `e2e-relay-selftest.log` | `FAIL` + `EXIT:1` | the suite's own counterfactual (`SELFTEST: the conservation assertion did not trip`) |
| `e2e-relay-gate-selftest.log` | `FAIL` + `EXIT:1` | the relay gate assertion itself (`RELAY_SELFTEST: the unpowered relay moved the sink`) |
| `e2e-relay-control.log` | `FAIL` + `EXIT:1` | the package-less game knows neither the block nor the relay id |
| `probe-relay.log` | `EXIT:0` | the probe: 99,980 → 0 through the relay across a power cycle, with the saved `netReady/netNext/netDemand`/`energy` |

`python3 ports/tools/check-e2e-logs.py createaddition createaddition-e2e --strict`:
**logs=16 ok=16 missing-exit=0 bad-exit=0**.


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
