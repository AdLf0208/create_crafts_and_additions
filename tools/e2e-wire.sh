#!/usr/bin/env bash
# The wire renderer's in-game leg: the **client half** of the wire family.
#
# Upstream draws every laid wire in `WireNodeRenderer` — a ribbon hung between the two nodes on
# the jar's catenary, in the `WireType`'s RGB, and for the festive wire a string of small lights.
# Until 0.2.9 this port drew nothing: a laid wire moved energy and was invisible. The renderer is
# a `[[native.mixin]]` HEAD patch of `horizon_mod_render::render::world_drawn` (a native-route
# place, not a `mod_api` hook) plus one of `render_needs`, and this leg is its proof.
#
# **The assertion is a screenshot differential in the wire's own colour.** Two frames of the same
# scene, the same camera and the same tick window, taken as PNG by `horizon-ctl screenshot` on an
# `--offscreen` game:
#
#   with  — the festive wire laid between two connectors, drawn by the package;
#   after — the same frame once `/createaddition unlink` has taken the wire out.
#
# The wire is `WireType.FESTIVE`'s rgb(26, 94, 12), shaded 0.7 to 1.0 by the jar's own
# alternating points, and nothing else in the End's void and the stone floor is green-dominant.
# So the leg counts the green pixels of the wire's screen box in each frame: >= 300 with the wire,
# <= 60 without it, and the mean of the drawn pixels green by >= 30 levels on both other channels.
# A renderer that draws nothing leaves both frames without a single green pixel and the leg
# fails; a wire drawn in the wrong colour fails the mean.
#
# A raw frame difference is **not** the assertion: the engine's own brightness shifts the floor's
# tone by a few levels between two shots (measured: 25,600 changed pixels that were all lighting),
# which is why the count is by colour and the difference is only reported.
#
# `NEGATIVE=1` is the counterfactual: the whole run happens, then the positive count is asked of
# the frame **without** the wire — the frame a renderer that draws nothing produces — and the leg
# MUST fail there. Its `EXIT:1` is the proof the default leg's PASS is not vacuous.
#
# `CONTROL_GAME=` runs the setup on a Nightly without the package: `createaddition:*` is unknown
# there, the rig cannot be built, and the leg fails.
#
# The rig (all in The End, on the arena's stone floor):
#   accumulator(268) – connector(269, facing west) – festive wire – connector(273, facing east) –
#   accumulator(274), all at y=101; the player stands four blocks south on the floor, at eye
#   height, looking at the point the wire hangs from.
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
SOCK=${SOCK:-$TMP/createaddition-wire.sock}
WCROP=${WCROP:-wirebox}
KEEP=${KEEP:-0}
WORLD=CreateAdditionWire
CONTROL_WORLD=CreateAdditionWireControl

X0=268
Y=101
AX=$X0
C1=$((X0 + 1))
C2=$((X0 + 5))
B=$((X0 + 6))
# Green pixels the wire's box must carry with the wire laid: the wire is ~2,000 pixels of green
# at 1280x720, so 300 is a generous floor that an undrawn wire (0) cannot reach.
PIXELS=300
# Green pixels the same box may carry with the wire taken out: the End's terrain and the stone
# floor are yellow-grey, so this is a stray antialiased pixel or two, not a wire.
LIMIT=60
# A changed pixel differs in at least one channel by this much (the reported frame difference).
DELTA=6
NOISE=50

guard() {
    echo "EXIT:1"
    exit 1
}
command -v jq >/dev/null || { echo "missing jq (every assertion parses a ctl reply)" >&2; guard; }
command -v python3 >/dev/null || { echo "missing python3 (the pixel count)" >&2; guard; }
[ -x "$CTL" ] || { echo "missing horizon-ctl: CTL=$CTL" >&2; guard; }
[ -n "$GAME" ] || {
    echo "GAME= is required: this package is native, so the game must be launcher-built" >&2
    guard
}
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; guard; }

SCRATCH=$(mktemp -d "$TMP/createaddition-wire-home.XXXXXX")
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
fail() {
    echo "FAIL: $*" >&2
    recent_chat >&2 || true
    ctl logs --lines 40 >&2 || true
    exit 1
}
on_error() { echo "FAIL: line $1: '$2' exited $?" >&2; ctl logs --lines 40 >&2 || true; }
trap 'on_error "$LINENO" "$BASH_COMMAND"' ERR

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
    # Last line of every run: `check-e2e-logs.py` reads it as the verdict.
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() { # launch BINARY
    local game=$1
    rm -f "$SOCK"
    cd "$SCRATCH"
    # `--offscreen`, not `--headless`: screenshots need a renderer. No window is shown.
    "$CTL" --socket "$SOCK" launch --game "$game" --world "$WORLD" --seed 20261105 \
        --creative --offscreen --wait --resources "$RES" >/dev/null
    ctl wait-ready 120
    ctl wait 40
}

block_name() { ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty'; }

# Every reply is tagged, as the main leg's are: a reply to an earlier command can never be read
# as this one's answer.
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
LINK_REPLY='^(Nodes linked\.|The connectors are too far apart for this wire\.|Every node of this connector has a wire\.|These connectors are already linked\.|A connector cannot be linked to itself\.|Two large connectors need a gold or electrum wire\.)'
WIRE_REPLY='^(Wire removed\.|These connectors are not linked\.)'

# The pad: a stone floor under the rig, air above it, no mobs. The chunk has to be loaded before
# a `fill` in it does anything (a `fill` in an unloaded chunk is a no-op that reports success).
arena() {
    ctl command "execute in minecraft:the_end run tp @s $((X0 + 3)).5 104 0.5" >/dev/null
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(ctl block "$X0" 100 0 | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the End arena chunk never loaded"
    ctl command "execute in minecraft:the_end run fill $((X0 - 8)) 100 -10 $((X0 + 14)) 100 10 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run fill $((X0 - 8)) 101 -10 $((X0 + 14)) 112 10 minecraft:air" >/dev/null
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

# The camera: feet on the floor (the stone at y=100 has its top at y=101), four blocks south of
# the wire, at eye height, looking at the point the wire hangs from (a connector's node is 3/16
# of a block above its centre). A player left falling moves the camera between the two shots, and
# a moved camera moves the whole scene — the first run of this leg measured 45,786 changed pixels
# of floor for that reason, not a wire. So wait until the position is the same over ten ticks.
camera() {
    ctl command "execute in minecraft:the_end run tp @s $((X0 + 3)).5 $Y 4.5 0 12" >/dev/null
    local settled=false pos="unknown" before
    before=$(ctl state | jq -r '.player.position | @csv')
    for _ in $(seq 1 40); do
        ctl wait 10 >/dev/null
        pos=$(ctl state | jq -r '.player.position | @csv')
        if [ "$pos" = "$before" ] \
            && [ "$(ctl state | jq -r '.player.on_ground // false')" = true ]; then
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

# The chat log is drawn over the top-left of the frame and its text changes with every command
# this leg types: a measurement that includes it is a measurement of the chat (the first run of
# this leg counted 86,413 changed pixels that were all the chat panel). Vanilla keeps a line at
# full opacity for 200 ticks and fades it after that, so the frames are taken once the log has
# been empty long enough to be gone. The wire box also starts below the log, so both guards hold.
let_chat_fade() {
    ctl wait 360 >/dev/null
}

# A paused frame is not the world: the pause state dims the whole picture.
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

# A screenshot of a frame that has stopped changing: the End's chunks keep meshing and uploading
# for a while after the camera arrives, and a shot taken then catches terrain the next frame has.
# Two frames twenty ticks apart must agree before either is used.
settle_shot() { # settle_shot PATH TAG -> STILL
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
            STILL=$count
            echo "  $tag: settled (two frames twenty ticks apart differ by $count pixels)" >&2
            return 0
        fi
        echo "  $tag: still changing ($count pixels), waiting for the chunks" >&2
        ctl wait 40 >/dev/null
    done
    fail "$tag: the scene never settled (the last two frames differ by $count pixels)"
}

# How many pixels changed between two frames in the crop, and the mean colour of them in the first.
compare() { # compare WITH AFTER
    python3 - "$1" "$2" "$DELTA" "$WCROP" <<'PY'
import sys
from PIL import Image

with_path, after_path, delta, crop_name = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
a = Image.open(with_path).convert("RGB")
b = Image.open(after_path).convert("RGB")
if a.size != b.size:
    print(f"SIZE {a.size} {b.size}")
    sys.exit(0)
w, h = a.size
crops = {
    # The wire's own screen box: below the chat log and above the hotbar and the hand.
    "wirebox": (w * 20 // 100, h * 60 // 100, w * 80 // 100, h * 84 // 100),
    "central": (w * 25 // 100, h * 25 // 100, w * 75 // 100, h * 65 // 100),
    "full": (0, 0, w, h),
}
box = crops[crop_name]
pa, pb = a.load(), b.load()
changed = 0
rs = gs = bs = 0
x0 = y0 = 10 ** 9
x1 = y1 = -1
for y in range(box[1], box[3]):
    for x in range(box[0], box[2]):
        ca, cb = pa[x, y], pb[x, y]
        if abs(ca[0] - cb[0]) > delta or abs(ca[1] - cb[1]) > delta or abs(ca[2] - cb[2]) > delta:
            changed += 1
            r, g, bl = ca
            rs += r
            gs += g
            bs += bl
            x0, y0, x1, y1 = min(x0, x), min(y0, y), max(x1, x), max(y1, y)
if changed:
    print(f"CHANGED {changed} MEAN {rs // changed} {gs // changed} {bs // changed} "
          f"BOX {x0} {y0} {x1} {y1} SIZE {w} {h}")
else:
    print(f"CHANGED 0 MEAN 0 0 0 BOX 0 0 0 0 SIZE {w} {h}")
PY
}

# The pixels of a frame that are the wire's green, in the wire's box. `WireType.FESTIVE` is
# rgb(26, 94, 12) and the jar shades every other point by 0.7 (rgb(18, 66, 8)): both are strongly
# green-dominant, and nothing else in this void-and-stone scene is.
green_pixels() { # green_pixels PATH -> COUNT MEAN_R MEAN_G MEAN_B
    python3 - "$1" "$WCROP" <<'PY'
import sys
from PIL import Image

path, crop_name = sys.argv[1], sys.argv[2]
im = Image.open(path).convert("RGB")
w, h = im.size
crops = {
    "wirebox": (w * 20 // 100, h * 60 // 100, w * 80 // 100, h * 84 // 100),
    "full": (0, 0, w, h),
}
box = crops[crop_name]
px = im.load()
count = 0
rs = gs = bs = 0
for y in range(box[1], box[3]):
    for x in range(box[0], box[2]):
        r, g, b = px[x, y]
        if g > r + 15 and g > b + 15:
            count += 1
            rs += r
            gs += g
            bs += b
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
    # The leg's own assertion, asked of a game that has no package: there is no connector to hang
    # a wire from, so there is no wire to draw and the leg cannot pass. The ids come from this
    # package, which is what the modded run's frames then measure.
    fail "the control game does not know createaddition:connector (the setblock left '$got'), so the rig cannot be built and no wire can be drawn: the green pixels the modded frame counts come from this package"
fi

echo "== launch (The End arena, launcher-built game, --offscreen for screenshots) =="
launch "$GAME"
# A focused window is not something an offscreen leg has: the game pauses itself when it loses
# focus and a paused frame is a picture of the pause, not the world. Turn that off for the run.
ctl options-set pauseOnLostFocus false >/dev/null 2>&1 || true
arena
rig

echo "== lay the festive wire =="
command_reply "createaddition link $C1 $Y 0 $C2 $Y 0 festive" "$LINK_REPLY"
[ "$REPLY" = "Nodes linked." ] || fail "the festive wire was not laid: $REPLY"
command_reply "createaddition info $C1 $Y 0" "^(Connector|Large Connector|Small Light Connector) at $C1,$Y,0:"
case "$REPLY" in
    *festive*) echo "  $REPLY" >&2 ;;
    *) fail "the wire is not saved as festive: $REPLY" ;;
esac

camera
let_chat_fade
settle_shot "$SCRATCH/wire-with.png" "with the wire"
read -r GREEN_WITH MEAN_R MEAN_G MEAN_B <<<"$(green_pixels "$SCRATCH/wire-with.png")"
echo "  with the wire: $GREEN_WITH green-dominant pixels in the wire box, mean rgb($MEAN_R, $MEAN_G, $MEAN_B)" >&2

echo "== take the wire out and take the same frame again =="
command_reply "createaddition unlink $C1 $Y 0 $C2 $Y 0" "$WIRE_REPLY"
[ "$REPLY" = "Wire removed." ] || fail "the wire was not removed: $REPLY"
command_reply "createaddition info $C1 $Y 0" "^(Connector|Large Connector|Small Light Connector) at $C1,$Y,0:"
case "$REPLY" in
    *"links []"*) echo "  $REPLY" >&2 ;;
    *) fail "the connector still holds a link after the unlink: $REPLY" ;;
esac
let_chat_fade
settle_shot "$SCRATCH/wire-without.png" "without the wire"
read -r GREEN_WITHOUT _ _ _ <<<"$(green_pixels "$SCRATCH/wire-without.png")"
echo "  without the wire: $GREEN_WITHOUT green-dominant pixels in the same box" >&2

# What a raw frame difference measures, for the record: it is dominated by the engine's own
# brightness between two shots (the floor's tone shifts by a few levels), which is why the
# assertion is a colour count and not a difference.
read -r _ CHANGED _ MEAN_CHANGED_R MEAN_CHANGED_G MEAN_CHANGED_B _ BOX_X0 BOX_Y0 BOX_X1 BOX_Y1 _ SIZE_W SIZE_H \
    <<<"$(compare "$SCRATCH/wire-with.png" "$SCRATCH/wire-without.png")"
echo "  (the frames differ in $CHANGED pixels of the box overall, mean rgb($MEAN_CHANGED_R, $MEAN_CHANGED_G, $MEAN_CHANGED_B): the floor's tone, not the wire)"
echo "  frame size: $SIZE_W x $SIZE_H; the changed pixels' box: x $BOX_X0..$BOX_X1, y $BOX_Y0..$BOX_Y1"

if [ "$NEGATIVE" = 1 ]; then
    # The counterfactual: the no-wire frame is the frame a renderer that draws nothing produces,
    # and the positive assertion below (>= $PIXELS green pixels) asked of it must fail. That is
    # exactly why a package which draws nothing cannot pass this leg.
    [ "$GREEN_WITHOUT" -lt "$PIXELS" ] \
        || fail "NEGATIVE must not pass: a frame without the wire carries $GREEN_WITHOUT green pixels"
    fail "NEGATIVE: the frame without the wire carries only $GREEN_WITHOUT green pixels (the wired frame carries $GREEN_WITH), so this leg fails on a renderer that draws nothing"
fi

[ "$GREEN_WITH" -ge "$PIXELS" ] \
    || fail "only $GREEN_WITH pixels of the wire box are festive green with the wire laid, expected at least $PIXELS (the renderer drew nothing, or the wire is outside the box)"
[ "$GREEN_WITHOUT" -le "$LIMIT" ] \
    || fail "the frame without the wire still carries $GREEN_WITHOUT green pixels (limit $LIMIT): something other than the wire is green"
# The jar's colour, not just "some green": festive is 26/94/12, shaded to 18/66/8, so the mean of
# the drawn pixels is green by at least 30 levels on both other channels.
[ $((MEAN_G - MEAN_R)) -ge 30 ] && [ $((MEAN_G - MEAN_B)) -ge 30 ] \
    || fail "the wire's pixels are rgb($MEAN_R, $MEAN_G, $MEAN_B): not the festive wire's green (26, 94, 12)"

if [ "$KEEP" = 1 ]; then
    cp "$SCRATCH/wire-with.png" "$SCRATCH/wire-without.png" "$PORT/target/" 2>/dev/null || true
fi

echo "PASS: the festive wire is drawn between its two connectors ($GREEN_WITH green pixels, mean rgb($MEAN_R, $MEAN_G, $MEAN_B)) and is gone ($GREEN_WITHOUT) once /createaddition unlink takes it out"
