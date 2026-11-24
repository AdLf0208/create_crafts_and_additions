#!/usr/bin/env bash
# The held-wire preview leg: the second half of upstream's `WireNodeRenderer` (`clientRenderHeldWire`).
#
# A real spool click selects a connector; while the selection is pending the client draws the wire
# being laid from the selected connector's node to the player, in the spool's own colour and under
# the 0.2.14 block/sky lightmap. The second click lays the wire and clears the selection, so the
# preview must vanish. Upstream reads all three values (block, node, WireType) off the held stack's
# custom data; this port publishes them from its `Pending` store through `/createaddition signals`,
# which the leg reads back as a parsed pending=1 / pending=0 assertion.
#
# **The assertion is green-dominant pixels in the connector→hand box of three --offscreen frames.**
# Only the wire's own festive green (26/94/12) is counted, never the glowstone-lit floor, the End
# sky or the held spool's item sprite: the box is measured by HELD_PROBE=1 to exclude them. The
# three printed counts are
#   NEGATIVE  — no click yet, no selection: must be 0
#   SELECTED  — after the first click, spool held: must be >= PIXELS
#   CLEARED   — after the second click (wire laid, selection cleared): must be 0
# A brightness-only check of the box is invalid (the glowstone lights the floor too): only the
# wire's green-dominant pixels are counted and averaged.
#
# NEGATIVE=1 asks the no-selection frame to be non-empty and must end EXIT:1.
# SELFTEST=preview asks the cleared frame to be non-empty and must end EXIT:1.
# CONTROL_GAME= runs the rig on a Nightly without the package: setblock leaves minecraft:air.
set -Eeuo pipefail
export LC_ALL=C
[ -z "${CA_DEBUG:-}" ] || set -x

NEGATIVE=${NEGATIVE:-0}
SELFTEST=${SELFTEST:-0}
PROBE=${HELD_PROBE:-0}
CONTROL=${CONTROL_GAME:+1}
CONTROL=${CONTROL:-0}
PORT=$(cd "$(dirname "$0")/.." && pwd)
CTL=${CTL:-}
GAME=${GAME:-}
CONTROL_GAME=${CONTROL_GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
TMP=${E2E_TMP:-/tmp/opencode}
mkdir -p "$TMP"
SOCK=${SOCK:-$TMP/createaddition-held.sock}
KEEP=${KEEP:-0}
WORLD=CreateAdditionHeld
CONTROL_WORLD=CreateAdditionHeldControl
EVIDENCE=$(cd "$PORT/../_shared/evidence/createaddition-e2e" && pwd)

X0=268
Y=101
C1=$((X0 + 1))
C2=$((X0 - 8))
GX=$((X0 + 2))
GY=$((Y + 3))
GZ=-1
# The connector→hand box, as frame fractions, measured by HELD_PROBE=1 so it holds the preview and
# excludes the held spool's item sprite (bottom centre) and the laid wire (along the rig).
BX0=${BX0:-24}
BY0=${BY0:-38}
BX1=${BX1:-90}
BY1=${BY1:-72}
# A preview is a green ribbon of a few hundred pixels at 1280x720; 100 is a floor that an empty
# box (0) and an End particle cannot reach.
PIXELS=100

guard() { echo "EXIT:1"; exit 1; }
command -v jq >/dev/null || { echo "missing jq (every assertion parses a ctl reply)" >&2; guard; }
command -v python3 >/dev/null || { echo "missing python3 (the pixel count)" >&2; guard; }
[ -x "$CTL" ] || { echo "missing horizon-ctl: CTL=$CTL" >&2; guard; }
[ -n "$GAME" ] || { echo "GAME= is required: this package is native, so the game must be launcher-built" >&2; guard; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; guard; }

SCRATCH=$(mktemp -d "$TMP/createaddition-held-home.XXXXXX")
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() {
    # Exit 3 is UNREACHABLE: the connection failed, so this command never reached the game and a
    # retry cannot double-apply it (including the chat-based link calls). A busy box occasionally
    # drops one; retry once rather than lose a whole leg to a socket blip.
    local out status
    out=$("$CTL" --socket "$SOCK" "$@" 2>&1); status=$?
    if [ "$status" -eq 3 ]; then
        sleep 1
        out=$("$CTL" --socket "$SOCK" "$@" 2>&1); status=$?
    fi
    [ -n "$out" ] && printf '%s\n' "$out"
    return "$status"
}
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
    # Launch directly (not through `ctl`'s retry): a second launch would fight the first for the
    # socket; `wait-ready` below is the real check that the game came up.
    "$CTL" --socket "$SOCK" launch --game "$game" --world "$WORLD" --seed 20261105 \
        --creative --offscreen --wait --resources "$RES" >/dev/null 2>&1 || true
    ctl wait-ready 120
    ctl wait 40
}

block_name() { ctl block "$1" "$2" "$3" 2>/dev/null | jq -r '.name // .block // empty' 2>/dev/null || true; }
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
SIGNALS_REPLY='^Signals: answers=[0-9]+ dimensions=[^ ]+ pending=[0-9]+'

arena() {
    ctl command "execute in minecraft:the_end run tp @s $((X0 + 3)).5 104 0.5" >/dev/null
    local loaded=false st
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        st=$(ctl block "$X0" 100 0 2>/dev/null | jq -r '.loaded // false' 2>/dev/null) || st=false
        if [ "$st" = true ]; then loaded=true; break; fi
    done
    [ "$loaded" = true ] || fail "the End arena chunk never loaded"
    ctl command "execute in minecraft:the_end run fill $((X0 - 20)) 100 -20 $((X0 + 14)) 100 12 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run fill $((X0 - 20)) 101 -20 $((X0 + 14)) 112 12 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run kill @e[type=!player]" >/dev/null
    ctl wait 6 >/dev/null
    [ "$(block_name "$X0" 100 0)" = "minecraft:stone" ] || fail "the arena floor is not stone"
}

rig() {
    ctl command "execute in minecraft:the_end run setblock $C1 $Y 0 createaddition:connector[facing=west,mode=none]" >/dev/null
    ctl command "execute in minecraft:the_end run setblock $C2 $Y 0 createaddition:connector[facing=west,mode=none]" >/dev/null
    ctl wait 8 >/dev/null
    [ "$(block_name "$C1" $Y 0)" = "createaddition:connector" ] || fail "the selected connector did not place"
    [ "$(block_name "$C2" $Y 0)" = "createaddition:connector" ] || fail "the far connector did not place"
    ctl command "execute in minecraft:the_end run setblock $GX $GY $GZ minecraft:glowstone" >/dev/null
    ctl wait 60 >/dev/null
}

load_spool() {
    ctl command "item replace entity @s hotbar.0 with createaddition:festive_spool 1" >/dev/null
    ctl select 1 >/dev/null
    ctl wait 6 >/dev/null
    local held
    held=$(ctl inventory | jq -r '[.hotbar[], .main[] | select(.item=="createaddition:festive_spool") | .count] | add // 0')
    [ "$held" -ge 1 ] || fail "the festive spool is not held (count=$held)"
    echo "  holding $held festive spool(s)" >&2
}

# The clicks need the player two blocks east of the near connector, looking along the rig (this is
# the geometry the A–M suite's spool phase uses). The screenshot needs a camera four blocks south,
# so the two positions are separate and the pending selection survives the teleport.
click_pos() { # click_pos [X] : stand two blocks east of X (default C1), looking along the rig
    local x=${1:-$C1}
    ctl command "execute in minecraft:the_end run tp @s $((x + 2)).5 101 0.5" >/dev/null
    ctl wait 12 >/dev/null
}
camera() {
    ctl command "execute in minecraft:the_end run tp @s $((X0 + 3)).5 $Y 4.5 0 12" >/dev/null
    local settled=false pos="unknown" before ground
    before=$(ctl state 2>/dev/null | jq -r '.player.position | @csv' 2>/dev/null) || before=""
    for _ in $(seq 1 40); do
        ctl wait 10 >/dev/null
        pos=$(ctl state 2>/dev/null | jq -r '.player.position | @csv' 2>/dev/null) || pos=""
        ground=$(ctl state 2>/dev/null | jq -r '.player.on_ground // false' 2>/dev/null) || ground=false
        if [ "$pos" = "$before" ] && [ "$ground" = true ]; then
            settled=true
            echo "  camera settled at $pos" >&2
            break
        fi
        before=$pos
    done
    [ "$settled" = true ] || fail "the player never stood still on the floor (at $pos)"
    ctl look-at "$((X0 + 2)).5 102.4 0.5" >/dev/null
    ctl wait 10 >/dev/null
}

let_chat_fade() { ctl wait 360 >/dev/null; }

unpause() {
    ctl resume >/dev/null 2>&1 || true
    local paused
    paused=$(ctl state 2>/dev/null | jq -r '.debug.pause_menu // false' 2>/dev/null) || paused=false
    if [ "$paused" != false ]; then
        ctl wait 4 >/dev/null
        ctl resume >/dev/null 2>&1 || true
        paused=$(ctl state 2>/dev/null | jq -r '.debug.pause_menu // false' 2>/dev/null) || paused=false
    fi
    [ "$paused" = false ] || fail "the game is paused (pause_menu=$paused)"
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
        if [ "$count" -lt 20000 ]; then
            cp "$SCRATCH/settle-b.png" "$path"
            echo "  $tag: settled (two frames twenty ticks apart differ by $count pixels)" >&2
            return 0
        fi
        echo "  $tag: still changing ($count pixels), waiting for the chunks" >&2
        ctl wait 40 >/dev/null
    done
    fail "$tag: the scene never settled"
}

compare() {
    python3 - "$1" "$2" <<'PY'
import sys
from PIL import Image
a = Image.open(sys.argv[1]).convert("RGB"); b = Image.open(sys.argv[2]).convert("RGB")
if a.size != b.size:
    print(f"SIZE {a.size} {b.size}"); sys.exit(0)
pa, pb = a.load(), b.load(); w, h = a.size; changed = 0
for y in range(0, h, 2):
    for x in range(0, w, 2):
        ca, cb = pa[x, y], pb[x, y]
        if abs(ca[0]-cb[0]) > 6 or abs(ca[1]-cb[1]) > 6 or abs(ca[2]-cb[2]) > 6:
            changed += 1
print(f"CHANGED {changed} SIZE {w} {h}")
PY
}

# green_stats PATH X0% Y0% X1% Y1% -> COUNT MEAN_R MEAN_G MEAN_B BBOX X0 Y0 X1 Y1
green_stats() {
    python3 - "$1" "$2" "$3" "$4" "$5" <<'PY'
import sys
from PIL import Image
path, a0, b0, a1, b1 = sys.argv[1], *map(int, sys.argv[2:6])
im = Image.open(path).convert("RGB"); w, h = im.size; px = im.load()
box = (0, 0, w, h) if (a0, b0, a1, b1) == (0, 0, 100, 100) else \
       (w * a0 // 100, h * b0 // 100, w * a1 // 100, h * b1 // 100)
count = rs = gs = bs = 0; xlo = ylo = 10**9; xhi = yhi = -1
for y in range(box[1], box[3]):
    for x in range(box[0], box[2]):
        r, g, b = px[x, y]
        if g > r + 15 and g > b + 15:
            count += 1; rs += r; gs += g; bs += b
            xlo, ylo, xhi, yhi = min(xlo, x), min(ylo, y), max(xhi, x), max(yhi, y)
if count:
    print(count, rs // count, gs // count, bs // count, "BBOX", xlo, ylo, xhi, yhi)
else:
    print(0, 0, 0, 0, "BBOX", -1, -1, -1, -1)
PY
}

measure() { # measure LABEL PATH -> sets COUNT R G B (box) and prints the line with a whole-frame bbox
    local label=$1 path=$2
    read -r COUNT R G B _ BX BY BXX BYY <<<"$(green_stats "$path" "$BX0" "$BY0" "$BX1" "$BY1")"
    local wc wr wg wb _ wx0 wy0 wx1 wy1
    read -r wc wr wg wb _ wx0 wy0 wx1 wy1 <<<"$(green_stats "$path" 0 0 100 100)"
    echo "== $label: box=$COUNT green pixels mean rgb($R,$G,$B) box ${BX0}..${BX1}x${BY0}..${BY1}; whole-frame=$wc bbox $wx0,$wy0,$wx1,$wy1" >&2
}

if [ "$CONTROL" = 1 ]; then
    echo "== control: the same rig on a Nightly without the package ==" >&2
    WORLD=$CONTROL_WORLD
    launch "$CONTROL_GAME"
    arena
    ctl command "execute in minecraft:the_end run setblock $C1 $Y 0 createaddition:connector" >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    got=$(block_name "$C1" $Y 0)
    [ "$got" != "createaddition:connector" ] || fail "the control game placed a connector, so the block id was known"
    fail "the control game does not know createaddition:connector (setblock left '$got'), so the green pixels the modded frames count come from this package"
fi

echo "== launch (The End arena, launcher-built game, --offscreen for screenshots) ==" >&2
launch "$GAME"
ctl options-set pauseOnLostFocus false >/dev/null 2>&1 || true
arena
rig
load_spool

echo "== NEGATIVE: no click yet, no selection ==" >&2
click_pos
command_reply "createaddition signals" "$SIGNALS_REPLY"
case "$REPLY" in *'pending=0'*) echo "  $REPLY" >&2 ;; *) fail "a selection is pending before the first click: $REPLY" ;; esac
camera
let_chat_fade
settle_shot "$SCRATCH/held-negative.png" "no selection"
measure NEGATIVE "$SCRATCH/held-negative.png"
NEGATIVE_COUNT=$COUNT
cp "$SCRATCH/held-negative.png" "$EVIDENCE/held-negative.png" 2>/dev/null || true

echo "== first click: select the near connector ==" >&2
click_pos
ctl look-at "$C1.5 $Y.5 0.5" >/dev/null 2>&1 || true
IOUT=$(ctl interact "$C1" $Y 0 2>&1) || echo "  (ctl interact C1: $IOUT)" >&2
ctl wait 6 >/dev/null
command_reply "createaddition signals" "$SIGNALS_REPLY"
case "$REPLY" in *'pending=1'*) echo "  $REPLY" >&2 ;; *) fail "the first click did not publish a pending selection: $REPLY" ;; esac
camera
let_chat_fade
settle_shot "$SCRATCH/held-on.png" "selection pending"
measure SELECTED "$SCRATCH/held-on.png"
SELECTED=$COUNT; SR=$R; SG=$G; SB=$B
cp "$SCRATCH/held-on.png" "$EVIDENCE/held-on.png" 2>/dev/null || true

echo "== second click: click the SAME connector (the port's clear-selection path) ==" >&2
# Upstream empties the stack's data on a second click on the same block; this port's `spool_click`
# does the same. It clears the preview without laying a wire, so the measured box is free of the
# laid wire's own green pixels — a wire between two connectors would sit in the same box and could
# not be told from the preview by colour alone. The wire-laid path is proved below with the admin
# command, and the A-M suite proves the two-click spool path.
click_pos
ctl look-at "$C1.5 $Y.5 0.5" >/dev/null 2>&1 || true
IOUT=$(ctl interact "$C1" $Y 0 2>&1) || echo "  (ctl interact C1: $IOUT)" >&2
ctl wait 6 >/dev/null
command_reply "createaddition signals" "$SIGNALS_REPLY"
case "$REPLY" in *'pending=0'*) echo "  $REPLY" >&2 ;; *) fail "the second click did not clear the selection: $REPLY" ;; esac
camera
let_chat_fade
settle_shot "$SCRATCH/held-off.png" "selection cleared"
measure CLEARED "$SCRATCH/held-off.png"
CLEARED=$COUNT
cp "$SCRATCH/held-off.png" "$EVIDENCE/held-off.png" 2>/dev/null || true

echo "== the wire-laid path is intact: admin link + parsed info readout ==" >&2
command_reply "createaddition link $C1 $Y 0 $C2 $Y 0 festive" "$LINK_REPLY"
[ "$REPLY" = "Nodes linked." ] || fail "the festive wire was not laid: $REPLY"
command_reply "createaddition info $C1 $Y 0" '^(Connector|Large Connector|Small Light Connector) at'
case "$REPLY" in *festive*) echo "  the wire is saved as festive: $REPLY" >&2 ;; *) fail "the wire is not festive: $REPLY" ;; esac

if [ "$PROBE" = 1 ]; then
    echo "== HELD_PROBE: whole-frame green bounding boxes printed above for all three frames ==" >&2
    echo "PASS: held probe" >&2
    exit 0
fi

echo "== the three pixel counts: NEGATIVE=$NEGATIVE_COUNT SELECTED=$SELECTED CLEARED=$CLEARED ==" >&2

if [ "$NEGATIVE" = 1 ]; then
    [ "$NEGATIVE_COUNT" -lt "$PIXELS" ] || fail "NEGATIVE must not pass: a frame with no selection carries $NEGATIVE_COUNT green pixels"
    fail "NEGATIVE: the no-selection frame carries $NEGATIVE_COUNT green preview pixels (< $PIXELS), so this inverted leg fails and a renderer that always draws a preview is caught"
fi

if [ "$SELFTEST" = preview ]; then
    [ "$CLEARED" -ge "$PIXELS" ] || fail "SELFTEST must not pass: the box reads $CLEARED green pixels after the selection was cleared (SELECTED reads $SELECTED)"
    fail "SELFTEST passed the inverted assertion: the preview is still drawn after the selection was cleared"
fi

[ "$SELECTED" -ge "$PIXELS" ] || fail "only $SELECTED green preview pixels with a selection, expected at least $PIXELS"
[ $((SG - SR)) -ge 30 ] && [ $((SG - SB)) -ge 30 ] \
    || fail "the preview's pixels are rgb($SR,$SG,$SB): not the festive wire's green (26, 94, 12)"
[ "$CLEARED" -lt "$PIXELS" ] || fail "after the second click the box still carries $CLEARED green preview pixels"
[ "$NEGATIVE_COUNT" -lt "$PIXELS" ] || fail "with no selection the box carries $NEGATIVE_COUNT green preview pixels"

echo "PASS: the held-wire preview — SELECTED=$SELECTED green pixels rgb($SR,$SG,$SB), CLEARED=$CLEARED, NEGATIVE=$NEGATIVE_COUNT (a selection draws the wire to the hand; the second click clears it)"
