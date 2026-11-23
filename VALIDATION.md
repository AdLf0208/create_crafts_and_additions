# Validation — Create Crafts & Additions slices 1–12

## 0.2.11 — the lifecycle clears (review fix: stale comparator entries)

Pin: Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` (nightly 2026-11-20), mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`; every cargo/package step inside one
`ports/tools/build-slot.sh` lease (`CARGO_INCREMENTAL=0`, no `--target-dir`), with `BUILD_LOCAL=1`
(`cargo horizon-mod` is installed on this box only). In-game legs: the launcher-built game
(`ports/tools/launcher-game.sh`, launcher 0.2.92).

The finding, in the reviewer's own words: *"Both match the previously reviewed archives that retain
stale comparator entries after accumulator removal, chunk unload and level close … make sure the
replacement clears the affected entries on each of those lifecycle events."* The mapping the fix
uses, measured from the pin's own hook declarations (not guessed), the red and the in-game proof are
`notes.md` § *Slice 12*.

```
red, pre-slice (no clearing at all)   99 passed, 6 failed  (lifecycle-red.log)
  a removed block's answer            "the removed block's answer was there to forget"
  a chunk unload                      left: 0  right: 2
  negative chunk coordinates          left: 0  right: 1
  a level close                       left: 0  right: 2
  a logout / a level close (Pending)  left: 2  right: 1
  the readout                         left: (0, [])  right: (3, […])
cargo test --offline                 105 passed, 0 failed             (sdk-test.log, regenerated)
cargo clippy --all-targets --offline -- -D warnings   exit 0, 0 warnings (sdk-clippy.log)
cargo fmt --check                    clean (empty)                   (sdk-fmt.log)
rm -f Cargo.lock && cargo horizon-mod build            exit 0       (lifecycle-build-1.log)
the same again (rm -f Cargo.lock)                      byte-identical (cmp)
cargo horizon-mod check createaddition.hmod            0.2.11 … 194450 bytes (lifecycle-check.log)
fits --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a  "is made for exactly one build (it has a
                                                       [native.mixin] table), so fits doesn't
                                                       apply", exit 2   (lifecycle-fits-pin.log)
fits --build 0b14f8acfd97fa2c750e42795249dcc981178ca4  the same refusal, exit 2
                                                                    (lifecycle-fits-other.log)
launcher-game.sh build createaddition   native-0a875da-05f84c083d3a, `Compiling createaddition (1 of 1)`
                                        (lifecycle-launcher-build.log); the launcher's own log:
                                        `Compiling createaddition: 8388 ms` and
                                        `Function replacement: 2 functions patched by 2 mixins,
                                        without rebuilding the game`
                                        (lifecycle-launcher-patch.log); identity
                                        `{"id":"createaddition","version":"0.2.11",…}` and
                                        `strings libcreateaddition.so` carries `Signals: answers`
python3 tools/data_shape.py createaddition.hmod          6/6 in the 26.3 dialect
python3 tools/carry_content.py --check                   assets/ and data/ match the jar carry
python3 tools/check_licence.py                           upstream notice verbatim
python3 ports/tools/check-recipe-types.py createaddition   createaddition: ok
python3 ports/tools/verify-sdk-package.py createaddition   1/1     (verify-sdk-package.log,
                                                                   incl. `package matches the tree`)
python3 ports/tools/check-e2e-logs.py createaddition-e2e --strict   logs=47 ok=47 missing-exit=0
                                                                   bad-exit=0
```

**Both `fits` lines are refusals, not a pass and a red** — unchanged since 0.2.9 (a package with a
`[[native.mixin]]` row is made for exactly one build, its helpers being compiled into a copy of that
build's own source). The replacement evidence is the launcher build (the mod recompiled and both
mixins were re-applied against `0a875da`) and the game it produced, which ran the whole lifecycle leg
and the A–M suite.

Deliverable: `createaddition.hmod`, **194,450 B**, SHA-256
`0b1b8184af405689c80154aacdc718949f701cc6466d473106e50a2c0705f6fc`, **127 entries**, reproducible
(two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f962`, packed too).

```
createaddition 0.2.11 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  194450 bytes, SHA-256 0b1b8184af405689c80154aacdc718949f701cc6466d473106e50a2c0705f6fc, unsigned
```

```
Create Crafts & Additions is made for exactly one build (it has a [native.mixin] table), so `fits` doesn't apply.
```

### In-game — the lifecycle leg, the A–M suite and the renderer

`tools/e2e-lifecycle.sh` (default / `LIFECYCLE_SELFTEST=1` / `CONTROL_GAME=`), then the whole A–M
suite and the renderer leg on this build:

| log | verdict | what it proves |
|---|---|---|
| `e2e-lifecycle-default.log` | `PASS` + `EXIT:0` | two accumulators → `answers=2`; a removal takes the broken one's (`2 → 1`) while the neighbour's wire still reads `power=8`; a re-placed block is recorded again (`1 → 2`); a chunk unload takes that chunk's (`2 → 1`) while the still-loaded accumulator reads `power=8`; going back re-records it (`1 → 2`); `/save-all` keeps both; a relaunch holds only what it computed (1 at the survivor, 2 with both chunks loaded) |
| `e2e-lifecycle-selftest.log` | `FAIL` + `EXIT:1` | `LIFECYCLE_SELFTEST: the chunk unload forgot the answer (answers=1), so the eviction assertion is alive` |
| `e2e-lifecycle-control.log` | `FAIL` + `EXIT:1` | a Nightly without the package cannot place `createaddition:modular_accumulator`, so the counts come from this package |
| `e2e-lifecycle-suite.log` | `PASS` + `EXIT:0` | A–M on 0.2.11: the relay's 25,020 FE conserved, the lamp's 1 FE/t, the comparator's 0/8/15, the generator's 1,000 FE/t, the festive wire's 1,000 FE/t, the break drops — no earlier family disturbed |
| `e2e-lifecycle-suite-selftest.log` | `FAIL` + `EXIT:1` | the suite's conservation counterfactual is still alive |
| `e2e-lifecycle-wire.log` | `PASS` + `EXIT:0` | the renderer is not regressed: **1901** green-dominant pixels with the wire (mean `rgb(21, 86, 9)`), 0 without it |
| `e2e-lifecycle-wire-negative.log` | `FAIL` + `EXIT:1` | the renderer's counterfactual (a renderer that draws nothing fails) |
| `probe-lifecycle.log`, `probe-lifecycle3.log` | `EXIT:0` | the geometry of the far rig (forceload marks chunk `[68, 0]`; `ctl block`'s `loaded:false` is the client's copy) and the relaunch (the same world returns with both accumulators and their saved `energy`) |

**The level close is unit-tested, not separately observable in game** — every chunk of a closing
level unloads first (and `chunk_unload` clears those answers) and a world close ends the process, so
the map cannot distinguish `level_unload` from the clears that preceded it. The leg's relaunch phase
covers the session's half (`notes.md` § *Slice 12*, deviations 3–4).

## 0.2.10 — the wires a broken connector drops


Pin: Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` (nightly 2026-11-20), mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`; every cargo/package step inside one
`ports/tools/build-slot.sh` lease (`CARGO_INCREMENTAL=0`, no `--target-dir`), the package steps with
`BUILD_LOCAL=1` (`cargo horizon-mod` is installed on this box only). In-game legs: the launcher-built
game `native-0a875da-4af24797bccd` (launcher 0.2.92), whose build shows
`Compiling createaddition (1 of 1)` and
`Function replacement: 2 functions patched by 2 mixins, without rebuilding the game (copy 114 ms,
horizon-mixins 303 ms, compile 3021 ms, total 3440 ms)` (`drop-launcher-patch.log`), and the launcher's identity for the mod
`0a875da91dbb… createaddition 0.2.10 1.99.1` (`drop-launcher-identity.json`).

The slice, in one line: upstream drops four wires of the wire's own kind at a broken connector
(a festive wire's four are **copper** wires), nothing in creative, and empties the peer's node
without a second drop. Rules quoted from the artefact and the route at the pin: `notes.md`
§ *Slice 11*.

Logs: `ports/_shared/evidence/createaddition/` (`drop-red.log`, `sdk-test.log`, `sdk-clippy.log`,
`sdk-fmt.log`, `drop-build-1..6.log`, `drop-check.log`, `drop-fits-pin.log`, `drop-fits-other.log`,
`drop-launcher-build.log`, `drop-launcher-patch.log`, `drop-launcher-patch-table.rs`,
`drop-launcher-identity.json`, `drop-launcher-strings.log`, `verify-sdk-package.log`) and
`ports/_shared/evidence/createaddition-e2e/` (`e2e-drop-*.log`).

```
red, pre-slice tables            94 passed, 5 failed (drop-red.log)
  one stack per kind             left [] right [(Copper, 8), (Gold, 4), (Festive, 4)]
  creative drops nothing         left [] right [(Copper, 4), (Electrum, 4)]
  the peer's link                left [gone, kept] right [kept]
  the drop tables                left "" right "createaddition:copper_wire"
  the unlinked differential      left [] right [(Copper, 4)]
cargo test --offline             99 passed, 0 failed             (sdk-test.log, regenerated)
cargo clippy --all-targets --offline -- -D warnings   exit 0, 0 warnings (sdk-clippy.log)
cargo fmt --check                clean (empty)                   (sdk-fmt.log)
rm -f Cargo.lock && cargo horizon-mod build            exit 0       (drop-build-1.log)
the same again (twice)                                 byte-identical (cmp)
cargo horizon-mod check createaddition.hmod            0.2.10 … 190180 bytes (drop-check.log)
fits --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a  "is made for exactly one build (it has a
                                                       [native.mixin] table), so fits doesn't
                                                       apply", exit 2   (drop-fits-pin.log)
fits --build 0b14f8acfd97fa2c750e42795249dcc981178ca4  the same refusal, exit 2
                                                                    (drop-fits-other.log)
launcher-game.sh build createaddition   Compiling createaddition (1 of 1);
                                        Function replacement: 2 functions patched by 2 mixins
                                                                    (drop-launcher-patch.log)
python3 ports/tools/check-recipe-types.py createaddition   createaddition: ok
python3 ports/tools/verify-sdk-package.py createaddition   1/1     (verify-sdk-package.log)
```

**Both `fits` lines are refusals, not a pass and a red** — unchanged since 0.2.9 (a
`[[native.mixin]]` package is made for one build). The replacement evidence is the launcher build
above plus `strings libcreateaddition.so` carrying `createaddition:copper_wire` (the new code, in the
binary the game ran).

In game, `tools/e2e-drop.sh` (default / `DROP_SELFTEST=1` / `CONTROL_GAME=`), plus the whole A–M
suite on this build:

```
e2e-drop-default.log        PASS + EXIT:0   four rigs: copper, gold, electrum drop their own wire
                                            ×4, the festive wire drops copper ×4, every peer reads
                                            `links []`; the unlinked connector drops no wire
e2e-drop-selftest.log       FAIL + EXIT:1   DROP_SELFTEST=1: the unlinked break dropped 0 wires, so
                                            the differential's assertion fails — it can fail
e2e-drop-control.log        FAIL + EXIT:1   a Nightly without the package cannot place
                                            createaddition:connector
e2e-drop-suite.log          PASS + EXIT:0   the A–M suite on 0.2.10, every earlier family green
e2e-drop-suite-selftest.log FAIL + EXIT:1   the suite's conservation counterfactual is alive
python3 ports/tools/check-e2e-logs.py createaddition-e2e --strict    logs=40 ok=40
```

Deliverable: `createaddition.hmod`, **190,180 B**, SHA-256
`db4306b2ce83cbc58d0ce71919f87741285391ae1dd7f18dc39cdbce1c912f76`, **127 entries**, reproducible
(repeated builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…95d`, packed too).

## 0.2.9 — the wire renderer, the wire family's client half

Pin: Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` (nightly 2026-11-20), mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`; every cargo step inside one
`ports/tools/build-slot.sh` lease (`CARGO_INCREMENTAL=0`, no `--target-dir`); the package steps use
`BUILD_LOCAL=1` because `cargo horizon-mod` is installed on this box only (the far runner has no
`horizon-mod`). The in-game legs use the launcher-built game (`ports/tools/launcher-game.sh`,
launcher 0.2.92); this build is `native-0a875da-3c69290c67cd`, and its build log shows
`Compiling createaddition (1 of 1)` plus
`Function replacement: 2 functions patched by 2 mixins, without rebuilding the game`
(`wire-launcher-patch.log`).

This is the campaign's first `[[native.mixin]]`: the render functions are native-route places
outside `mod_api` at this pin, verified before the slice (route check in `notes.md` § *Slice 10*).

Logs: `ports/_shared/evidence/createaddition/` (`wire-red.log`, `sdk-test.log`, `sdk-clippy.log`,
`sdk-fmt.log`, `wire-build-1.log`, `wire-build-2.log`, `wire-check.log`, `wire-fits-pin.log`,
`wire-fits-other.log`, `wire-launcher-build.log`, `wire-launcher-patch.log`,
`verify-sdk-package.log`) and
`ports/_shared/evidence/createaddition-e2e/` (`e2e-wire-*.log`).

```
red, pre-slice renderer           91 passed, 3 failed  (wire-red.log)
  the WireType colour             left: [0, 0, 0]   right: [78, 37, 30]
  the catenary                    a four-block wire sags by about 0.06, not 0
  the festive lights              a festive wire draws its lights (576 vs 576 vertices)
cargo test --offline              94 passed, 0 failed             (sdk-test.log, regenerated)
cargo clippy --all-targets --offline -- -D warnings   exit 0, 0 warnings (sdk-clippy.log)
cargo fmt --check                 clean                            (sdk-fmt.log)
rm -f Cargo.lock && cargo horizon-mod build            exit 0       (wire-build-1.log)
rm -f Cargo.lock && cargo horizon-mod build (again)    byte-identical (cmp) (wire-build-2.log)
cargo horizon-mod check createaddition.hmod            0.2.9 … 187056 bytes (wire-check.log)
fits --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a  "is made for exactly one build (it has a
                                                       [native.mixin] table), so fits doesn't
                                                       apply", exit 2   (wire-fits-pin.log)
fits --build 0b14f8acfd97fa2c750e42795249dcc981178ca4  the same refusal, exit 2
                                                                    (wire-fits-other.log)
launcher-game.sh build createaddition   Compiling createaddition (1 of 1);
                                        Function replacement: 2 functions patched by 2 mixins
                                                                    (wire-launcher-build.log)
python3 tools/data_shape.py createaddition.hmod   6/6 26.3 loot tables
python3 tools/carry_content.py --check    assets/ and data/ match the jar carry
python3 tools/check_licence.py    upstream notice verbatim
python3 ports/tools/verify-sdk-package.py createaddition   1/1     (verify-sdk-package.log)
```

**The two `fits` lines are refusals, not a pass and a red.** A package with a `[[native.mixin]]`
row compiles its helper into a copy of one game build's own source, so it is made for exactly that
build and `fits` declines to answer for any build — the pin included. That is the route's cost;
the evidence that replaces it is the launcher build above, which compiled and applied both mixins
against `0a875da` and ran the game. 0.2.8's other-build line was red for the 0.2.5 lamp's
`BlockStates::light_emission`; that API gap is unchanged, but `fits` no longer evaluates the
surface for this package at all.

Deliverable: `createaddition.hmod`, **187,056 B**, SHA-256
`a9f4dcac05edbb72835f122f94514e034b41848aaa332266311bf8044a0e23e9`, **127 entries**, reproducible
(two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f95d`, packed too).

### In-game — five legs, `check-e2e-logs.py createaddition-e2e --strict`: logs=35 ok=35

`tools/e2e-wire.sh` is the renderer's own leg (`--offscreen`, screenshots) and its three verdicts;
`tools/e2e.sh` is the A–M suite, re-run whole on this package.

```
e2e-wire-default.log         PASS + EXIT:0   with the wire: 1901 green pixels, mean rgb(21, 86, 9);
                                             without: 0; box x 356..923, y 514..538
e2e-wire-negative.log        FAIL + EXIT:1   NEGATIVE: the no-wire frame carries 0 green pixels,
                                             so a renderer that draws nothing fails this leg
e2e-wire-control.log         FAIL + EXIT:1   a Nightly without the package does not know
                                             createaddition:connector (setblock left minecraft:air)
e2e-wire-suite.log           PASS + EXIT:0   A–M on 0.2.9, every earlier family unchanged
e2e-wire-suite-selftest.log  FAIL + EXIT:1   the suite's conservation counterfactual is alive
```

The assertion is a count of festive-green pixels in the wire's own screen box, not a raw frame
difference: the engine's brightness moves the floor's tone between two shots (measured: 25,600
changed pixels of lighting), and the chat log redraws with every command (86,413 changed pixels).
The green count is 1901 with the wire and 0 without it, and the wire's mean is the jar's
`WireType.FESTIVE` 26/94/12 shaded by its own 0.7.


## 0.2.8 — the festive wire and spool (and the spool's pending-selection defect)

Pin: Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` (nightly 2026-11-20), mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`; every cargo step inside one
`ports/tools/build-slot.sh` lease (`CARGO_INCREMENTAL=0`, no `--target-dir`); the package steps use
`BUILD_LOCAL=1` because `cargo horizon-mod` is installed on this box only (the far runner has no
`horizon-mod`). The in-game legs use the launcher-built game (`ports/tools/launcher-game.sh`,
launcher 0.2.92, `Compiling createaddition (1 of 1)`); this build is `native-0a875da-bbeb13d53095`.

Logs: `ports/_shared/evidence/createaddition/` (`festive-red.log`, `festive-build-1.log`,
`festive-build-2.log`, `festive-check.log`, `festive-fits-pin.log`, `festive-fits-other.log`,
`sdk-test.log`, `sdk-clippy.log`, `sdk-fmt.log`, `verify-sdk-package.log`) and
`ports/_shared/evidence/createaddition-e2e/` (`e2e-festive-*.log`, `probe-festive.log`).

```
red, pre-slice festive rules     3 failed: 81 passed  (festive-red.log)
  festive between two large      left: Linked   right: Refused(RequiresHighCurrent)
  the empty spool's refund       left: [(Copper, 3) … (Festive, 0)]
                                 right: [(Copper, 2) … (Festive, 1)]
  festive's rule                 left: Gold   right: Copper
cargo test --offline             84 passed, 0 failed              (sdk-test.log, regenerated)
cargo clippy --all-targets --offline -- -D warnings   exit 0, 0 warnings (sdk-clippy.log)
cargo fmt --check                clean                            (sdk-fmt.log)
rm -f Cargo.lock && cargo horizon-mod build           exit 0       (festive-build-1.log)
rm -f Cargo.lock && cargo horizon-mod build (again)   byte-identical (cmp) (festive-build-2.log)
cargo horizon-mod check createaddition.hmod           exit 0       (festive-check.log)
fits --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a  "0a875da  made for" (festive-fits-pin.log)
fits --build 0b14f8acfd97fa2c750e42795249dcc981178ca4  "does not fit: 1 item changed
                                                       the method `BlockStates::light_emission` (removed)"
                                                                    (festive-fits-other.log)
python3 tools/data_shape.py createaddition.hmod   6/6 26.3 loot tables
python3 tools/carry_content.py --check    assets/ and data/ match the jar carry
python3 tools/check_licence.py    upstream notice verbatim       (licence-notice.log)
python3 ports/tools/verify-sdk-package.py createaddition   1/1     (verify-sdk-package.log)
```

Deliverable: `createaddition.hmod`, **186,895 B**, SHA-256
`715ce0ea799f9ad7c16d8bbc33df8b6c3fcafa2057ddf03bf60b84620a5025a4`, **125 entries**, reproducible
(two builds `cmp`-identical), no packed `Cargo.lock`, `src/runtime.rs` byte-identical
(`03721ca4…f962`, packed too); 114 `assets/`+`data/` files (was 106).

```
createaddition 0.2.8 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  186895 bytes, SHA-256 715ce0ea799f9ad7c16d8bbc33df8b6c3fcafa2057ddf03bf60b84620a5025a4, unsigned
```

```
Create Crafts & Additions:
0a875da  made for
Create Crafts & Additions:
0b14f8a  does not fit: 1 item changed
    the method `BlockStates::light_emission` (removed)
```

**The second `fits` run is red and that is the measured finding**: the lamp's `light_emission`
surface (0.2.5) does not exist at `0b14f8a`. This slice adds no new API surface — `Laying`,
`Pending` and the carried items use only what the pin has.

| log | verdict | what it proves |
|---|---|---|
| `e2e-festive-default.log` | `PASS` + `EXIT:0` | A–L unchanged in this run (the relay carried 25,020 FE conserved, the lamp 1 FE/t over 43 ticks, the comparator 0/8/15, the generator exactly 1,000 FE/t over 28 ticks), plus **M**: the festive spool (the **item**, two `ctl interact` clicks) lays a wire both ends save as `festive`, exactly one festive spool becomes exactly one empty spool, the wire carries **exactly 1,000 FE/t** (29,000 over 29 ticks; 20,000 in the 20-tick window), festive is refused between two large connectors while **gold is accepted on the same pair**, `createaddition:iron_wire` is held while `link … iron` is refused, and the empty spool hands back exactly one festive spool (0 → 1) |
| `e2e-festive-selftest.log` | `FAIL` + `EXIT:1` | `FAIL: FESTIVE_SELFTEST: festive was refused between two large connectors (Two large connectors need a gold or electrum wire.), so the refusal assertion is alive` |
| `e2e-festive-suite-selftest.log` | `FAIL` + `EXIT:1` | the suite's conservation counterfactual (`SELFTEST: the conservation assertion did not trip`) |
| `e2e-festive-control.log` | `FAIL` + `EXIT:1` | the package-less game has no accumulator item (`the accumulator item did not register: …`); the default run's control leg also refuses `createaddition:festive_spool` and `createaddition:iron_wire` by id |
| `probe-festive.log` | `PASS` + `EXIT:0` | the probe that found this port's pending-selection defect (one spool became three copies, no wire laid) and then pinned the click geometry (`FESTIVE_ONLY=1`) |

`python3 ports/tools/check-e2e-logs.py createaddition-e2e --strict`:
**logs=30 ok=30 missing-exit=0 bad-exit=0** (the 29 earlier legs still green).

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
