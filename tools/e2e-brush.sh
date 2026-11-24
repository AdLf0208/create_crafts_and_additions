#!/usr/bin/env bash
# The brush-detach leg: what Create's brush (the jar's shears path) does to a connector's wires.
#
# Upstream (`AbstractConnectorBlock.useItemOn`, the `Items.SHEARS` branch) takes every wire out of
# a connector that `hasAnyConnection()` and drops each on the ground — the **level** path
# `IWireNode.dropWires(Level, boolean)` → `WireType.getDrop()`: `CAItems.<KIND>_WIRE.asStack(4)`,
# and `FESTIVE`'s drop is a **copper** wire. The block stays; its peer's reciprocal node goes without
# a second drop. A brush on an empty connector is a no-op. Create's brush is not installed here, so
# the route is exposed as `/createaddition brush <pos>` — the same command stand-in as `/mode` for
# Create's wrench.
#
# The assertion is the **parsed ground-item count**, not the command's text: in the End arena (no
# ambient item entities) a connector seeded with one festive wire must read `links []` after the
# brush and leave exactly four `createaddition:copper_wire` item entities on the ground; an empty
# connector leaves 0. `BRUSH_SELFTEST=1` inverts the four-count and must end EXIT:1.
#
# `CONTROL_GAME=` runs the same rig on the Nightly without the package: `createaddition:connector`
# is unknown there, the rig cannot be built, and the leg fails.
#
# Traps already paid for: item entities are read with `.pos` (not `.position`); the arena's chunk is
# loaded before `fill`; every reply is tagged so an older identical reply cannot be read as this one's;
# and E2E_TMP (not /tmp) is the scratch root.
set -Eeuo pipefail
export LC_ALL=C
[ -z "${CA_DEBUG:-}" ] || set -x

SELFTEST=${BRUSH_SELFTEST:-0}
CONTROL=${CONTROL_GAME:+1}
CONTROL=${CONTROL:-0}
PORT=$(cd "$(dirname "$0")/.." && pwd)
CTL=${CTL:-}
GAME=${GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
TMP=${E2E_TMP:-/tmp/opencode}
mkdir -p "$TMP"
SOCK=${SOCK:-$TMP/createaddition-brush.sock}
KEEP=${KEEP:-0}
WORLD=CreateAdditionBrush
CONTROL_WORLD=CreateAdditionBrushControl

Y=101
Z=0
X0=400
GAP=12

guard() { echo "EXIT:1"; exit 1; }
command -v jq >/dev/null || { echo "missing jq (every assertion parses a ctl reply)" >&2; guard; }
[ -x "$CTL" ] || { echo "missing horizon-ctl: CTL=$CTL" >&2; guard; }
[ -n "$GAME" ] || {
    echo "GAME= is required: this package is native, so the game must be launcher-built" >&2
    guard
}
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; guard; }

SCRATCH=$(mktemp -d "$TMP/createaddition-brush-home.XXXXXX")
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

launch() { # launch BINARY WORLD
    local game=$1 world=$2
    rm -f "$SOCK"
    cd "$SCRATCH"
    "$CTL" --socket "$SOCK" launch --game "$game" --world "$world" --seed 20261105 \
        --creative --headless --wait --resources "$RES" >/dev/null
    ctl wait-ready 120
    ctl wait 40
}

block_name() { ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty'; }

# Every reply is tagged: a reply to an earlier command can never be read as this one's.
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
BRUSH_REPLY='^(Wires detached\.|No wires to detach\.)'
INFO_REPLY='^(Connector|Large Connector|Small Light Connector|Redstone Relay) at'

# The End arena: no ambient item entities, so a ground count is exactly what the mod dropped.
# The chunk must be loaded before a `fill` in it does anything, so the teleport comes first.
arena() {
    ctl command "execute in minecraft:the_end run tp @s $((X0 + 2)).5 104 0.5" >/dev/null
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(ctl block "$X0" 100 0 | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the End arena chunk never loaded"
    ctl command "execute in minecraft:the_end run fill $((X0 - 6)) 100 -6 $((X0 + 60)) 100 6 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run fill $((X0 - 6)) 101 -6 $((X0 + 60)) 112 6 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    clear_items
    [ "$(block_name "$X0" 100 0)" = "minecraft:stone" ] || fail "the arena floor is not stone"
    [ "$(block_name "$X0" 101 0)" = "minecraft:air" ] || fail "the air above the floor is not air"
}

# Clear every item entity in the arena before a phase. Pickup delay on a freshly dropped wire is
# 10 ticks, so the count is read well before the player could take anything.
clear_items() {
    ctl command "execute in minecraft:the_end run kill @e[type=minecraft:item]" >/dev/null
    ctl wait 4 >/dev/null
    local n
    n=$(ground_wires)
    [ "$n" = 0 ] || fail "the arena still has $n ground wires after clearing"
}

# The exact number of ground item entities of a kind near the connector. Item entities report
# `.pos` at this pin (not `.position`); the drop is within a quarter block of the connector.
ground_at() { # ground_at ITEM X
    local item=$1 x=$2
    ctl entities 16 minecraft:item | jq -r --arg item "$item" --argjson x "$x" \
        '[.entities[]
          | select(.kind == "item" and .item.item == $item)
          | select(((.pos // .position)[0] - $x) < 3 and ((.pos // .position)[0] - $x) > -3)]
         | map(.item.count) | add // 0'
}
ground_wires() { ground_at "createaddition:copper_wire" "$X0"; }

# A wire pair four blocks apart, so `/createaddition link` accepts one festive wire.
wire_rig() { # wire_rig NEAR FAR
    local near=$1 far=$2
    ctl command "execute in minecraft:the_end run setblock $near $Y $Z createaddition:connector[facing=west,mode=none]" >/dev/null
    ctl wait 4 >/dev/null
    ctl command "execute in minecraft:the_end run setblock $far $Y $Z createaddition:connector[facing=east,mode=none]" >/dev/null
    ctl wait 4 >/dev/null
    [ "$(block_name "$near" $Y $Z)" = "createaddition:connector" ] \
        || fail "the near connector did not place at $near,$Y,$Z (got '$(block_name "$near" $Y $Z)')"
    [ "$(block_name "$far" $Y $Z)" = "createaddition:connector" ] \
        || fail "the far connector did not place at $far,$Y,$Z (got '$(block_name "$far" $Y $Z)')"
}

if [ "$CONTROL" = 1 ]; then
    echo "== control: the same rig on a Nightly without the package =="
    launch "$CONTROL_GAME" "$CONTROL_WORLD"
    arena
    ctl command "execute in minecraft:the_end run setblock $X0 $Y $Z createaddition:connector" >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    got=$(block_name "$X0" $Y $Z)
    [ "$got" != "createaddition:connector" ] \
        || fail "the control game placed a connector, so the block id was known"
    fail "the control game does not know createaddition:connector (the setblock left '$got'), so no wire can be brushed off: the four copper wires counted in the default leg come from this package"
fi

echo "== launch (The End arena, launcher-built game) =="
launch "$GAME" "$WORLD"
arena

# --- Phase 1: one festive wire, detach with the brush -----------------------
NEAR=$X0
FAR=$((X0 + 4))
echo "== seed one festive wire between $NEAR and $FAR, then brush $NEAR =="
wire_rig "$NEAR" "$FAR"
command_reply "createaddition link $NEAR $Y $Z $FAR $Y $Z festive" "$LINK_REPLY"
[ "$REPLY" = "Nodes linked." ] || fail "the festive wire was not laid: $REPLY"
command_reply "createaddition info $NEAR $Y $Z" "$INFO_REPLY"
case "$REPLY" in
    *"/festive]"*|*"/festive),"*) echo "  wired connector: $REPLY" >&2 ;;
    *) fail "the connector does not hold the festive wire: $REPLY" ;;
esac
clear_items

command_reply "createaddition brush $NEAR $Y $Z" "$BRUSH_REPLY"
[ "$REPLY" = "Wires detached." ] || fail "the brush did not detach the wire: $REPLY"
command_reply "createaddition info $NEAR $Y $Z" "$INFO_REPLY"
case "$REPLY" in
    *"links []"*) echo "  brushed connector: $REPLY" >&2 ;;
    *) fail "the brushed connector still holds a wire: $REPLY" ;;
esac
command_reply "createaddition info $FAR $Y $Z" "$INFO_REPLY"
case "$REPLY" in
    *"links []"*) echo "  peer: $REPLY" >&2 ;;
    *) fail "the peer still names the brushed connector: $REPLY" ;;
esac
FOUR=$(ground_wires)
echo "  ground createaddition:copper_wire after the brush: $FOUR (pos-filtered)" >&2
[ "$FOUR" = 4 ] || fail "a detached festive wire dropped $FOUR copper wires, expected exactly 4"

if [ "$SELFTEST" = 1 ]; then
    # The counterfactual: invert the count the default leg asserts. A wrong package that drops
    # nothing (the pre-slice behaviour) makes this pass; the correct one makes it exit 1.
    [ "$FOUR" -ne 4 ] || fail "SELFTEST: the brush dropped exactly $FOUR copper wires (4), so the inverted assertion cannot fail — the default leg's count is not a real assertion"
fi

# --- Phase 2: an empty connector drops nothing ------------------------------
EMPTY=$((X0 + 8))
clear_items
echo "== brush an empty connector at $EMPTY (drops 0) =="
ctl command "execute in minecraft:the_end run setblock $EMPTY $Y $Z createaddition:connector[facing=west,mode=none]" >/dev/null
ctl wait 4 >/dev/null
[ "$(block_name "$EMPTY" $Y $Z)" = "createaddition:connector" ] || fail "the empty connector did not place"
command_reply "createaddition info $EMPTY $Y $Z" "$INFO_REPLY"
case "$REPLY" in
    *"links []"*) echo "  empty connector: $REPLY" >&2 ;;
    *) fail "the empty connector was not empty: $REPLY" ;;
esac
command_reply "createaddition brush $EMPTY $Y $Z" "$BRUSH_REPLY"
[ "$REPLY" = "No wires to detach." ] || fail "a brush on an empty connector was not a no-op: $REPLY"
ZERO=$(ground_wires)
echo "  ground createaddition:copper_wire after brushing the empty connector: $ZERO" >&2
[ "$ZERO" = 0 ] || fail "brushing an empty connector dropped $ZERO wires, expected 0"

if [ "$SELFTEST" = brush ]; then
    # The brief's counterfactual: invert the empty-connector count. It is 0 in the correct package,
    # so this leg must exit 1 — exactly what makes the phase-2 assertion fail-capable.
    [ "$ZERO" -ne 0 ] || fail "SELFTEST=brush: the empty connector dropped $ZERO wires (0), so the inverted assertion cannot fail"
fi

echo "PASS: a brush detaches one festive wire as exactly four ground createaddition:copper_wire items, leaves links [] on both connectors, and an empty connector drops 0"
