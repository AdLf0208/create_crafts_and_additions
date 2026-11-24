#!/usr/bin/env bash
# The wire renderer's lightmap leg: the missing half of the 0.2.9 wire renderer.
#
# Upstream's `WireNodeRenderer.wireRender` reads block and sky light at both wire endpoints, and
# `wirePart` interpolates each layer per piece (`Mth.lerp(part, start, end)` narrowed to int), packs
# it with `LightTexture.pack`, and gives it to `setLight`. The 0.2.9 port drew the wire at its
# `WireType` colour with no lightmap: a wire in a dark room was as bright as one in daylight.
#
# **The assertion is two --offscreen screenshots of the same festive-wire rig at two light
# levels.** The rig is the proven one from `e2e-wire.sh` (accumulator – connector – festive wire –
# connector – accumulator in The End). A glowstone placed near the wire makes a lit pad; moving the
# same glowstone further away makes a dark room. The leg counts the wire's **own green pixels** in
# the wire's screen box and prints the mean brightness of those pixels in each frame:
#
#   lit  — wire rgb near the jar's festive 26/94/12 (shaded 0.7/1.0)
#   dark — the same wire's green pixels, with a mean green level measurably lower
#
# A brightness-only check of the whole box is invalid (the glowstone lights the floor too): only
# the wire's green-dominant pixels are counted and averaged. Nothing else in the End's void and
# stone floor is green, so a renderer that draws nothing counts 0 — that is NEGATIVE's proof.
#
# SELFTEST=1 inverts the comparison (asks the dark frame to be brighter) and must exit 1.
# NEGATIVE=1 unlinks the wire and asks the positive count of the empty frame; that must exit 1.
# CONTROL_GAME= runs the rig on a Nightly without the package: createaddition:* is unknown.
set -Eeuo pipefail
export LC_ALL=C
[ -z "${CA_DEBUG:-}" ] || set -x

NEGATIVE=${NEGATIVE:-0}
SELFTEST=${SELFTEST:-0}
PROBE=${LIGHTMAP_PROBE:-0}
CONTROL=${CONTROL_GAME:+1}
CONTROL=${CONTROL:-0}
PORT=$(cd "$(dirname "$0")/.." && pwd)
CTL=${CTL:-}
GAME=${GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
TMP=${E2E_TMP:-/tmp/opencode}
mkdir -p "$TMP"
SOCK=${SOCK:-$TMP/createaddition-lightmap.sock}
WCROP=wirebox
KEEP=${KEEP:-0}
WORLD=CreateAdditionLightmap
CONTROL_WORLD=CreateAdditionLightmapControl
EVIDENCE=$(cd "$PORT/../_shared/evidence/createaddition-e2e" && pwd)

X0=268
Y=101
AX=$X0
C1=$((X0 + 1))
C2=$((X0 + 5))
B=$((X0 + 6))
GX=$((X0 + 3))
GY=$((Y + 3))
# The glowstone sits three blocks ABOVE the wire and north of it (behind it from this camera):
# high enough to light the rig without standing in the middle of the frame and occluding the wire.
# A near one lights it; a far one leaves it dim. The End has no sky light, so the only light on the
# wire is what this block gives.
LIT_Z=${LIT_Z:--1}
DARK_Z=${DARK_Z:--4}
# A wire is ~1,900 green pixels at 1280x720; 300 is a floor an undrawn wire (0) cannot reach.
PIXELS=300
# The mean green level of the wire's own pixels must differ by at least this much between frames.
MEAN_MARGIN=10
DELTA=6
# The End spawns drifting purple particles across the whole frame; they are not the wire and are
# not green-dominant, but they do make two otherwise identical frames differ by a few thousand
# pixels. A real relight changes most of the wire box, so this sits between the two.
NOISE=20000

guard() { echo "EXIT:1"; exit 1; }
command -v jq >/dev/null || { echo "missing jq (every assertion parses a ctl reply)" >&2; guard; }
command -v python3 >/dev/null || { echo "missing python3 (the pixel count)" >&2; guard; }
[ -x "$CTL" ] || { echo "missing horizon-ctl: CTL=$CTL" >&2; guard; }
[ -n "$GAME" ] || { echo "GAME= is required: this package is native, so the game must be launcher-built" >&2; guard; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; guard; }

SCRATCH=$(mktemp -d "$TMP/createaddition-lightmap-home.XXXXXX")
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

launch() {
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
command_reply() {
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
    chat | tail -15 >&2
    fail "no reply to /$1 #$tag"
}
LINK_REPLY='^(Nodes linked\.|The connectors are too far apart for this wire\.|Every node of this connector has a wire\.|These connectors are already linked\.|A connector cannot be linked to itself\.|Two large connectors need a gold or electrum wire\.)'
WIRE_REPLY='^(Wire removed\.|These connectors are not linked\.)'

arena() {
    ctl command "execute in minecraft:the_end run tp @s $((X0 + 3)).5 104 0.5" >/dev/null
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(ctl block "$X0" 100 0 | jq -r '.loaded // false')" = true ]; then loaded=true; break; fi
    done
    [ "$loaded" = true ] || fail "the End arena chunk never loaded"
    ctl command "execute in minecraft:the_end run fill $((X0 - 20)) 100 -20 $((X0 + 14)) 100 12 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run fill $((X0 - 20)) 101 -20 $((X0 + 14)) 112 12 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run kill @e[type=!player]" >/dev/null
    ctl wait 6 >/dev/null
    [ "$(block_name "$X0" 100 0)" = "minecraft:stone" ] || fail "the arena floor is not stone"
    [ "$(block_name "$X0" 101 0)" = "minecraft:air" ] || fail "the air above the floor is not air"
}

rig() {
    local at name x
    ctl command "execute in minecraft:the_end run setblock $AX $Y 0 createaddition:modular_accumulator" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run setblock $C1 $Y 0 createaddition:connector[facing=west,mode=none]" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run setblock $C2 $Y 0 createaddition:connector[facing=east,mode=none]" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run setblock $B $Y 0 createaddition:modular_accumulator" >/dev/null
    ctl wait 6 >/dev/null
    for at in "$AX:createaddition:modular_accumulator" "$C1:createaddition:connector" \
              "$C2:createaddition:connector" "$B:createaddition:modular_accumulator"; do
        x=${at%%:*}
        name=${at#*:}
        [ "$(block_name "$x" $Y 0)" = "$name" ] \
            || fail "the rig block $name did not place at $x,$Y,0 (got '$(block_name "$x" $Y 0)')"
    done
}

camera() {
    ctl command "execute in minecraft:the_end run tp @s $((X0 + 3)).5 $Y 4.5 0 12" >/dev/null
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
    ctl look-at "$((X0 + 3)).5 102.7 0.5" >/dev/null
    ctl wait 10 >/dev/null
}

# A glowstone moved between two z positions: the only light on the wire in the End.
LIGHT_Z=""            # the position the glowstone currently occupies
set_light() { # set_light Z
    local z=$1
    if [ -n "$LIGHT_Z" ] && [ "$LIGHT_Z" != "$z" ]; then
        ctl command "execute in minecraft:the_end run setblock $GX $GY $LIGHT_Z minecraft:air" >/dev/null
    fi
    ctl command "execute in minecraft:the_end run setblock $GX $GY $z minecraft:glowstone" >/dev/null
    LIGHT_Z=$z
    # A block light change makes the engine relight the region; the chunk mesh and the wire's own
    # vertex colours settle over a few seconds, so give it time before any frame is measured.
    ctl wait 60 >/dev/null
    echo "  light: glowstone at $GX,$GY,$z" >&2
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
box = (w * 20 // 100, h * 60 // 100, w * 80 // 100, h * 84 // 100)
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

# The wire's own green pixels in the wire box, and their mean colour. `WireType.FESTIVE` is
# 26/94/12; the jar's 0.7 alternating shade keeps every piece green-dominant. The End's void and
# stone floor are not, so this counts the wire and not the glowstone-lit floor.
wire_pixels() { # wire_pixels PATH -> COUNT MEAN_R MEAN_G MEAN_B
    python3 - "$1" <<'PY'
import sys
from PIL import Image
im = Image.open(sys.argv[1]).convert("RGB")
w, h = im.size
box = (w * 20 // 100, h * 60 // 100, w * 80 // 100, h * 84 // 100)
px = im.load()
count = rs = gs = bs = 0
for y in range(box[1], box[3]):
    for x in range(box[0], box[2]):
        r, g, b = px[x, y]
        if g > r + 15 and g > b + 15:
            count += 1
            rs += r; gs += g; bs += b
if count:
    print(count, rs // count, gs // count, bs // count)
else:
    print(0, 0, 0, 0)
PY
}

if [ "$CONTROL" = 1 ]; then
    echo "== control: the same rig on a Nightly without the package =="
    WORLD=$CONTROL_WORLD
    launch "$CONTROL_GAME"
    arena
    ctl command "execute in minecraft:the_end run setblock $C1 $Y 0 createaddition:connector" >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    got=$(block_name "$C1" $Y 0)
    [ "$got" != "createaddition:connector" ] \
        || fail "the control game placed a connector, so the block id was known"
    fail "the control game does not know createaddition:connector (the setblock left '$got'), so the rig cannot be built: the green pixels the modded frames count come from this package"
fi

echo "== launch (The End arena, launcher-built game, --offscreen for screenshots) =="
launch "$GAME"
ctl options-set pauseOnLostFocus false >/dev/null 2>&1 || true
arena
rig
echo "== lay the festive wire =="
command_reply "createaddition link $C1 $Y 0 $C2 $Y 0 festive" "$LINK_REPLY"
[ "$REPLY" = "Nodes linked." ] || fail "the festive wire was not laid: $REPLY"
command_reply "createaddition info $C1 $Y 0" "^(Connector|Large Connector|Small Light Connector) at $C1,$Y,0:"
case "$REPLY" in *festive*) echo "  $REPLY" >&2 ;; *) fail "the wire is not saved as festive: $REPLY" ;; esac
camera
let_chat_fade

if [ "$PROBE" = 1 ]; then
    echo "== LIGHTMAP_PROBE: sweep the glowstone distance and print the wire's pixel means ==" >&2
    for z in -1 -4 -7 -10; do
        set_light "$z"
        let_chat_fade
        settle_shot "$SCRATCH/probe-$z.png" "probe glowstone z=$z"
        read -r PCOUNT PR PG PB <<<"$(wire_pixels "$SCRATCH/probe-$z.png")"
        echo "PROBE z=$z count=$PCOUNT rgb=$PR,$PG,$PB" >&2
        cp "$SCRATCH/probe-$z.png" "$EVIDENCE/probe-lightmap-$z.png" 2>/dev/null || true
    done
    echo "PASS: lightmap probe" >&2
    exit 0
fi

echo "== lit pad: glowstone at z=$LIT_Z =="
set_light "$LIT_Z"
let_chat_fade
settle_shot "$SCRATCH/wire-lit.png" "lit wire"
read -r LIT LIT_R LIT_G LIT_B <<<"$(wire_pixels "$SCRATCH/wire-lit.png")"
echo "  lit: $LIT green wire pixels, mean rgb($LIT_R, $LIT_G, $LIT_B)" >&2

echo "== dark room: the same glowstone at z=$DARK_Z =="
set_light "$DARK_Z"
let_chat_fade
settle_shot "$SCRATCH/wire-dark.png" "dark wire"
read -r DARK DARK_R DARK_G DARK_B <<<"$(wire_pixels "$SCRATCH/wire-dark.png")"
echo "  dark: $DARK green wire pixels, mean rgb($DARK_R, $DARK_G, $DARK_B)" >&2

if [ "$NEGATIVE" = 1 ]; then
    echo "== NEGATIVE: the wire is removed, so there are no wire pixels ==" >&2
    command_reply "createaddition unlink $C1 $Y 0 $C2 $Y 0" "$WIRE_REPLY"
    [ "$REPLY" = "Wire removed." ] || fail "the wire was not removed: $REPLY"
    let_chat_fade
    settle_shot "$SCRATCH/wire-absent.png" "wire absent"
    read -r ABSENT _ _ _ <<<"$(wire_pixels "$SCRATCH/wire-absent.png")"
    echo "  without the wire: $ABSENT green wire pixels" >&2
    cp "$SCRATCH/wire-lit.png" "$SCRATCH/wire-dark.png" "$SCRATCH/wire-absent.png" "$EVIDENCE/" 2>/dev/null || true
    [ "$ABSENT" -lt "$PIXELS" ] \
        || fail "NEGATIVE must not pass: a frame with no wire carries $ABSENT green pixels"
    fail "NEGATIVE: the frame with the wire removed carries $ABSENT green wire pixels (lit carries $LIT), so this leg fails on a renderer that draws no wire"
fi

cp "$SCRATCH/wire-lit.png" "$SCRATCH/wire-dark.png" "$EVIDENCE/" 2>/dev/null || true
echo "  dumped wire-lit.png and wire-dark.png to $EVIDENCE" >&2

read -r _ CHANGED _ MEAN_CHANGED_R MEAN_CHANGED_G MEAN_CHANGED_B _ BOX_X0 BOX_Y0 BOX_X1 BOX_Y1 _ SIZE_W SIZE_H \
    <<<"$(compare "$SCRATCH/wire-lit.png" "$SCRATCH/wire-dark.png")"
echo "  the frames differ in $CHANGED pixels of the box overall (mean rgb($MEAN_CHANGED_R, $MEAN_CHANGED_G, $MEAN_CHANGED_B)); the wire's own green pixels are counted, not the floor's tone" >&2
echo "  frame size: $SIZE_W x $SIZE_H; the changed pixels' box: x $BOX_X0..$BOX_X1, y $BOX_Y0..$BOX_Y1" >&2

[ "$LIT" -ge "$PIXELS" ] \
    || fail "only $LIT green wire pixels in the lit frame, expected at least $PIXELS"
[ "$DARK" -ge "$PIXELS" ] \
    || fail "only $DARK green wire pixels in the dark frame, expected at least $PIXELS (the wire must stay visible, just dimmer)"
[ $((LIT_G - LIT_R)) -ge 30 ] && [ $((LIT_G - LIT_B)) -ge 30 ] \
    || fail "the lit wire's pixels are rgb($LIT_R, $LIT_G, $LIT_B): not the festive wire's green (26, 94, 12)"
[ $((DARK_G - DARK_R)) -ge 15 ] && [ $((DARK_G - DARK_B)) -ge 15 ] \
    || fail "the dark wire's pixels are rgb($DARK_R, $DARK_G, $DARK_B): its hue was lost"

if [ "$SELFTEST" = 1 ]; then
    echo "== SELFTEST: invert the brightness comparison (the dark frame must be brighter) ==" >&2
    [ $((DARK_G - LIT_G)) -ge "$MEAN_MARGIN" ] \
        || fail "SELFTEST must not pass: lit mean green $LIT_G, dark mean green $DARK_G (the dark frame is dimmer by $((LIT_G - DARK_G)))"
    fail "SELFTEST passed the inverted comparison: the dark wire is not dimmer"
fi

[ $((LIT_G - DARK_G)) -ge "$MEAN_MARGIN" ] \
    || fail "the lit wire is not measurably brighter: lit mean green $LIT_G, dark mean green $DARK_G (need >= $MEAN_MARGIN)"

echo "PASS: the festive wire follows the lightmap — lit mean rgb($LIT_R, $LIT_G, $LIT_B) over $LIT green pixels, dark mean rgb($DARK_R, $DARK_G, $DARK_B) over $DARK green pixels (dark green is $((LIT_G - DARK_G)) levels lower)"
