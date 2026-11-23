#!/usr/bin/env bash
# The support leg: what the **level** does to a wire block whose support has gone.
#
# Upstream removes a block whose support left in `AbstractConnectorBlock.neighborChanged` /
# `RedstoneRelayBlock.neighborChanged`: `if (!state.canSurvive(...)) { dropResources(...);
# dropWires(level, true); removeBlock(...); }`. A support that is *gone* is a block that **was
# there** — upstream's `connector_ignore_face_check` default (`true`) lets a connector be placed
# with nothing behind it, and this port keeps that half (a floated block is the default's
# floating connector) while fixing the orphan: a connector that had a support and lost it goes,
# and it drops what a player's break would drop — the connector (its own loot table) and four
# wires per wire it held — with its peer's link emptied.
#
# The rig, four blocks apart on a pad in the End:
#
#   near glass + near connector (facing=down)   far glass + far connector (facing=down)
#
# with one copper wire between the connectors. The assertion is `ctl break`'s **own reply** for the
# **glass** support the far connector stands on: glass drops nothing, so the map is exactly the
# connector's and the wire's.
#
#   default     the far connector is gone, `drops` = {connector:1, copper_wire:4}, the near
#               connector's readout names no link; an identical rig whose support **stays** still
#               has its connector and its link after the same tick (the differential)
#   SUPPORT_SELFTEST=1  inverts the removal: it asserts the far connector is still there, finds it
#               gone, and exits 1 — the counterfactual that makes the default's "gone" a real
#               assertion
#   CONTROL_GAME=  a Nightly without the package cannot place `createaddition:connector`, so the
#               rig cannot be built and the leg fails: the removal this leg reads is this package's
#
# Traps this leg pays for: the pad's chunk must be loaded before a `fill` in it does anything; the
# player stands on the floor so the glass is in reach and the drops are outside the pickup radius;
# the glass is the support so the break's `drops` map carries our items only; and every reply is
# tagged, so an older identical reply can never be read as this command's.
set -Eeuo pipefail
export LC_ALL=C
[ -z "${CA_DEBUG:-}" ] || set -x

SELFTEST=${SUPPORT_SELFTEST:-0}
CONTROL=${CONTROL_GAME:+1}
CONTROL=${CONTROL:-0}
PORT=$(cd "$(dirname "$0")/.." && pwd)
CTL=${CTL:-}
GAME=${GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
# `/tmp` is a small tmpfs on this box: honour the runner's scratch root when it sets one.
TMP=${E2E_TMP:-/tmp/opencode}
mkdir -p "$TMP"
SOCK=${SOCK:-$TMP/createaddition-support.sock}
KEEP=${KEEP:-0}
WORLD=CreateAdditionSupport
CONTROL_WORLD=CreateAdditionSupportControl

Y=101
Z=0
X0=300
GAP=12
# How far the player stands from the support being broken: inside the 5-block reach, outside the
# one-block pickup radius of the connector's cell one block above it.
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

SCRATCH=$(mktemp -d "$TMP/createaddition-support-home.XXXXXX")
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
    ctl command "execute in minecraft:the_end run fill $((X0 - 8)) 100 -6 $((X0 + 48)) 100 6 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run fill $((X0 - 8)) 101 -6 $((X0 + 48)) 112 6 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run kill @e[type=!player]" >/dev/null
    ctl wait 6 >/dev/null
    [ "$(block_name "$X0" 100 0)" = "minecraft:stone" ] || fail "the arena floor is not stone"
    [ "$(block_name "$X0" 101 0)" = "minecraft:air" ] || fail "the air above the floor is not air"
}

# A pair of connectors four blocks apart, each mounted on a **glass** support one block below it
# (`facing=down`). Glass is a full block, so it holds the connectors, and it drops nothing when
# broken, so the break's `drops` map carries only our own items.
support_rig() { # support_rig NEAR FAR
    local near=$1 far=$2
    ctl command "execute in minecraft:the_end run setblock $near 101 $Z minecraft:glass" >/dev/null
    ctl wait 4 >/dev/null
    ctl command "execute in minecraft:the_end run setblock $far 101 $Z minecraft:glass" >/dev/null
    ctl wait 4 >/dev/null
    ctl command "execute in minecraft:the_end run setblock $near 102 $Z createaddition:connector[facing=down,mode=none]" >/dev/null
    ctl wait 4 >/dev/null
    ctl command "execute in minecraft:the_end run setblock $far 102 $Z createaddition:connector[facing=down,mode=none]" >/dev/null
    ctl wait 4 >/dev/null
    [ "$(block_name "$near" 102 $Z)" = "createaddition:connector" ] \
        || fail "the near connector did not place at $near,102,$Z (got '$(block_name "$near" 102 $Z)')"
    [ "$(block_name "$far" 102 $Z)" = "createaddition:connector" ] \
        || fail "the far connector did not place at $far,102,$Z (got '$(block_name "$far" 102 $Z)')"
    command_reply "createaddition link $near 102 $Z $far 102 $Z copper" "$LINK_REPLY"
    [ "$REPLY" = "Nodes linked." ] || fail "the copper wire between $near and $far was not laid: $REPLY"
    # Both ends must read a support to begin with: a floated connector is upstream's default and
    # the differential would prove nothing about the support rule.
    [ "$(block_name "$near" 101 $Z)" = "minecraft:glass" ] || fail "the near support is not glass"
    [ "$(block_name "$far" 101 $Z)" = "minecraft:glass" ] || fail "the far support is not glass"
}

# Feet on the floor, 3.5 blocks off, holding the tool that breaks glass (a pickaxe): a player's
# break is a player's, and the glass drops nothing either way.
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

# Break one block and keep `ctl break`'s own reply in BREAK.
break_block() { # break_block X Y
    local x=$1 y=$2
    BREAK=$(ctl break "$x" "$y" $Z --timeout 60)
    [ "$(jq -r '.ok // false' <<<"$BREAK")" = true ] || fail "ctl break $x $y $Z did not break: $BREAK"
    [ "$(block_name "$x" "$y" $Z)" = "minecraft:air" ] \
        || fail "the block at $x,$y,$Z is still $(block_name "$x" "$y" $Z)"
    echo "  $x,$y: $(jq -c '{drops, picked_up, ticks}' <<<"$BREAK")" >&2
}

# The near connector's own readout: the peer must let go of a removed far one, and keep a live
# one.
peer_links() { # peer_links X -> LINK_COUNT
    command_reply "createaddition info $1 102 $Z" "$INFO_REPLY"
    LINK_COUNT=$(sed -n 's/.*links \(\[[^]]*\]\).*/\1/p' <<<"$REPLY")
    LINK_COUNT=${LINK_COUNT:-unparsed}
    echo "  peer at $1: $REPLY" >&2
}

if [ "$CONTROL" = 1 ]; then
    echo "== control: the same rig on a Nightly without the package =="
    launch "$CONTROL_GAME" "$CONTROL_WORLD"
    arena
    ctl command "execute in minecraft:the_end run setblock $X0 102 $Z createaddition:connector" >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    got=$(block_name "$X0" 102 $Z)
    [ "$got" != "createaddition:connector" ] \
        || fail "the control game placed a connector, so the block id was known"
    fail "the control game does not know createaddition:connector (the setblock left '$got'), so the rig cannot be built and nothing can be removed: the removal this leg reads comes from this package"
fi

echo "== launch (The End arena, launcher-built game) =="
launch "$GAME" "$WORLD"
arena

# --- the orphan: a connector whose support is broken -----------------------------------------
ORPHAN_NEAR=$X0
ORPHAN_FAR=$((X0 + 4))
echo "== an orphan: the far connector stands on glass at $ORPHAN_FAR,101, break that glass =="
support_rig "$ORPHAN_NEAR" "$ORPHAN_FAR"
peer_links "$ORPHAN_NEAR"
case "$LINK_COUNT" in
    "["*"$ORPHAN_FAR,102,0"*"]") echo "  the wire is saved on the peer" >&2 ;;
    *) fail "the near connector does not hold the laid wire: $REPLY" ;;
esac
stance "$ORPHAN_FAR"
break_block "$ORPHAN_FAR" 101
# The level's own removal is one tick after the support changed.
ctl wait 8 >/dev/null
GONE=$(block_name "$ORPHAN_FAR" 102 $Z)
WANT=$(jq -cn '{"createaddition:connector": 1, "createaddition:copper_wire": 4}')
GOT=$(jq -cS '.drops' <<<"$BREAK")
if [ "$SELFTEST" = 1 ]; then
    # The counterfactual: the default leg asserts the orphan is **gone**. Assert the opposite and
    # let it trip — the same package really removed it, so this exits 1 and the default's "gone"
    # cannot be an assertion a do-nothing package also passes.
    [ "$GONE" = "createaddition:connector" ] \
        || fail "SUPPORT_SELFTEST: the orphan reads '$GONE' — the level removed it, so the default leg's removal assertion is alive"
fi
[ "$GONE" = "minecraft:air" ] || fail "the far connector is still $GONE after its support was broken"
[ "$GOT" = "$(jq -cS . <<<"$WANT")" ] \
    || fail "a connector removed by the level dropped $GOT, expected $(jq -cS . <<<"$WANT")"
peer_links "$ORPHAN_NEAR"
[ "$LINK_COUNT" = "[]" ] || fail "the peer still names the removed connector: $REPLY"

# --- the differential: the same rig, its support stays ----------------------------------------
KEPT_NEAR=$((X0 + GAP))
KEPT_FAR=$((KEPT_NEAR + 4))
echo "== differential: the same rig at $KEPT_FAR, its support stays =="
support_rig "$KEPT_NEAR" "$KEPT_FAR"
# Poke the kept far connector so the support check really runs on it: a neighbour change above it
# (a stone placed and taken away) queues it for the pass, exactly as the orphan's broken support
# did. The glass under it is still there, so the check must keep it.
ctl command "execute in minecraft:the_end run setblock $KEPT_FAR 103 $Z minecraft:stone" >/dev/null
ctl wait 6 >/dev/null
ctl command "execute in minecraft:the_end run setblock $KEPT_FAR 103 $Z minecraft:air" >/dev/null
ctl wait 8 >/dev/null
KEPT=$(block_name "$KEPT_FAR" 102 $Z)
[ "$KEPT" = "createaddition:connector" ] || fail "the connector whose support stayed is $KEPT, not createaddition:connector"
peer_links "$KEPT_NEAR"
case "$LINK_COUNT" in
    "["*"$KEPT_FAR,102,0"*"]") echo "  the kept rig still carries its wire" >&2 ;;
    *) fail "the kept rig's peer lost a link it should still hold: $REPLY" ;;
esac

echo "PASS: a connector whose support is broken is removed by the level and drops the connector and four wires per wire it held, with its peer's link emptied; an identically placed connector on a support that stays keeps both"
