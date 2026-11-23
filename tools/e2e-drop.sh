#!/usr/bin/env bash
# The break-drop leg: what a **broken** connector does with the wires it holds.
#
# Upstream (`IWireNode.dropWires(level, dropItems)`, from
# `AbstractConnectorBlock.playerWillDestroy`) aggregates one `ItemStack` per `WireType` it held —
# `CAItems.<KIND>_WIRE.asStack(4)`, and `FESTIVE`'s own drop is a **copper** wire — and drops each
# stack at the connector's own position, `Containers.dropContents`. A creative player's break
# (`dropWires(level, false)`) only disconnects. The block entity's `remove()` then clears the
# peer's reciprocal node without a second drop, so the peer's `/createaddition info` reads
# `links []` and nothing drops twice.
#
# The assertion is `ctl break`'s **own reply**: the agent's break task counts the item entities
# that appear within three blocks of the block centre (`drops`) and what the player took
# (`picked_up`). So the leg stands 3.5 blocks off — inside the 5-block reach, outside the pickup
# radius — breaks the far connector in survival, and reads the reply.
#
#   copper / gold / electrum / festive rig   four wire items of the kind's own drop, one connector
#   an unlinked connector                    no wire at all (the differential)
#
# `DROP_SELFTEST=1` is the counterfactual: the unlinked connector's break must have dropped a
# wire, and it did not, so the leg exits 1 — which is what proves the default leg's "no wire"
# assertion is a real assertion and not a pass that any package would get.
#
# `CONTROL_GAME=` runs the same setup on the Nightly without the package: `createaddition:connector`
# is unknown there, the rig cannot be built, and the leg fails.
#
# Traps this leg already pays for: the pad's chunk must be loaded before a `fill` in it does
# anything (a `fill` in an unloaded chunk reports success and changes nothing); the player stands
# on the floor (feet at y=101 over stone at y=100) so a break never comes from a falling player;
# and every reply is tagged, so an older identical reply can never be read as this command's.
set -Eeuo pipefail
export LC_ALL=C
[ -z "${CA_DEBUG:-}" ] || set -x

SELFTEST=${DROP_SELFTEST:-0}
CONTROL=${CONTROL_GAME:+1}
CONTROL=${CONTROL:-0}
PORT=$(cd "$(dirname "$0")/.." && pwd)
CTL=${CTL:-}
GAME=${GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
# `/tmp` is a small tmpfs on this box: honour the runner's scratch root when it sets one.
TMP=${E2E_TMP:-/tmp/opencode}
mkdir -p "$TMP"
SOCK=${SOCK:-$TMP/createaddition-drop.sock}
KEEP=${KEEP:-0}
WORLD=CreateAdditionDrop
CONTROL_WORLD=CreateAdditionDropControl

Y=101
Z=0
X0=300
GAP=10
# How far the player stands from the block being broken: inside the 5-block reach, outside the
# one-block pickup radius, so the drop is still an item entity when `ctl break` reports.
STANCE=3.5

guard() {
    echo "EXIT:1"
    exit 1
}
command -v jq >/dev/null || { echo "missing jq (every assertion parses a ctl reply)" >&2; guard; }
[ -x "$CTL" ] || { echo "missing horizon-ctl: CTL=$CTL" >&2; guard; }
[ -n "$GAME" ] || {
    echo "GAME= is required: this package is native, so the game must be launcher-built" >&2
    guard
}
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; guard; }

SCRATCH=$(mktemp -d "$TMP/createaddition-drop-home.XXXXXX")
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
INFO_REPLY='^(Connector|Large Connector|Small Light Connector|Redstone Relay) at'

# The pad: a stone floor under every rig, air above it, no mobs. The End arena's chunk has to be
# loaded before a `fill` in it does anything, so the teleport comes first and the floor is read
# back before the leg trusts it.
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
    ctl command "execute in minecraft:the_end run fill $((X0 - 6)) 100 -6 $((X0 + 40)) 100 6 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run fill $((X0 - 6)) 101 -6 $((X0 + 40)) 112 6 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run kill @e[type=!player]" >/dev/null
    ctl wait 6 >/dev/null
    [ "$(block_name "$X0" 100 0)" = "minecraft:stone" ] || fail "the arena floor is not stone"
    [ "$(block_name "$X0" 101 0)" = "minecraft:air" ] || fail "the air above the floor is not air"
}

# A pair of connectors four blocks apart, facing each other, on the floor.
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

# Feet on the floor, 3.5 blocks south of the block that is about to be broken, holding the tool the
# connector's loot table asks for: a bare hand drops nothing at all, which would make the "no wire"
# differential vacuous (the differential asserts the connector's own drop arrived).
stance() { # stance X
    local x=$1
    ctl command "item replace entity @s hotbar.0 with minecraft:iron_pickaxe 1" >/dev/null
    ctl select 1 >/dev/null
    ctl command "gamemode survival" >/dev/null
    ctl command "execute in minecraft:the_end run tp @s $x.5 $Y $STANCE" >/dev/null
    local settled=false pos=""
    for _ in $(seq 1 40); do
        ctl wait 5 >/dev/null
        pos=$(ctl state | jq -r '.player.position | @csv')
        if [ "$(ctl state | jq -r '.player.on_ground // false')" = true ]; then
            settled=true
            break
        fi
    done
    [ "$settled" = true ] || fail "the player never stood on the floor (at $pos)"
    ctl wait 4 >/dev/null
}

# Break one block and keep `ctl break`'s own reply in BREAK_JSON.
break_block() { # break_block X
    local x=$1
    BREAK=$(ctl break "$x" $Y $Z --timeout 60)
    [ "$(jq -r '.ok // false' <<<"$BREAK")" = true ] || fail "ctl break $x $Y $Z did not break: $BREAK"
    [ "$(block_name "$x" $Y $Z)" = "minecraft:air" ] \
        || fail "the block at $x,$Y,$Z is still $(block_name "$x" $Y $Z)"
    echo "  $x: $(jq -c '{drops, picked_up, ticks}' <<<"$BREAK")" >&2
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
    fail "the control game does not know createaddition:connector (the setblock left '$got'), so the rig cannot be built and a broken connector can drop nothing: the wires this leg counts come from this package"
fi

echo "== launch (The End arena, launcher-built game) =="
launch "$GAME" "$WORLD"
arena

# The four wire kinds and the drop each one owes. `WireType`'s own table: copper, gold, electrum
# and the festive wire, whose drop is a **copper** wire (`FESTIVE(3, 256, …, COPPER_WIRE.asStack(4),
# …)`) — a festive wire is never an item upstream.
KINDS=(copper gold electrum festive)
EXPECT=(createaddition:copper_wire createaddition:gold_wire createaddition:electrum_wire createaddition:copper_wire)
failures=0
for i in "${!KINDS[@]}"; do
    kind=${KINDS[$i]}
    want=${EXPECT[$i]}
    near=$((X0 + i * GAP))
    far=$((near + 4))
    echo "== $kind wire: lay it between $near and $far, break the far connector in survival =="
    wire_rig "$near" "$far"
    command_reply "createaddition link $near $Y $Z $far $Y $Z $kind" "$LINK_REPLY"
    [ "$REPLY" = "Nodes linked." ] || fail "the $kind wire was not laid: $REPLY"
    command_reply "createaddition info $near $Y $Z" "$INFO_REPLY"
    case "$REPLY" in
        *"/$kind]"*|*"/$kind),"*) echo "  near end: $REPLY" >&2 ;;
        *) fail "the near connector does not hold the $kind wire: $REPLY" ;;
    esac
    stance "$far"
    break_block "$far"
    # Exactly the kind's own wire, four of them, and the connector's own loot — nothing else.
    want_json=$(jq -cn --arg wire "$want" '{($wire): 4, "createaddition:connector": 1}')
    got_json=$(jq -cS '.drops' <<<"$BREAK")
    if [ "$got_json" != "$(jq -cS . <<<"$want_json")" ]; then
        echo "  MISMATCH: a $kind wire's break dropped $got_json, expected $(jq -cS . <<<"$want_json")" >&2
        failures=$((failures + 1))
    fi
    [ "$(jq -r '.picked_up | length' <<<"$BREAK")" = 0 ] \
        || fail "the player picked the $kind wire's drop up; the stance must leave it on the ground ($(jq -c .picked_up <<<"$BREAK"))"
    # The peer lets go of the broken connector: upstream's block-entity `remove()`.
    command_reply "createaddition info $near $Y $Z" "$INFO_REPLY"
    case "$REPLY" in
        *"links []"*) echo "  peer: $REPLY" >&2 ;;
        *) fail "the peer still names the broken $far after the $kind wire's break: $REPLY" ;;
    esac
done

echo "== differential: an unlinked connector drops no wire =="
LONE=$((X0 + 4 * GAP))
ctl command "execute in minecraft:the_end run setblock $LONE $Y $Z createaddition:connector[facing=west,mode=none]" >/dev/null
ctl wait 4 >/dev/null
[ "$(block_name "$LONE" $Y $Z)" = "createaddition:connector" ] || fail "the unlinked connector did not place at $LONE,$Y,$Z"
stance "$LONE"
break_block "$LONE"
LONE_WIRES=$(jq -r '[.drops | keys[] | select(endswith("_wire"))] | length' <<<"$BREAK")
LONE_DROPS=$(jq -cS '.drops' <<<"$BREAK")
echo "  an unlinked connector's break: $LONE_DROPS ($LONE_WIRES wire stacks)" >&2
[ "$LONE_WIRES" = 0 ] \
    || fail "an unlinked connector dropped wire(s): $LONE_DROPS"
[ "$(jq -r '.drops["createaddition:connector"] // 0' <<<"$BREAK")" = 1 ] \
    || fail "the unlinked connector did not drop itself, so this differential proves nothing: $LONE_DROPS"

if [ "$SELFTEST" = 1 ]; then
    # The counterfactual: a drop that must not happen. The unlinked connector dropped no wire, so
    # this assertion — the one the default leg's differential rests on — fails here, and the leg's
    # EXIT:1 is what makes the default pass a real pass rather than a package that always agrees.
    [ "$LONE_WIRES" -gt 0 ] \
        || fail "SELFTEST: the unlinked connector dropped $LONE_WIRES wires, so nothing here proves a break drops only a held wire ($LONE_DROPS)"
fi

[ "$failures" = 0 ] \
    || fail "$failures wire kind(s) dropped the wrong items (see the MISMATCH lines above)"

echo "PASS: a broken connector drops four wires of each kind it held — the festive wire's four are copper wires — the peer's links empty, and an unlinked connector drops none"
