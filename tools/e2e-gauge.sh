#!/usr/bin/env bash
# The accumulator gauge's in-game leg: the **client half** of the accumulator family.
#
# Upstream's `ModularAccumulatorRenderer.renderDial` swings the dial by `-180 * fill` degrees on
# all four horizontal faces. Until 0.2.13 this port drew no fill at all: the jar's gauge texture
# was carried but no needle was rendered. The needle now uses the same wgpu pipeline as the
# 0.2.9 wire renderer — one extra draw of the existing shader — with the jar's gauge cyan, so a
# screenshot counts the needle rather than a grey panel's brightness.
#
# **The assertion is an offscreen screenshot differential at two fills.** The same scene, camera
# and settled tick window, as PNG from `horizon-ctl screenshot`:
#
#   empty — `ctl chat "/createaddition energy <pos> 0"`
#   full  — `ctl chat "/createaddition energy <pos> 2000000"` (one block's full capacity)
#
# The needle is the jar's gauge-texture cyan (123, 213, 214), which nothing in the End's void and
# stone floor carries. The leg counts those pixels in the gauge's own screen box, split above and
# below the box's middle row: from this camera the `-180 * fill` swing puts an empty needle above
# the pivot and a full one below it (upper/lower 88/48 at 0, 1/160 at full, measured in
# `probe-gauge-screen.log`). A renderer that draws no gauge leaves both frames with 0 cyan pixels
# (that is what NEGATIVE asserts, by taking the second frame with the accumulator gone).
#
# A raw frame difference is not the assertion: the engine's own brightness shifts the floor tone
# between two shots and the chat redraws with every command (see e2e-wire.sh's measured numbers).
#
# `CONTROL_GAME=` runs the setup on a Nightly without the package: `createaddition:*` is unknown,
# the accumulator cannot be placed, and the leg fails.
set -Eeuo pipefail
export LC_ALL=C
[ -z "${CA_DEBUG:-}" ] || set -x

NEGATIVE=${NEGATIVE:-0}
CONTROL=${CONTROL_GAME:+1}
CONTROL=${CONTROL:-0}
PORT=$(cd "$(dirname "$0")/.." && pwd)
CTL=${CTL:-}
GAME=${GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
TMP=${E2E_TMP:-/tmp/opencode}
mkdir -p "$TMP"
SOCK=${SOCK:-$TMP/createaddition-gauge.sock}
WCROP=${WCROP:-gaugebox}
KEEP=${KEEP:-0}
WORLD=CreateAdditionGauge
CONTROL_WORLD=CreateAdditionGaugeControl
EVIDENCE=$(cd "$PORT/../_shared/evidence/createaddition-e2e" && pwd)

X0=400
Y=101
ACC=$X0
# Cyan pixels the gauge box must carry in each frame (the needle is ~40 px long at this camera).
PIXELS=25
# The swing is measured as upper-minus-lower cyan pixels in the gauge box: an empty needle sits
# above the box's middle row, a full one below it. These floors are well inside the measured
# 88/48 (empty) and 1/160 (full); a gauge that does not move cannot satisfy both.
SWING=20
# A changed pixel differs in at least one channel by this much (settling only).
DELTA=6
NOISE=50

guard() { echo "EXIT:1"; exit 1; }
command -v jq >/dev/null || { echo "missing jq (every assertion parses a ctl reply)" >&2; guard; }
command -v python3 >/dev/null || { echo "missing python3 (the pixel count)" >&2; guard; }
[ -x "$CTL" ] || { echo "missing horizon-ctl: CTL=$CTL" >&2; guard; }
[ -n "$GAME" ] || { echo "GAME= is required: this package is native, so the game must be launcher-built" >&2; guard; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; guard; }

SCRATCH=$(mktemp -d "$TMP/createaddition-gauge-home.XXXXXX")
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
chat() { ctl messages 60 | jq -r '.chat[]'; }
recent_chat() { chat | head -20; }
fail() { echo "FAIL: $*" >&2; recent_chat >&2 || true; ctl logs --lines 40 >&2 || true; exit 1; }
on_error() { echo "FAIL: line $1: '$2' exited $?" >&2; ctl logs --lines 40 >&2 || true; }
trap 'on_error "$LINENO" "$BASH_COMMAND"' ERR

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then echo "kept scratch: $SCRATCH" >&2; else rm -rf "$SCRATCH" 2>/dev/null || true; fi
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() { # launch BINARY
    local game=$1
    rm -f "$SOCK"
    cd "$SCRATCH"
    "$CTL" --socket "$SOCK" launch --game "$game" --world "$WORLD" --seed 20261105 \
        --creative --offscreen --wait --resources "$RES" >/dev/null
    ctl wait-ready 120
    ctl wait 40
}

block_name() { ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty'; }
NEXT_TAG=0
command_reply() { # command_reply COMMAND REPLY_PATTERN
    NEXT_TAG=$((NEXT_TAG + 1))
    local tag pattern round
    tag="q$NEXT_TAG"
    pattern="$2 .*\\[#$tag\\]$"
    for round in 1 2; do
        say "$1 #$tag"
        for _ in $(seq 1 30); do
            ctl wait 3 >/dev/null
            if chat | grep -qE "$pattern"; then
                REPLY=$(chat | grep -E "$pattern" | tail -1)
                REPLY=${REPLY% "[#$tag]"}
                REPLY=${REPLY% }
                return 0
            fi
        done
    done
    echo "--- newest chat lines:" >&2
    chat | tail -15 >&2
    fail "no reply to /$1 #$tag"
}
ENERGY_REPLY='^Energy set to [0-9]+ FE\.'
INFO_REPLY="^Modular Accumulator at $ACC,$Y,0:"

arena() {
    ctl command "execute in minecraft:the_end run tp @s $((X0 + 4)).5 104 0.5" >/dev/null
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(ctl block "$ACC" 100 0 | jq -r '.loaded // false')" = true ]; then loaded=true; break; fi
    done
    [ "$loaded" = true ] || fail "the End arena chunk never loaded"
    ctl command "execute in minecraft:the_end run fill $((X0 - 8)) 100 -10 $((X0 + 14)) 100 10 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run fill $((X0 - 8)) 101 -10 $((X0 + 14)) 112 10 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run kill @e[type=!player]" >/dev/null
    ctl wait 6 >/dev/null
    [ "$(block_name "$ACC" 100 0)" = "minecraft:stone" ] || fail "the arena floor is not stone"
    [ "$(block_name "$ACC" $Y 0)" = "minecraft:air" ] || fail "the air above the floor is not air"
}

rig() {
    ctl command "execute in minecraft:the_end run setblock $ACC $Y 0 createaddition:modular_accumulator" >/dev/null
    ctl wait 6 >/dev/null
    [ "$(block_name "$ACC" $Y 0)" = "createaddition:modular_accumulator" ] \
        || fail "the accumulator did not place at $ACC,$Y,0 (got '$(block_name "$ACC" $Y 0)')"
}

# Feet on the floor, four blocks east of the accumulator, looking straight at its gauge pivot:
# the east face is the jar's base direction, and the needle swings in that face's plane.
camera() {
    ctl command "execute in minecraft:the_end run tp @s $((X0 + 4)).5 $Y 0.5 0 0" >/dev/null
    local settled=false pos="unknown" before
    before=$(ctl state | jq -r '.player.position | @csv')
    for _ in $(seq 1 40); do
        ctl wait 10 >/dev/null
        pos=$(ctl state | jq -r '.player.position | @csv')
        if [ "$pos" = "$before" ] && [ "$(ctl state | jq -r '.player.on_ground // false')" = true ]; then
            settled=true
            echo "  camera settled at $pos" >&2
            break
        fi
        before=$pos
    done
    [ "$settled" = true ] || fail "the player never stood still on the floor (at $pos)"
    ctl look-at "$((X0 + 1)).125 $Y.375 0.5" >/dev/null
    ctl wait 10 >/dev/null
}

let_chat_fade() { ctl wait 360 >/dev/null; }

unpause() {
    ctl resume >/dev/null 2>&1 || true
    local paused
    paused=$(ctl state | jq -r '.debug.pause_menu // false')
    if [ "$paused" != false ]; then
        ctl wait 4 >/dev/null
        ctl resume >/dev/null 2>&1 || true
        paused=$(ctl state | jq -r '.debug.pause_menu // false')
    fi
    [ "$paused" = false ] || fail "the game is paused (pause_menu=$paused); its frame is not the world"
}

# A shot of a frame that has stopped changing: chunks keep meshing after the camera arrives.
settle_shot() { # settle_shot PATH TAG
    local path=$1 tag=$2 attempt count
    for attempt in $(seq 1 8); do
        unpause
        ctl screenshot "$SCRATCH/settle-a.png" >/dev/null
        ctl wait 20 >/dev/null
        unpause
        ctl screenshot "$SCRATCH/settle-b.png" >/dev/null
        read -r _ count _ <<<"$(compare "$SCRATCH/settle-a.png" "$SCRATCH/settle-b.png")"
        if [ "$count" -lt "$NOISE" ]; then
            cp "$SCRATCH/settle-b.png" "$path"
            echo "  $tag: settled (two frames twenty ticks apart differ by $count pixels)" >&2
            return 0
        fi
        echo "  $tag: still changing ($count pixels), waiting for the chunks" >&2
        ctl wait 40 >/dev/null
    done
    fail "$tag: the scene never settled (the last two frames differ by $count pixels)"
}

compare() { # compare WITH AFTER -> CHANGED MEAN_R MEAN_G MEAN_B BOX... SIZE...
    python3 - "$1" "$2" "$DELTA" "$WCROP" <<'PY'
import sys
from PIL import Image
with_path, after_path, delta, crop_name = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
a = Image.open(with_path).convert("RGB"); b = Image.open(after_path).convert("RGB")
if a.size != b.size:
    print(f"SIZE {a.size} {b.size}"); sys.exit(0)
w, h = a.size
crops = {
    "gaugebox": (w * 30 // 100, h * 30 // 100, w * 70 // 100, h * 70 // 100),
    "full": (0, 0, w, h),
}
box = crops[crop_name]
pa, pb = a.load(), b.load()
changed = rs = gs = bs = 0
x0 = y0 = 10 ** 9
x1 = y1 = -1
for y in range(box[1], box[3]):
    for x in range(box[0], box[2]):
        ca, cb = pa[x, y], pb[x, y]
        if abs(ca[0] - cb[0]) > delta or abs(ca[1] - cb[1]) > delta or abs(ca[2] - cb[2]) > delta:
            changed += 1
            rs += ca[0]; gs += ca[1]; bs += ca[2]
            x0, y0, x1, y1 = min(x0, x), min(y0, y), max(x1, x), max(y1, y)
if changed:
    print(f"CHANGED {changed} MEAN {rs // changed} {gs // changed} {bs // changed} "
          f"BOX {x0} {y0} {x1} {y1} SIZE {w} {h}")
else:
    print(f"CHANGED 0 MEAN 0 0 0 BOX 0 0 0 0 SIZE {w} {h}")
PY
}

# The needle's cyan pixels in the gauge box, and their mean x (the flip the leg asserts).
needle_pixels() { # needle_pixels PATH -> COUNT MEAN_R MEAN_G MEAN_B MEAN_X MEAN_Y UPPER LOWER
    python3 - "$1" "$WCROP" <<'PY'
import sys
from PIL import Image
path, crop_name = sys.argv[1], sys.argv[2]
im = Image.open(path).convert("RGB")
w, h = im.size
crops = {
    "gaugebox": (w * 30 // 100, h * 30 // 100, w * 70 // 100, h * 70 // 100),
    "full": (0, 0, w, h),
}
box = crops[crop_name]
middle_row = (box[1] + box[3]) // 2
px = im.load()
count = rs = gs = bs = xs = ys = upper = lower = 0
for y in range(box[1], box[3]):
    for x in range(box[0], box[2]):
        r, g, b = px[x, y]
        if g > r + 40 and b > r + 40:   # the jar's gauge cyan 123/213/214, not a grey panel
            count += 1
            rs += r; gs += g; bs += b; xs += x; ys += y
            if y < middle_row: upper += 1
            else: lower += 1
if count:
    print(count, rs // count, gs // count, bs // count, xs // count, ys // count, upper, lower)
else:
    print(0, 0, 0, 0, 0, 0, 0, 0)
PY
}

if [ "$CONTROL" = 1 ]; then
    echo "== control: the same rig on a Nightly without the package =="
    WORLD=$CONTROL_WORLD
    launch "$CONTROL_GAME"
    arena
    ctl command "execute in minecraft:the_end run setblock $ACC $Y 0 createaddition:modular_accumulator" >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    got=$(block_name "$ACC" $Y 0)
    [ "$got" != "createaddition:modular_accumulator" ] \
        || fail "the control game placed an accumulator, so the block id was known"
    fail "the control game does not know createaddition:modular_accumulator (the setblock left '$got'), so there is no gauge to draw: the cyan pixels the modded frames count come from this package"
fi

echo "== launch (The End arena, launcher-built game, --offscreen for screenshots) =="
launch "$GAME"
ctl options-set pauseOnLostFocus false >/dev/null 2>&1 || true
arena
rig
camera
let_chat_fade

echo "== empty: /createaddition energy $ACC $Y 0 0 =="
command_reply "createaddition energy $ACC $Y 0 0" "$ENERGY_REPLY"
echo "  $REPLY" >&2
command_reply "createaddition info $ACC $Y 0" "$INFO_REPLY"
echo "  $REPLY" >&2
ctl wait 20 >/dev/null
let_chat_fade
settle_shot "$SCRATCH/gauge-empty.png" "empty gauge"
read -r EMPTY EMPTY_R EMPTY_G EMPTY_B EMPTY_X EMPTY_Y EMPTY_UP EMPTY_LOW <<<"$(needle_pixels "$SCRATCH/gauge-empty.png")"
echo "  empty: $EMPTY cyan needle pixels in the gauge box, mean rgb($EMPTY_R, $EMPTY_G, $EMPTY_B), mean x $EMPTY_X, mean y $EMPTY_Y, upper $EMPTY_UP / lower $EMPTY_LOW" >&2

if [ "${GAUGE_PROBE:-0}" = 1 ]; then
    echo "== GAUGE_PROBE: sweep the fill and print the needle's pixel numbers ==" >&2
    for amount in 0 500000 1000000 1500000 2000000; do
        command_reply "createaddition energy $ACC $Y 0 $amount" "$ENERGY_REPLY"
        ctl wait 20 >/dev/null
        let_chat_fade
        settle_shot "$SCRATCH/probe-$amount.png" "probe fill $amount"
        read -r PROBE_COUNT PROBE_R PROBE_G PROBE_B PROBE_X PROBE_MEANY PROBE_UP PROBE_LOW <<<"$(needle_pixels "$SCRATCH/probe-$amount.png")"
        echo "PROBE fill=$amount count=$PROBE_COUNT rgb=$PROBE_R,$PROBE_G,$PROBE_B meanx=$PROBE_X meany=$PROBE_MEANY upper=$PROBE_UP lower=$PROBE_LOW" >&2
        cp "$SCRATCH/probe-$amount.png" "$EVIDENCE/probe-gauge-fill-$amount.png" 2>/dev/null || true
    done
    echo "PASS: gauge probe" >&2
    exit 0
fi

if [ "$NEGATIVE" = 1 ]; then
    echo "== NEGATIVE: the gauge is absent (the accumulator is removed) ==" >&2
    ctl command "execute in minecraft:the_end run setblock $ACC $Y 0 minecraft:air" >/dev/null
    ctl wait 20 >/dev/null
    let_chat_fade
    settle_shot "$SCRATCH/gauge-full.png" "gauge absent"
    read -r ABSENT _ _ _ _ <<<"$(needle_pixels "$SCRATCH/gauge-full.png")"
    cp "$SCRATCH/gauge-empty.png" "$SCRATCH/gauge-full.png" "$EVIDENCE/" 2>/dev/null || true
    [ "$ABSENT" -lt "$PIXELS" ] \
        || fail "NEGATIVE must not pass: a frame with no gauge carries $ABSENT cyan pixels"
    fail "NEGATIVE: the frame with the accumulator removed carries only $ABSENT cyan needle pixels (the empty gauge carries $EMPTY), so this leg fails on a renderer that draws no gauge"
fi

echo "== full: /createaddition energy $ACC $Y 0 2000000 =="
command_reply "createaddition energy $ACC $Y 0 2000000" "$ENERGY_REPLY"
echo "  $REPLY" >&2
command_reply "createaddition info $ACC $Y 0" "$INFO_REPLY"
echo "  $REPLY" >&2
ctl wait 20 >/dev/null
let_chat_fade
settle_shot "$SCRATCH/gauge-full.png" "full gauge"
read -r FULL FULL_R FULL_G FULL_B FULL_X FULL_Y FULL_UP FULL_LOW <<<"$(needle_pixels "$SCRATCH/gauge-full.png")"
echo "  full: $FULL cyan needle pixels in the gauge box, mean rgb($FULL_R, $FULL_G, $FULL_B), mean x $FULL_X, mean y $FULL_Y, upper $FULL_UP / lower $FULL_LOW" >&2

# The two PNGs are the evidence; the numbers are printed above, not just implied by them.
cp "$SCRATCH/gauge-empty.png" "$EVIDENCE/gauge-empty.png"
cp "$SCRATCH/gauge-full.png" "$EVIDENCE/gauge-full.png"
echo "  dumped gauge-empty.png and gauge-full.png to $EVIDENCE" >&2

read -r _ CHANGED _ MEAN_CHANGED_R MEAN_CHANGED_G MEAN_CHANGED_B _ BOX_X0 BOX_Y0 BOX_X1 BOX_Y1 _ SIZE_W SIZE_H \
    <<<"$(compare "$SCRATCH/gauge-empty.png" "$SCRATCH/gauge-full.png")"
echo "  the frames differ in $CHANGED pixels of the box overall (mean rgb($MEAN_CHANGED_R, $MEAN_CHANGED_G, $MEAN_CHANGED_B)); the needle cyan is counted, not the floor's tone" >&2
echo "  frame size: $SIZE_W x $SIZE_H; the changed pixels' box: x $BOX_X0..$BOX_X1, y $BOX_Y0..$BOX_Y1" >&2

[ "$EMPTY" -ge "$PIXELS" ] \
    || fail "only $EMPTY cyan needle pixels in the empty gauge's box, expected at least $PIXELS (no needle was drawn, or it is outside the box)"
[ "$FULL" -ge "$PIXELS" ] \
    || fail "only $FULL cyan needle pixels in the full gauge's box, expected at least $PIXELS"
[ $((EMPTY_UP - EMPTY_LOW)) -ge "$SWING" ] \
    || fail "the empty needle did not sit above the pivot: upper=$EMPTY_UP lower=$EMPTY_LOW (need >= $SWING)"
[ $((FULL_LOW - FULL_UP)) -ge "$SWING" ] \
    || fail "the full needle did not sit below the pivot: upper=$FULL_UP lower=$FULL_LOW (need >= $SWING)"
[ $((FULL_G - FULL_R)) -ge 30 ] && [ $((FULL_B - FULL_R)) -ge 30 ] \
    || fail "the needle is rgb($FULL_R, $FULL_G, $FULL_B): not the gauge texture's cyan (123, 213, 214)"

echo "PASS: the accumulator's gauge needle is drawn and swings with its fill (empty $EMPTY cyan pixels, upper/lower $EMPTY_UP/$EMPTY_LOW; full $FULL, $FULL_UP/$FULL_LOW — the needle moves across the gauge's middle row)"
