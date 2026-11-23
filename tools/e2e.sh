#!/usr/bin/env bash
# In-game check for Create Crafts & Additions' FE half: two connectors, one wire
# and two modular accumulators, with the jar's own rates.
#
# **The game must be a launcher-built one** (`horizon-launcher native-build <port> --instance <i>
# --trust-native-code`), with this package installed. `horizon-launcher` is not installed in this
# environment, so this scenario has NOT been run at the 0b14f8a pin: `notes.md` records it as a gap.
# `GAME=` names that binary; `CONTROL_GAME=` optionally names a game WITHOUT the package, which the
# control leg uses to prove the ids really come from it.
#
# Every mod command is typed with `ctl chat "/cmd"` because the console path bypasses the mod's
# command hook.
#
# The End, in the void at x=200 (away from the central island, its dragon egg
# and the ambient item entities of a generated level), on a stone floor.
#
# Arena (all at y=100 unless said):
#   A  accumulator (source)   x=10      C1 small connector x=11 facing west
#   C2 small connector x=15 facing east   B accumulator (sink) x=16
# so C1 is mounted on A and C2 on B, and the wire spans four blocks.
#
# Default run:
#  A  a connector placed beside an accumulator takes `pull` by itself, and the
#     accumulator is 2,000,000 FE; the connector's window is 1,000 FE;
#  B  a source accumulator seeded with 100,000 FE sends energy across the wire
#     to the sink through a `push` connector: the source loses exactly what the
#     sink gains plus what the network still holds (FE is moved, never copied),
#     and no tick moves more than the small connector's 1,000 FE;
#  C  the CONTROL: with the wire taken out, nothing moves at all over the same
#     number of ticks — a connector with no wire is not a network;
#  D  a wire stretched past a small connector's 16 blocks is refused, and no
#     link is recorded;
#  E  copper between two large connectors is refused (gold is accepted), as
#     upstream's `REQUIRES_HIGH_CURRENT`;
#  F  an accumulator feeds the block above it (`tickOutput`), and both stay empty
#     when both start empty;
#  G  the accumulator tower (3x3x5 multiblock, one pool, split, salvage, doctored saves);
#  H  every carried block drops one of itself through its 26.3 loot table;
#  I  the redstone relay: an unpowered relay between two accumulators moves nothing
#     over 20 ticks, a redstone block on its side sets `powered=true` and the sink
#     then gains FE with the source, both pools and the relay's buffer conserved.
#  J  the small light connector: dark at 0 FE, lit after its own hysteresis once its
#     network pays it, exactly 1 FE a tick out of the source (the pool steady), block
#     light 15 while lit and 0 while dark, and the unlinked differential: dark again and
#     the source does not move.
#  K  the accumulator's comparator output: a hierarchy comparator reading a formed 1x3
#     tower drives its own redstone wire to power 0 (empty), 8 (3,000,000 of 6,000,000 — the
#     jar's `fractionToRedstoneLevel` midpoint) and 15 (full); the same rig behind a stone
#     block reads 0, and emptying the tower with `/createaddition energy` takes the wire
#     back to 0.
#  L  the creative generator: an unlimited source whose capability gives what is asked and
#     takes nothing; a connector mounted on it settles `pull` and a wire carries exactly
#     1,000 FE a tick to an accumulator (20,000 FE in the 20-tick window), stone in the
#     generator's place moves nothing, and a generator beside an accumulator fills it at the
#     accumulator's 5,000 FE a tick with no connector at all.
#  M  the festive wire: `createaddition:festive_spool` lays it with two right clicks, both ends
#     save the type as `festive`, the creative hand keeps its spool, it carries exactly 1,000 FE
#     a tick like copper, two large connectors refuse it (`REQUIRES_HIGH_CURRENT`) while gold is
#     accepted, no iron wire kind exists (the item is content only, as in 1.7.2), and in survival
#     the empty spool coils a festive wire back onto a festive spool.
#
# SELFTEST=1 flips B's conservation assertion (it must FAIL), proving the
# scenario can fail. RELAY_SELFTEST=1 flips I's unpowered gate assertion instead, so
# the relay leg's own counterfactual is a run of its own. LAMP_SELFTEST=1 flips J's
# unlinked-lamp assertion (the lamp must not draw a single FE), so the lamp's
# differential is a run of its own too. COMPARATOR_SELFTEST=1 flips K's emptied-tower
# assertion (the wire must have stayed where it was), so the comparator's own
# counterfactual is a run as well. CREATIVE_SELFTEST=1 flips L's gain assertion (the generator
# must have supplied nothing), so the creative generator's own counterfactual is a run too.
# CMP_ONLY=1 runs just the comparator rig and prints both
# comparator orientations with the wire each drives (the probe that pinned the geometry).
#
# The 0.1.0 runs of this scenario (PASS/EXIT:0 default, PASS/EXIT:0 with the
# bundle's embedded content, FAIL/EXIT:1 selftest) are historical evidence for the
# retired `.hnmod`; `VALIDATION.md` says which format each log belongs to.
set -Eeuo pipefail
export LC_ALL=C
[ -z "${CA_DEBUG:-}" ] || set -x

CONTROL=${CONTROL_GAME:+1}
CONTROL=${CONTROL:-0}
SELFTEST=${SELFTEST:-0}

CLONE=${CLONE:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CLONE/ports/createaddition
PIN_BIN=$CLONE/ports/_shared/bin/horizon-1d63e0a5
CTL=${CTL:-$PIN_BIN/horizon-ctl}
GAME=${GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
# `/tmp` is a 7.5 GB tmpfs; the launcher hands e2e runs a scratch root on /home via E2E_TMP.
TMP=${E2E_TMP:-/tmp/opencode}
mkdir -p "$TMP"
SOCK=${SOCK:-$TMP/createaddition-e2e.sock}
KEEP=${KEEP:-0}
WORLD=CreateAdditionE2E
CONTROL_WORLD=CreateAdditionControl
# The arena: the End's void, far from the central island.
X0=200
AX=$((X0 - 10))
CX1=$((AX + 1))
CX2=$((CX1 + 4))
B=$((CX2 + 1))
# The long-wire pair: 17 blocks apart, past a small connector's 16.
FAR_A=$((X0 + 2))
FAR_B=$((FAR_A + 17))
# The large-connector pair: four blocks apart.
BIG_A=$((X0 + 6))
BIG_B=$((BIG_A + 4))
# A clean stone pad for the loot leg: the arena floor has device blocks (connectors,
# accumulators) replacing its stone, and a player teleported onto one of those can fall through
# the partial-collision shape into the void. The loot test stands here instead.
LOOT_X=$((X0 + 40))
LOOT_Z=8
TICKS=20
RATE=1000
SEED=100000
CAPACITY=2000000

# A setup refusal still ends with an exit line, so a captured log always says how the run ended.
guard() { echo "EXIT:1"; exit 1; }
command -v jq >/dev/null || { echo "missing jq (every assertion parses a ctl reply)" >&2; guard; }
[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; guard; }
[ -n "$GAME" ] || {
    echo "GAME= is required: this package is native, so the game must be launcher-built" >&2
    echo "  horizon-launcher native-build $PORT --instance <instance> --trust-native-code" >&2
    echo "  GAME=<that horizon-client> $0" >&2
    guard
}
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; guard; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; guard; }

SCRATCH=$(mktemp -d "$TMP/createaddition-home.XXXXXX")
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
# `ctl messages` lists the newest line first, so `head` is what just happened and `tail` is the
# oldest of the window (printing the tail once hid the answer that mattered).
recent_chat() { chat | head -20; }
fail() { echo "FAIL: $*" >&2; recent_chat >&2 || true; ctl logs --lines 40 >&2 || true; exit 1; }
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

launch() { # launch BINARY [WORLD] [extra args]
    local game=$1 world=$2; shift 2
    rm -f "$SOCK"
    cd "$SCRATCH"
    local args=(--game "$game" --world "$world" --seed 20261105 --creative --headless
                --wait --resources "$RES")
    "$CTL" --socket "$SOCK" launch "${args[@]}" "$@" >/dev/null
    ctl wait-ready 120
    ctl wait 40
}

block_name() { ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty'; }

# The block state's own string, e.g. `createaddition:modular_accumulator[bottom=false,top=true]`.
block_state() { ctl block "$1" "$2" "$3" | jq -r '.state // empty'; }

# A redstone wire's own `power` state — the signal the world carries out of a comparator, not the
# mod's readout. Empty when the block at that position has no `power` property.
wire_power() { # X Y Z -> the integer in `power=`
    block_state "$1" "$2" "$3" | sed -n 's/.*power=\([0-9][0-9]*\).*/\1/p'
}

expect_wire() { # expect_wire X Y Z LEVEL WHAT
    local power
    power=$(wire_power "$1" "$2" "$3")
    [ -n "$power" ] \
        || fail "no redstone wire at $1,$2,$3 (state '$(block_state "$1" "$2" "$3")')"
    [ "$power" = "$4" ] \
        || fail "$5: the wire at $1,$2,$3 carries power=$power, expected power=$4"
    echo "  $5: redstone_wire at $1,$2,$3 power=$power" >&2
}

# The `top` / `bottom` multiblock bits of an accumulator, asserted on the state string the game
# reports (not on the mod's own readout).
expect_bits() { # expect_bits X Y TOP BOTTOM
    local state
    state=$(block_state "$1" "$2" 0)
    case "$state" in
        *"top=$3"*) ;;
        *) fail "the accumulator at $1,$2 reports '$state', expected top=$3" ;;
    esac
    case "$state" in
        *"bottom=$4"*) ;;
        *) fail "the accumulator at $1,$2 reports '$state', expected bottom=$4" ;;
    esac
    echo "  $state" >&2
}

# The End's void at X0: a stone floor and clear air above it.
arena() {
    ctl command "execute in minecraft:the_end run tp @s $X0.5 104 0.5" >/dev/null
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(ctl block "$X0" 100 0 | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the End arena chunk never loaded"
    ctl command "execute in minecraft:the_end run fill $((X0 - 20)) 100 -14 $((X0 + 30)) 100 14 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run fill $((X0 - 20)) 101 -14 $((X0 + 30)) 108 14 minecraft:air" >/dev/null
    # The loot pad: all stone, away from every device block.
    ctl command "execute in minecraft:the_end run fill $((LOOT_X - 2)) 100 $((LOOT_Z - 2)) $((LOOT_X + 4)) 100 $((LOOT_Z + 2)) minecraft:stone" >/dev/null
    ctl command "execute in minecraft:the_end run fill $((LOOT_X - 2)) 101 $((LOOT_Z - 2)) $((LOOT_X + 4)) 104 $((LOOT_Z + 2)) minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run kill @e[type=!player]" >/dev/null
    ctl command "execute in minecraft:the_end run tp @s $X0.5 101 0.5" >/dev/null
    ctl wait 20
    [ "$(block_name "$X0" 100 0)" = "minecraft:stone" ] || fail "the arena floor is not stone"
    [ "$(block_name "$X0" 101 0)" = "minecraft:air" ] || fail "the arena air above the floor is not air"
}

# Every reply is tagged: the command carries a `#tag` word and the mod appends
# ` [#tag]` to its answer, so a reply from an earlier command (chat lines can
# arrive twice) can never be read as this command's answer.
LINK_REPLY='^(Nodes linked\.|The connectors are too far apart for this wire\.|Every node of this connector has a wire\.|These connectors are already linked\.|A connector cannot be linked to itself\.|Two large connectors need a gold or electrum wire\.)'
WIRE_REPLY='^(Wire removed\.|These connectors are not linked\.)'
MODE_REPLY='^Mode set to (none|pull|push)\.'
ENERGY_REPLY='^Energy set to [0-9]+ FE\.'
MODE_REFUSAL='^The mode is none, pull or push\.'
WIRE_WORD_REFUSAL='^The wire is copper, gold, electrum or festive\.'

NEXT_TAG=0

chat() { ctl messages 60 | jq -r '.chat[]'; }

# Type a native command with a fresh tag appended and wait for its answer; the
# tag is removed from REPLY.
command_reply() { # command_reply COMMAND REPLY_PATTERN
    NEXT_TAG=$((NEXT_TAG + 1))
    local tag pattern round
    tag="q$NEXT_TAG"
    pattern="$2 .*\\[#$tag\\]$"
    for round in 1 2; do
        # A chat line can be lost between the client and the server; the reply, not the send,
        # is what this waits for, so a second send of the same tagged line costs only ticks and
        # never weakens the assertion (a command that is refused answers nothing matching).
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
    chat | head -20 >&2
    fail "no reply to /$1 #$tag"
}

# `/createaddition info X Y Z`: the line, then the parsed fields.
info() { # info X Y Z
    # The reply names the position, so a line about another block can never be
    # mistaken for this one.
    command_reply "createaddition info $1 $2 $3" "^(Connector|Large Connector|Small Light Connector|Modular Accumulator|Redstone Relay|Creative Generator) at $1,$2,$3:"
    INFO_LINE=$REPLY
    eval "$(python3 "$PORT/tools/parse_info.py" "$INFO_LINE")"
    [ "${ok:-0}" = 1 ] || fail "the info line did not parse: $INFO_LINE"
    echo "  $INFO_LINE" >&2
}

# While `/tick freeze` holds the level pass off, `ctl wait` counts game ticks and would never
# return: poll the chat log by wall clock instead.
frozen_command_reply() { # frozen_command_reply COMMAND REPLY_PATTERN
    NEXT_TAG=$((NEXT_TAG + 1))
    local tag pattern
    tag="q$NEXT_TAG"
    pattern="$2 .*\\[#$tag\\]$"
    say "$1 #$tag"
    for _ in $(seq 1 50); do
        if chat | grep -qE "$pattern"; then
            REPLY=$(chat | grep -E "$pattern" | tail -1)
            REPLY=${REPLY% "[#$tag]"}
            REPLY=${REPLY% }
            return 0
        fi
        sleep 0.2
    done
    echo "--- newest chat lines:" >&2
    chat | tail -15 >&2
    fail "no reply to /$1 #$tag while the game was frozen"
}

frozen_info() { # frozen_info X Y
    frozen_command_reply "createaddition info $1 $2 0" "^(Connector|Large Connector|Modular Accumulator) at $1,$2,0:"
    INFO_LINE=$REPLY
    eval "$(python3 "$PORT/tools/parse_info.py" "$INFO_LINE")"
    [ "${ok:-0}" = 1 ] || fail "the frozen info line did not parse: $INFO_LINE"
    echo "  $INFO_LINE" >&2
}

# One connector on the floor, then the connector settles.
place_connector() { # place_connector X FACING [BLOCK] [Y]
    local x=$1 facing=$2 block=${3:-createaddition:connector} y=${4:-100}
    ctl command "execute in minecraft:the_end run setblock $x $y 0 $block[facing=$facing,mode=none]" >/dev/null
    ctl wait 6 >/dev/null
    [ "$(block_name "$x" "$y" 0)" = "$block" ] \
        || fail "the $block did not place at $x,$y (got '$(block_name "$x" "$y" 0)')"
}

place_accumulator() { # place_accumulator X [Y]
    local y=${2:-100}
    ctl command "execute in minecraft:the_end run setblock $1 $y 0 createaddition:modular_accumulator" >/dev/null
    ctl wait 6 >/dev/null
    [ "$(block_name "$1" "$y" 0)" = "createaddition:modular_accumulator" ] \
        || fail "the accumulator did not place at $1,$y"
}

# A block at (X, Y, 0) from its setblock argument, asserted by its name: used for the blocks
# that are not one of the two above (the creative generator).
place_block() { # place_block X Y ARG NAME
    local x=$1 y=$2 arg=$3 name=$4
    ctl command "execute in minecraft:the_end run setblock $x $y 0 $arg" >/dev/null
    ctl wait 6 >/dev/null
    [ "$(block_name "$x" "$y" 0)" = "$name" ] \
        || fail "the $name did not place at $x,$y (got '$(block_name "$x" "$y" 0)')"
}

# `/createaddition energy X Y Z AMOUNT`.
seed_at() { # seed_at X Y AMOUNT
    command_reply "createaddition energy $1 $2 0 $3" "$ENERGY_REPLY"
}

# The source's charge, the sink's and the network buffer as one total. Both
# connectors are parked in `none` first, so nothing moves between the three
# reads and the totals are of one instant.
measure() { # measure -> SOURCE SINK POOL TOTAL
    command_reply "createaddition mode $CX1 100 0 none" "$MODE_REPLY"
    command_reply "createaddition mode $CX2 100 0 none" "$MODE_REPLY"
    info "$AX" 100 0
    SOURCE=$energy
    [ "$kind" = accumulator ] || fail "the source at $AX is a $kind"
    info "$B" 100 0
    SINK=$energy
    info "$CX1" 100 0
    POOL=$((pool_ready + pool_next))
    NOW=$t
    TOTAL=$((SOURCE + SINK + POOL))
    command_reply "createaddition mode $CX1 100 0 pull" "$MODE_REPLY"
    command_reply "createaddition mode $CX2 100 0 push" "$MODE_REPLY"
}

seed_source() { # seed_source AMOUNT
    seed_at "$AX" 100 "$1"
}

link() { # link X1 X2 WIRE
    command_reply "createaddition link $1 100 0 $2 100 0 $3" "$LINK_REPLY"
    LAST_REPLY=$REPLY
    echo "  link $1-$2 ($3): $LAST_REPLY" >&2
}

unlink() { # unlink X1 X2
    command_reply "createaddition unlink $1 100 0 $2 100 0" "$WIRE_REPLY"
    LAST_REPLY=$REPLY
    echo "  unlink $1-$2: $LAST_REPLY" >&2
}

set_mode() { # set_mode X MODE
    command_reply "createaddition mode $1 100 0 $2" "$MODE_REPLY"
    LAST_REPLY=$REPLY
    echo "  mode $1 $2: $LAST_REPLY" >&2
}

# CONTROL_GAME=: a game without the package, where the ids must be unknown.
control_leg() {
    echo "== control: a game without this package =="
    launch "$CONTROL_GAME" "$CONTROL_WORLD"
    arena
    ctl give createaddition:connector 1 >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    ctl inventory | jq -e '[.hotbar[], .main[] | select(.item == "createaddition:connector")] | length == 0' >/dev/null \
        || fail "the control game has the connector item, so the ids do not come from this package"
    ctl command "execute in minecraft:the_end run setblock $CX1 100 0 createaddition:connector" >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    local name
    name=$(block_name "$CX1" 100 0)
    [ "$name" != "createaddition:connector" ] || fail "the control game placed a connector, so the block id was known"
    echo "  the control's setblock left $(block_name "$CX1" 100 0) (the arena floor)" >&2
    # The relay is this slice's new id: the same package-less game must not know it either.
    local relay_at=$((X0 + 2))
    ctl command "execute in minecraft:the_end run setblock $relay_at 101 0 createaddition:redstone_relay" >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    [ "$(block_name "$relay_at" 101 0)" != "createaddition:redstone_relay" ] \
        || fail "the control game placed a redstone relay, so the block id was known"
    echo "  the control's setblock left $(block_name "$relay_at" 101 0) where the relay would be" >&2
    # The lamp is this slice's new id: the same package-less game must not know it.
    local lamp_at=$((X0 + 50))
    ctl command "execute in minecraft:the_end run setblock $lamp_at 101 0 createaddition:small_light_connector" >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    [ "$(block_name "$lamp_at" 101 0)" != "createaddition:small_light_connector" ] \
        || fail "the control game placed a small light connector, so the block id was known"
    echo "  the control's setblock left $(block_name "$lamp_at" 101 0) where the lamp would be" >&2
    # The creative generator is this slice's new id: the same package-less game must not know it.
    local creative_at=$((X0 + 55))
    ctl command "execute in minecraft:the_end run setblock $creative_at 101 0 createaddition:creative_energy" >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    [ "$(block_name "$creative_at" 101 0)" != "createaddition:creative_energy" ] \
        || fail "the control game placed a creative generator, so the block id was known"
    echo "  the control's setblock left $(block_name "$creative_at" 101 0) where the creative generator would be" >&2
    # The festive spool and the iron wire are this slice's items: the same package-less game
    # must not know them either.
    ctl give createaddition:festive_spool 1 >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    [ "$(item_held createaddition:festive_spool)" = 0 ] \
        || fail "the control game has the festive spool item, so the item ids do not come from this package"
    ctl give createaddition:iron_wire 1 >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    [ "$(item_held createaddition:iron_wire)" = 0 ] \
        || fail "the control game has the iron wire item, so the item ids do not come from this package"
    echo "  the control game refuses both createaddition:festive_spool and createaddition:iron_wire" >&2
    ctl quit >/dev/null 2>&1 || true
}

# How many of item `$1` the player holds, hotbar and main inventory.
item_held() {
    ctl inventory | jq --arg item "$1" '[.hotbar[], .main[] | select(.item == $item) | .count] | add // 0'
}

# The block the player stands in, as three integers.
player_block() {
    ctl state | jq -r '.player.position | [(.[0] | floor), (.[1] | floor), (.[2] | floor)] | @tsv'
}

# One of the mod's blocks, placed beside the player, mined for real with an iron pickaxe, and its
# drop counted in the inventory. The drop is the block's own loot table (26.3's `"condition":
# {"type": ...}` shape, no `bonus_rolls`): in 1.21.1's `"conditions": [ ... ]` shape the block
# drops nothing, which is how `modularrouters` caught its router dropping 0 of itself.
break_drop_leg() { # break_drop_leg BLOCK -> sets DROPPED
    local block=$1 before after px py pz at
    # A creative break drops nothing, and a block that needs a correct tool drops nothing to a
    # hand: mine it in survival, holding the pickaxe its template's tag asks for.
    ctl command "item replace entity @s hotbar.0 with minecraft:iron_pickaxe 1" >/dev/null
    ctl select 1 >/dev/null
    ctl command "gamemode survival" >/dev/null
    ctl command "execute in minecraft:the_end run tp @s $LOOT_X.5 101 $LOOT_Z.5" >/dev/null
    ctl wait 6 >/dev/null
    read -r px py pz <<<"$(player_block)"
    at=$((px + 1))
    ctl command "execute at @s run setblock ~1 ~ ~ $block" >/dev/null
    ctl wait 4 >/dev/null
    [ "$(block_name "$at" "$py" "$pz")" = "$block" ] \
        || fail "the drop-test $block did not place at $at,$py,$pz"
    before=$(item_held "$block")
    ctl break "$at" "$py" "$pz" >/dev/null
    # Stand on the drop (the block was just removed, so the cell is air over the floor): pickup
    # must not depend on the item settling at the edge of the one-block pickup radius.
    ctl command "tp @s $at.5 $py.5 $pz.5" >/dev/null
    # Pickup is a server tick after the drop: poll for it (bounded) instead of reading one
    # instant. A wrong loot table never raises the count, so the assertion below still fails.
    after=$before
    for _ in $(seq 1 40); do
        ctl wait 2 >/dev/null
        after=$(item_held "$block")
        [ "$after" -gt "$before" ] && break
    done
    [ "$(block_name "$at" "$py" "$pz")" = "minecraft:air" ] \
        || fail "the $block at $at,$py,$pz did not break (state $(block_name "$at" "$py" "$pz"))"
    DROPPED=$((after - before))
    echo "  $block: broke and dropped $DROPPED (held $before -> $after)" >&2
}

# FESTIVE_ONLY=1: the festive spool click alone — place the rig, right-click both connectors
# with the spool, and print what the two ends saved. It is the probe that established the click
# geometry (a connector's outline is the nub on its mounted face, so the click comes from the
# side away from the wall); it asserts only that a wire was laid, so a geometry problem stops
# here instead of inside the full scenario.
if [ "${FESTIVE_ONLY:-0}" = 1 ]; then
    launch "$GAME" "$WORLD"
    arena
    P_A=$((X0 + 68))
    P_C1=$((P_A + 1))
    P_C2=$((P_C1 + 4))
    P_B=$((P_C2 + 1))
    ctl command "execute in minecraft:the_end run tp @s $((X0 + 52)).5 101 0.5" >/dev/null
    ctl wait 10 >/dev/null
    ctl command "execute in minecraft:the_end run fill $((P_A - 3)) 100 -2 $((P_B + 1)) 100 2 minecraft:stone" >/dev/null
    ctl command "execute in minecraft:the_end run fill $((P_A - 3)) 101 -2 $((P_B + 1)) 104 2 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run tp @s $((P_C1 + 2)).5 101 0.5" >/dev/null
    ctl wait 4 >/dev/null
    place_accumulator "$P_A" 102
    place_accumulator "$P_B" 102
    place_connector "$P_C1" west createaddition:connector 102
    place_connector "$P_C2" east createaddition:connector 102
    ctl command "item replace entity @s hotbar.0 with createaddition:festive_spool 1" >/dev/null
    ctl select 1 >/dev/null
    ctl wait 4 >/dev/null
    echo "  probe: holding $(item_held createaddition:festive_spool) festive spool(s), state $(block_state "$P_C1" 102 0)" >&2
    ctl interact "$P_C1" 102 0 east >/dev/null 2>&1 || true
    ctl wait 2 >/dev/null
    echo "  probe: after the first click the spool reads $(ctl inventory | jq -c '.hotbar[] | select(.item=="createaddition:festive_spool")')" >&2
    echo "  probe: chat after the first click: $(chat | head -4 | tr '\n' '|')" >&2
    ctl interact "$P_C2" 102 0 west >/dev/null 2>&1 || true
    ctl wait 6 >/dev/null
    echo "  probe: after the second click the spool reads $(ctl inventory | jq -c '.hotbar[] | select(.item=="createaddition:festive_spool")')" >&2
    echo "  probe: chat after the second click: $(chat | head -4 | tr '\n' '|')" >&2
    info "$P_C1" 102 0
    info "$P_C2" 102 0
    echo "PASS: the festive probe ran (FESTIVE_ONLY)"
    exit 0
fi

# LOOT_ONLY=1: the loot-table leg alone, on a launcher-built game. Used to run the same
# assertion against a package that still carries the 1.21.1 loot tables: that run must fail,
# and its EXIT:1 is the pre-fix red leg for the conversion.
if [ "${LOOT_ONLY:-0}" = 1 ]; then
    launch "$GAME" "$WORLD"
    arena
    for LOOT_BLOCK in createaddition:connector createaddition:large_connector createaddition:small_light_connector createaddition:modular_accumulator createaddition:redstone_relay createaddition:creative_energy; do
        break_drop_leg "$LOOT_BLOCK"
        [ "$DROPPED" = 1 ] \
            || fail "mining $LOOT_BLOCK dropped $DROPPED of itself, expected the one its 26.3 loot table names"
        ctl command "execute at @s run setblock ~2 ~ ~ minecraft:air" >/dev/null
    done
    echo "PASS: every block drops one of itself (LOOT_ONLY)"
    exit 0
fi

# CMP_ONLY=1: the comparator rig alone — both orientations of one comparator, each with its own
# wire, printed so the geometry the default run asserts is measured and not assumed. The probe
# log is the evidence for which side of a comparator reads the block behind it.
if [ "${CMP_ONLY:-0}" = 1 ]; then
    launch "$GAME" "$WORLD"
    arena
    CMP_PROBE=$((X0 + 60))
    ctl command "execute in minecraft:the_end run fill $((CMP_PROBE - 3)) 100 -4 $((CMP_PROBE + 3)) 100 4 minecraft:stone" >/dev/null
    ctl command "execute in minecraft:the_end run fill $((CMP_PROBE - 3)) 101 -4 $((CMP_PROBE + 3)) 104 4 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run tp @s $((CMP_PROBE + 1)).5 101 0.5" >/dev/null
    ctl wait 4 >/dev/null
    place_accumulator "$CMP_PROBE" 100
    place_accumulator "$CMP_PROBE" 101
    place_accumulator "$CMP_PROBE" 102
    ctl wait 6 >/dev/null
    info "$CMP_PROBE" 100 0
    echo "  probe: tower $energy/$capacity at x=$CMP_PROBE" >&2
    seed_at "$CMP_PROBE" 100 6000000
    ctl wait 8 >/dev/null
    ctl command "execute in minecraft:the_end run setblock $CMP_PROBE 101 1 minecraft:comparator[facing=north]" >/dev/null
    ctl command "execute in minecraft:the_end run setblock $CMP_PROBE 101 2 minecraft:redstone_wire" >/dev/null
    ctl command "execute in minecraft:the_end run setblock $CMP_PROBE 101 -1 minecraft:comparator[facing=south]" >/dev/null
    ctl command "execute in minecraft:the_end run setblock $CMP_PROBE 101 -2 minecraft:redstone_wire" >/dev/null
    ctl wait 12 >/dev/null
    echo "  probe: comparator$([ "$(block_name "$CMP_PROBE" 101 1)" = minecraft:comparator ] && echo ok || echo missing) facing=north, wire z=2 power=$(wire_power "$CMP_PROBE" 101 2)" >&2
    echo "  probe: comparator$([ "$(block_name "$CMP_PROBE" 101 -1)" = minecraft:comparator ] && echo ok || echo missing) facing=south, wire z=-2 power=$(wire_power "$CMP_PROBE" 101 -2)" >&2
    echo "  probe: tower reads $(block_state "$CMP_PROBE" 101 0)" >&2
    echo "PASS: the comparator probe printed both orientations (CMP_ONLY)"
    exit 0
fi

# CONTROL_ONLY=1: the package's first assertion — its item exists — against GAME, with no arena.
# Run it with GAME=<a game without the package> to prove the ids come from this package: that
# run must FAIL, and its EXIT:1 is the counterfactual the evidence keeps.
if [ "${CONTROL_ONLY:-0}" = 1 ]; then
    launch "$GAME" "$CONTROL_WORLD"
    ctl give createaddition:modular_accumulator 1 >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    ctl inventory | jq -e '[.hotbar[], .main[] | select(.item == "createaddition:modular_accumulator")] | length >= 1' >/dev/null \
        || fail "the game has no createaddition:modular_accumulator item, so the ids come from this package"
    echo "PASS: the game carries createaddition:modular_accumulator (CONTROL_ONLY, no arena)"
    exit 0
fi

echo "== launch (The End arena, launcher-built game) =="
if [ "$CONTROL" = 1 ]; then control_leg; fi
launch "$GAME" "$WORLD"
arena

# A package-less game refuses the id, so the give itself fails: the assertion is the inventory
# below, which names what is missing.
ctl give createaddition:modular_accumulator 1 >/dev/null 2>&1 || true
ctl wait 4 >/dev/null
ctl inventory | jq -e '[.hotbar[], .main[] | select(.item == "createaddition:modular_accumulator")] | length >= 1' >/dev/null \
    || fail "the accumulator item did not register: $(ctl inventory | jq -c '{hotbar,main}')"

echo "== A: the four blocks, their buffers and the placement mode =="
place_accumulator "$AX"
place_accumulator "$B"
place_connector "$CX1" west
place_connector "$CX2" east
info "$CX1" 100 0
[ "$kind" = connector ] || fail "C1 has no block entity (info says $kind)"
[ "$capacity" = 1000 ] || fail "a small connector's window is $capacity, expected 1000"
[ "$mode" = pull ] || fail "C1 placed beside an accumulator took mode '$mode', expected pull"
[ "${large:-0}" = 0 ] || fail "C1 is a large connector"
# 1.7.2 declares a `Passive` mode its own wrench can never reach (the port
# drops it: `## Upstream defects fixed` in notes.md), so setting it must be
# refused and the mode must stay as it was.
command_reply "createaddition mode $CX1 100 0 passive" "$MODE_REFUSAL"
[ "$REPLY" = "The mode is none, pull or push." ] \
    || fail "the unreachable passive mode was not refused: $REPLY"
info "$CX1" 100 0
[ "$mode" = pull ] || fail "the refused mode still changed the connector to '$mode'"
info "$AX" 100 0
[ "$kind" = accumulator ] || fail "the source has no block entity"
[ "$capacity" = "$CAPACITY" ] || fail "the accumulator's capacity is $capacity, expected $CAPACITY"
[ "$energy" = 0 ] || fail "a fresh accumulator holds $energy FE"

echo "== B: 100,000 FE crosses the wire at 1,000 FE a tick =="
seed_source "$SEED"
set_mode "$CX2" push
link "$CX1" "$CX2" gold
[ "$LAST_REPLY" = "Nodes linked." ] || fail "the wire was not laid: $LAST_REPLY"
measure
BEFORE_SOURCE=$SOURCE
BEFORE_SINK=$SINK
BEFORE_POOL=$POOL
BEFORE_NOW=$NOW
BEFORE_TOTAL=$TOTAL
[ "$BEFORE_TOTAL" = "$SEED" ] || fail "the seeded $SEED FE totals $BEFORE_TOTAL across source, sink and network"
ctl wait "$TICKS"
measure
MOVED=$((BEFORE_SOURCE - SOURCE))
GAINED=$((SINK - BEFORE_SINK))
POOL_DELTA=$((POOL - BEFORE_POOL))
# `t=` on the two reads is the game clock: it also counts the ticks the four
# mode commands and the six reads take, so the window is $TICKS plus that
# overhead. The cap is asserted over $TICKS plus a generous allowance for those
# commands; at 1,000 FE a tick the flow must also be near the cap.
ELAPSED=$((NOW - BEFORE_NOW))
[ "$ELAPSED" -ge "$TICKS" ] || fail "only $ELAPSED ticks passed, expected at least $TICKS"
[ "$MOVED" -gt 0 ] || fail "no energy left the source over $ELAPSED ticks"
[ "$MOVED" -le $((RATE * (TICKS + 10))) ] \
    || fail "$MOVED FE moved over $ELAPSED ticks, more than the ${RATE}/tick a small connector allows"
[ "$MOVED" -ge $((RATE * (TICKS - 4))) ] \
    || fail "$MOVED FE moved over $ELAPSED ticks ($((MOVED / (ELAPSED > 0 ? ELAPSED : 1))) FE/tick), far below the ${RATE}/tick expected"
[ "$GAINED" = $((MOVED - POOL_DELTA)) ] \
    || fail "the source lost $MOVED FE, the network gained $POOL_DELTA and the sink gained $GAINED"
if [ "$SELFTEST" = 1 ]; then
    # The deliberately wrong expectation: the total MUST have changed. FE is
    # conserved, so this leg fails — which is the proof the scenario can fail.
    [ "$TOTAL" != "$BEFORE_TOTAL" ] || fail "SELFTEST: the conservation assertion did not trip"
fi
[ "$TOTAL" = "$BEFORE_TOTAL" ] \
    || fail "FE was not conserved across the wire: $BEFORE_TOTAL -> $TOTAL (source $BEFORE_SOURCE->$SOURCE, sink $BEFORE_SINK->$SINK)"
echo "  source $BEFORE_SOURCE->$SOURCE, sink $BEFORE_SINK->$SINK, network buffer $POOL, total conserved at $TOTAL"

echo "== C: CONTROL — the same two connectors with no wire move nothing =="
unlink "$CX1" "$CX2"
[ "$LAST_REPLY" = "Wire removed." ] || fail "the wire was not removed: $LAST_REPLY"
info "$CX1" 100 0
[ "$links" = 0 ] || fail "C1 still holds $links wire(s) after the unlink"
measure
CUT_SOURCE=$SOURCE
CUT_SINK=$SINK
CUT_TOTAL=$TOTAL
ctl wait "$TICKS"
measure
[ "$SOURCE" = "$CUT_SOURCE" ] || fail "the source moved $CUT_SOURCE->$SOURCE FE with no wire"
[ "$SINK" = "$CUT_SINK" ] || fail "the sink moved $CUT_SINK->$SINK FE with no wire"
[ "$TOTAL" = "$CUT_TOTAL" ] || fail "the total moved $CUT_TOTAL->$TOTAL with no wire"
echo "  no wire: source $SOURCE, sink $SINK, total $TOTAL, unchanged over $TICKS ticks"

echo "== D: a wire past a small connector's 16 blocks is refused =="
place_accumulator $((FAR_A - 1))
place_accumulator $((FAR_B + 1))
place_connector "$FAR_A" west
place_connector "$FAR_B" east
link "$FAR_A" "$FAR_B" copper
case "$LAST_REPLY" in
    *"too far apart"*) echo "  refused at 17 blocks, as the jar's 16-block limit says" ;;
    *) fail "a 17-block wire was not refused: $LAST_REPLY" ;;
esac
info "$FAR_A" 100 0
[ "$links" = 0 ] || fail "the refused wire was recorded anyway ($links link(s))"

echo "== E: copper cannot join two large connectors, gold can =="
place_accumulator $((BIG_A - 1))
place_accumulator $((BIG_B + 1))
place_connector "$BIG_A" west createaddition:large_connector
place_connector "$BIG_B" east createaddition:large_connector
info "$BIG_A" 100 0
[ "$capacity" = 5000 ] || fail "a large connector's window is $capacity, expected 5000"
link "$BIG_A" "$BIG_B" copper
case "$LAST_REPLY" in
    *"gold or electrum"*) echo "  copper refused between two large connectors" ;;
    *) fail "copper was not refused between two large connectors: $LAST_REPLY" ;;
esac
info "$BIG_A" 100 0
[ "$links" = 0 ] || fail "the refused copper wire was recorded anyway"
link "$BIG_A" "$BIG_B" gold
[ "$LAST_REPLY" = "Nodes linked." ] || fail "gold was refused between two large connectors: $LAST_REPLY"

echo "== F: an accumulator feeds the block above it (tickOutput UP) =="
# The block above is a connector held in `none`, so its 1,000 FE window only
# takes energy: nothing drains it, and its own pull is off. Two stacked
# accumulators would trade energy both ways — that pair is upstream's
# multiblock, a recorded gap.
STACK=$((X0 + 7))
place_accumulator "$STACK" 100
place_connector "$STACK" down createaddition:connector 101
command_reply "createaddition mode $STACK 101 0 none" "$MODE_REPLY"
# Empty both: a connector in `none` above an empty accumulator must stay empty,
# which is the control that nothing but the accumulator's UP output can fill it.
command_reply "createaddition energy $STACK 101 0 0" "$ENERGY_REPLY"
command_reply "createaddition energy $STACK 100 0 0" "$ENERGY_REPLY"
ctl wait 10
info "$STACK" 100 0
EMPTY_FE=$energy
info "$STACK" 101 0
EMPTY_WINDOW=$energy
[ "$EMPTY_FE" = 0 ] && [ "$EMPTY_WINDOW" = 0 ] \
    || fail "with both empty the pair reads $EMPTY_FE/$EMPTY_WINDOW, so something else fills the window"
seed_at "$STACK" 100 10000
ctl wait 12
info "$STACK" 100 0
FE=$energy
info "$STACK" 101 0
WINDOW=$energy
[ "$WINDOW" -gt 0 ] || fail "the block above took nothing from the accumulator below"
[ "$WINDOW" = 1000 ] \
    || fail "the block above holds $WINDOW FE, expected its 1,000 FE window filled by the accumulator"
[ $((FE + WINDOW)) = 10000 ] \
    || fail "the accumulator and the block above total $((FE + WINDOW)) FE, not the 10000 seeded"
echo "  accumulator 10000->$FE, the connector above 0->$WINDOW, total conserved at $((FE + WINDOW))"

tower_info() { # tower_info X Y
    info "$1" "$2" 0
    [ "$kind" = accumulator ] || fail "the accumulator at $1,$2 reads as $kind"
}

echo "== G: three stacked accumulators form one tower of 6,000,000 FE =="
# Upstream's multiblock (`CAConnectivityHandler`): a contiguous box up to 3x3 wide and 5 high is
# one controller, capacity 2,000,000 FE a block. The pool is what every part answers with, so a
# seed larger than one block's 2,000,000 FE is only possible once the tower has formed.
TOWER=$((X0 + 20))
SINK=$((TOWER + 6))
TC1=$((TOWER - 1))
TC2=$((SINK - 1))
place_accumulator "$TOWER" 100
place_accumulator "$TOWER" 101
place_accumulator "$TOWER" 102
ctl wait 8
# The visible shape: the jar's own `top`/`bottom` bits, read from the game's block state.
expect_bits "$TOWER" 100 false true
expect_bits "$TOWER" 101 false false
expect_bits "$TOWER" 102 true false
tower_info "$TOWER" 100
[ "$capacity" = 6000000 ] || fail "a three-high tower's capacity is $capacity, expected 6000000"
[ "$energy" = 0 ] || fail "a fresh tower holds $energy FE"
[ "$tower_w" = 1 ] && [ "$tower_h" = 3 ] \
    || fail "the tower reads ${tower_w}x${tower_h}, expected 1x3"
[ "$tower_x" = "$TOWER" ] && [ "$tower_y" = 100 ] \
    || fail "the tower's controller reads $tower_x,$tower_y, expected $TOWER,100"
echo "  tower 1x3 at $TOWER,100: $energy/$capacity FE, bits bottom/middle/top as the jar sets them" >&2

echo "== G2: one pool — 5,000,000 FE into the top block is read from every part =="
seed_at "$TOWER" 102 5000000
tower_info "$TOWER" 102
[ "$energy" = 5000000 ] || fail "the top block reports $energy FE of the 5000000 seeded, not the tower's pool"
tower_info "$TOWER" 100
[ "$energy" = 5000000 ] || fail "the bottom block reports $energy FE, not the 5000000 seed"
[ "$capacity" = 6000000 ] || fail "the bottom block reports a capacity of $capacity"
echo "  the top and the bottom both report 5000000/6000000: one pool behind every block" >&2

echo "== G3: breaking the top re-forms the tower at four million =="
# The seed filled the tower bottom-first: 2,000,000 in each of the lower two blocks and the
# remaining 1,000,000 in the top one, so removing the top leaves exactly what the two kept.
ctl command "execute in minecraft:the_end run setblock $TOWER 102 0 minecraft:air" >/dev/null
ctl wait 8
expect_bits "$TOWER" 100 false true
expect_bits "$TOWER" 101 true false
tower_info "$TOWER" 101
[ "$capacity" = 4000000 ] || fail "the two-high tower's capacity is $capacity, expected 4000000"
[ "$energy" = 4000000 ] || fail "the two-high tower holds $energy FE, expected the 4000000 its blocks kept"
echo "  two-high tower: $energy/$capacity FE, one block's 1,000,000 left with the broken block" >&2

echo "== G4: a formed tower serves a wire from its pooled charge =="
place_accumulator "$SINK" 100
# Seed before the wire exists, so no connector is draining the tower while the command runs.
seed_at "$TOWER" 101 4000000
tower_info "$TOWER" 100
[ "$energy" = 4000000 ] || fail "the tower reports $energy FE after seeding 4000000"
place_connector "$TC1" east createaddition:large_connector
place_connector "$TC2" east createaddition:large_connector
set_mode "$TC2" push
link "$TC1" "$TC2" gold
[ "$LAST_REPLY" = "Nodes linked." ] || fail "the wire onto the tower was not laid: $LAST_REPLY"

# The tower's charge, the sink's and the wire's pool as one total: both connectors parked in
# `none` first, so the three reads are of one instant and nothing moves between them.
tower_measure() { # tower_measure -> TOWER_FE SINK_FE POOL_FE TOTAL_FE
    command_reply "createaddition mode $TC1 100 0 none" "$MODE_REPLY"
    command_reply "createaddition mode $TC2 100 0 none" "$MODE_REPLY"
    tower_info "$TOWER" 100
    TOWER_FE=$energy
    tower_info "$SINK" 100
    SINK_FE=$energy
    info "$TC1" 100 0
    POOL_FE=$((pool_ready + pool_next))
    TOTAL_FE=$((TOWER_FE + SINK_FE + POOL_FE))
    command_reply "createaddition mode $TC1 100 0 pull" "$MODE_REPLY"
    command_reply "createaddition mode $TC2 100 0 push" "$MODE_REPLY"
}

tower_measure
BEFORE_TOWER=$TOWER_FE
BEFORE_SINK=$SINK_FE
BEFORE_TOTAL=$TOTAL_FE
[ "$BEFORE_TOTAL" = 4000000 ] \
    || fail "the tower, the sink and the wire total $BEFORE_TOTAL FE, not the 4000000 seeded"
ctl wait "$TICKS"
tower_measure
MOVED=$((BEFORE_TOWER - TOWER_FE))
GAINED=$((SINK_FE - BEFORE_SINK))
echo "  tower $BEFORE_TOWER->$TOWER_FE, sink $BEFORE_SINK->$SINK_FE, pool $POOL_FE, total $BEFORE_TOTAL->$TOTAL_FE" >&2
[ "$MOVED" -gt 0 ] || fail "nothing left the formed tower over $TICKS ticks"
[ "$GAINED" -gt 0 ] || fail "the sink took nothing from the formed tower"
[ "$GAINED" -le "$MOVED" ] \
    || fail "the sink gained $GAINED FE while the tower lost $MOVED"
[ "$TOTAL_FE" = "$BEFORE_TOTAL" ] \
    || fail "FE was not conserved across the tower's wire: $BEFORE_TOTAL -> $TOTAL_FE"
echo "  the tower's pool served the wire and FE was conserved at $TOTAL_FE" >&2
unlink "$TC1" "$TC2"
[ "$LAST_REPLY" = "Wire removed." ] || fail "the tower's wire was not removed: $LAST_REPLY"

echo "== G5: CONTROL — three separated accumulators stay singles of 2,000,000 FE =="
GAP=$((TOWER + 10))
place_accumulator "$GAP" 100
place_accumulator "$GAP" 102
place_accumulator "$GAP" 104
ctl wait 8
expect_bits "$GAP" 100 true true
expect_bits "$GAP" 102 true true
tower_info "$GAP" 102
[ "$capacity" = 2000000 ] \
    || fail "a separated accumulator reports $capacity FE capacity, expected the single block's 2000000"
seed_at "$GAP" 102 5000000
tower_info "$GAP" 102
SINGLE_FE=$energy
[ "$SINGLE_FE" = 2000000 ] \
    || fail "a separated accumulator took $SINGLE_FE FE from a 5000000 seed; a single block holds 2000000"
tower_info "$GAP" 104
NEIGHBOUR_FE=$energy
[ "$NEIGHBOUR_FE" = 0 ] || fail "the block two above holds $NEIGHBOUR_FE FE, so the three are not separate"
echo "  separate stack seeded 5000000 holds $SINGLE_FE/2000000, the block two above $NEIGHBOUR_FE" >&2
if [ "$SELFTEST" = 1 ]; then
    # The deliberately wrong expectation: a tower of three MUST not report four million. It
    # does not, so this leg fails — which is the proof the tower assertions can fail.
    [ "$capacity" = 4000000 ] || fail "SELFTEST: the tower assertion did not trip (capacity $capacity)"
fi

echo "== G6: a doctored saved tower is refused before capacity arithmetic =="
# The block entity's saved compound IS its NBT root (`tw`/`th`/`tx`/`ty`/`tz`, `energy`), so a
# save can be edited with vanilla `/data merge block`. A shape outside the 3x3x5 limit must be
# read as the block's own single: `1000*1000*1000` would overflow the capacity and make
# `members()` allocate a billion entries. `tick freeze` holds the level pass (which would
# re-form the tower) off so the read sees the doctored data; the probe in
# `ports/_shared/evidence/createaddition/probe-doctored.log` established both facts.
DOCTOR=$((X0 + 24))
place_accumulator "$DOCTOR" 100
ctl wait 8
expect_bits "$DOCTOR" 100 true true
command_reply "createaddition energy $DOCTOR 100 0 123" "$ENERGY_REPLY"
tower_info "$DOCTOR" 100
[ "$capacity" = 2000000 ] && [ "$energy" = 123 ] \
    || fail "the fresh doctor block reads $energy/$capacity, expected 123/2000000"
ctl command "execute in minecraft:the_end run tp @s $DOCTOR.5 101 0.5" >/dev/null
ctl wait 4
ctl command "tick freeze" >/dev/null
ctl command "execute in minecraft:the_end run data merge block $DOCTOR 100 0 {tw:4,th:1,tx:$DOCTOR,ty:100,tz:0}" >/dev/null
ctl command "execute in minecraft:the_end run data get block $DOCTOR 100 0" \
    | jq -e '.chat[0] | contains("tw: 4") and contains("th: 1")' >/dev/null \
    || fail "the doctored 4x1 tower was not written into the block entity"
frozen_info "$DOCTOR" 100
[ "$capacity" = 2000000 ] && [ "$energy" = 123 ] \
    || fail "a doctored 4x1 save reads $energy/$capacity, not the block's own 123/2000000"
case "$INFO_LINE" in
    *"tower 4x1"*) fail "the readout echoed the doctored 4x1 tower" ;;
esac
# The 1000x1000 shape is the same code path the unit tests measure without a game (capacity
# overflow and a billion-entry `members()` on the old code); assert the live read stays single.
ctl command "execute in minecraft:the_end run data merge block $DOCTOR 100 0 {tw:1000,th:1000,tx:$DOCTOR,ty:100,tz:0}" >/dev/null
frozen_info "$DOCTOR" 100
[ "$capacity" = 2000000 ] && [ "$energy" = 123 ] \
    || fail "a doctored 1000x1000 save reads $energy/$capacity, not the block's own 123/2000000"
echo "  doctored tw/th 4x1 and 1000x1000 both read as the block's own 123/2000000" >&2
ctl command "tick unfreeze" >/dev/null
# The next pass re-forms the legal single: the doctored shape does not survive it.
ctl wait 8
expect_bits "$DOCTOR" 100 true true
tower_info "$DOCTOR" 100
[ "$capacity" = 2000000 ] || fail "the doctored tower survived the pass ($capacity)"
echo "  the next pass re-formed it as a 1x1 single" >&2

echo "== H: every block's loot table drops the block itself (the 26.3 dialect) =="
# 1.21.1 writes a pool's condition as `"conditions": [ {...} ]` with the kind under
# `"condition"`; 26.3's own tables (the prepared resources) write one `"condition":
# {"type": ...}` per pool and have no `bonus_rolls`. Carried verbatim, the 1.21.1 shape made a
# block drop nothing (`modularrouters` measured 0 while a vanilla table beside it dropped 1).
for LOOT_BLOCK in createaddition:connector createaddition:large_connector createaddition:small_light_connector createaddition:modular_accumulator createaddition:redstone_relay createaddition:creative_energy; do
    break_drop_leg "$LOOT_BLOCK"
    [ "$DROPPED" = 1 ] \
        || fail "mining $LOOT_BLOCK dropped $DROPPED of itself, expected the one its 26.3 loot table names"
    ctl command "execute at @s run setblock ~2 ~ ~ minecraft:air" >/dev/null
done
echo "  all six blocks drop one of themselves through their 26.3 loot tables" >&2

echo "== I: the redstone relay gates a wire between two accumulators =="
# The jar's relay (`RedstoneRelayBlockEntity`, `WireGraph.tickBridges`): eight wire nodes in
# two ports, 0-3 input and 4-7 output, and energy crosses from its input network to its
# output network only while the block's `powered` bit is set. The rig is
# A - C1 - relay - C2 - B on its own stone pad, away from the arena's device blocks; the
# relay is flat and faces east, so C1 (west) is its input and C2 (east) its output.
RIG_X=$((X0 + 34))
RIG_A=$((RIG_X - 2))
RIG_C1=$((RIG_X - 1))
RIG_C2=$((RIG_X + 2))
RIG_B=$((RIG_X + 3))
ctl command "execute in minecraft:the_end run fill $((RIG_A - 1)) 100 -2 $((RIG_B + 1)) 100 2 minecraft:stone" >/dev/null
ctl command "execute in minecraft:the_end run fill $((RIG_A - 1)) 101 -2 $((RIG_B + 1)) 104 2 minecraft:air" >/dev/null
ctl wait 6 >/dev/null
place_accumulator "$RIG_A"
place_accumulator "$RIG_B"
place_connector "$RIG_C1" west
place_connector "$RIG_C2" east
ctl command "execute in minecraft:the_end run setblock $RIG_X 100 0 createaddition:redstone_relay[facing=east,vertical=false,powered=false]" >/dev/null
ctl wait 6 >/dev/null
[ "$(block_name "$RIG_X" 100 0)" = "createaddition:redstone_relay" ] \
    || fail "the relay did not place at $RIG_X,100 (got '$(block_name "$RIG_X" 100 0)')"
RELAY_STATE=$(block_state "$RIG_X" 100 0)
case "$RELAY_STATE" in
    *powered=false*) echo "  $RELAY_STATE" >&2 ;;
    *) fail "a freshly placed relay reads '$RELAY_STATE', expected powered=false" ;;
esac
# C1 sits on the source accumulator and settles `pull` by itself; C2 must push into the sink.
info "$RIG_C1" 100 0
[ "$mode" = pull ] || fail "the source-side connector took mode '$mode', expected pull"
set_mode "$RIG_C2" push
link "$RIG_C1" "$RIG_X" copper
[ "$LAST_REPLY" = "Nodes linked." ] || fail "the wire onto the relay's input was not laid: $LAST_REPLY"
link "$RIG_X" "$RIG_C2" copper
[ "$LAST_REPLY" = "Nodes linked." ] || fail "the wire off the relay's output was not laid: $LAST_REPLY"
info "$RIG_X" 100 0
[ "$kind" = relay ] || fail "the relay reads as '$kind'"
[ "$input_links" = 1 ] && [ "$output_links" = 1 ] \
    || fail "the relay holds $input_links input and $output_links output wire(s), expected one each"
echo "  relay input [(233,100,0)] and output [(236,100,0)], powered=false" >&2

# The source, the sink, both network pools and the relay's in-flight buffer, read as one
# instant: both connectors are parked in `none`, and for a powered read the redstone block is
# taken away first so the bridge cannot shift energy between the five reads.
relay_measure() { # relay_measure POWERED(0|1) -> R_SOURCE R_SINK R_POOL1 R_POOL2 R_RELAY R_TOTAL
    # Not named `powered`: `info` evals the relay's `powered=...` field, and bash's dynamic
    # scoping would put it into this function's local of the same name.
    local with_power=$1
    if [ "$with_power" = 1 ]; then
        ctl command "execute in minecraft:the_end run setblock $RIG_X 100 1 minecraft:air" >/dev/null
        ctl wait 4 >/dev/null
    fi
    command_reply "createaddition mode $RIG_C1 100 0 none" "$MODE_REPLY"
    command_reply "createaddition mode $RIG_C2 100 0 none" "$MODE_REPLY"
    info "$RIG_A" 100 0
    R_SOURCE=$energy
    info "$RIG_B" 100 0
    R_SINK=$energy
    info "$RIG_C1" 100 0
    R_POOL1=$((pool_ready + pool_next))
    info "$RIG_C2" 100 0
    R_POOL2=$((pool_ready + pool_next))
    info "$RIG_X" 100 0
    R_RELAY=$energy
    # The relay is the output network's lowest member, so its compound holds that network's
    # pool too; the relay's line prints it.
    R_NET=$((pool_ready + pool_next))
    R_TOTAL=$((R_SOURCE + R_SINK + R_POOL1 + R_POOL2 + R_RELAY + R_NET))
    command_reply "createaddition mode $RIG_C1 100 0 pull" "$MODE_REPLY"
    command_reply "createaddition mode $RIG_C2 100 0 push" "$MODE_REPLY"
    if [ "$with_power" = 1 ]; then
        ctl command "execute in minecraft:the_end run setblock $RIG_X 100 1 minecraft:redstone_block" >/dev/null
        ctl wait 4 >/dev/null
        echo "  (measurement restored the power: $(block_state "$RIG_X" 100 0))" >&2
    fi
}

seed_at "$RIG_A" 100 "$SEED"
relay_measure 0
UNPOWERED_SINK=$R_SINK
UNPOWERED_TOTAL=$R_TOTAL
ctl wait "$TICKS"
relay_measure 0
echo "  unpowered: source $R_SOURCE, sink $UNPOWERED_SINK->$R_SINK, pools $R_POOL1/$R_POOL2/$R_NET, relay $R_RELAY, total $UNPOWERED_TOTAL->$R_TOTAL" >&2
if [ "$SELFTEST" = 1 ] || [ "${RELAY_SELFTEST:-0}" = 1 ]; then
    # The deliberately wrong expectation: an unpowered relay MUST NOT carry a single FE.
    # It does not, so this leg fails — the counterfactual for the gate assertion below.
    [ "$R_SINK" != "$UNPOWERED_SINK" ] || fail "RELAY_SELFTEST: the unpowered relay moved the sink"
fi
[ "$R_SINK" = "$UNPOWERED_SINK" ] \
    || fail "the unpowered relay let $((R_SINK - UNPOWERED_SINK)) FE into the sink over $TICKS ticks"
[ "$R_TOTAL" = "$UNPOWERED_TOTAL" ] \
    || fail "FE was not conserved with the relay unpowered: $UNPOWERED_TOTAL -> $R_TOTAL"

echo "  powering the relay with a redstone block on its side"
ctl command "execute in minecraft:the_end run setblock $RIG_X 100 1 minecraft:redstone_block" >/dev/null
ctl wait 6 >/dev/null
RELAY_STATE=$(block_state "$RIG_X" 100 0)
case "$RELAY_STATE" in
    *powered=true*) echo "  $RELAY_STATE" >&2 ;;
    *) fail "the relay did not read the redstone block: '$RELAY_STATE'" ;;
esac
relay_measure 1
POWERED_SOURCE=$R_SOURCE
POWERED_SINK=$R_SINK
POWERED_TOTAL=$R_TOTAL
ctl wait "$TICKS"
relay_measure 1
MOVED=$((POWERED_SOURCE - R_SOURCE))
GAINED=$((R_SINK - POWERED_SINK))
echo "  powered: source $POWERED_SOURCE->$R_SOURCE, sink $POWERED_SINK->$R_SINK, pools $R_POOL1/$R_POOL2/$R_NET, relay $R_RELAY, total $POWERED_TOTAL->$R_TOTAL" >&2
[ "$MOVED" -gt 0 ] || fail "nothing left the source over $TICKS ticks with the relay powered"
[ "$GAINED" -gt 0 ] || fail "the sink gained nothing over $TICKS ticks with the relay powered"
[ "$GAINED" -le "$MOVED" ] || fail "the sink gained $GAINED FE while the source lost $MOVED"
[ "$R_TOTAL" = "$POWERED_TOTAL" ] \
    || fail "FE was not conserved across the powered relay: $POWERED_TOTAL -> $R_TOTAL"
echo "  the powered relay carried $GAINED FE to the sink, conserved at $R_TOTAL" >&2

echo "== J: the small light connector lights on its network's one FE a tick =="
# Upstream (`SmallLightConnectorBlockEntity`): a small connector whose lamp asks its wire
# network for one FE a tick and keeps a counter, +1 while the network pays it and -1 while it
# does not, clamped to `10 + (|x*31+y*45+z*33| % 7)*3` either way. At the top clamp it sets
# `powered` and the block emits light 15; at the bottom clamp it clears it. No Create, no GUI:
# the lamp's energy is the network's, so the family is a conservation rule plus a block state.
LAMP_A=$((X0 + 50))
LAMP_C1=$((LAMP_A + 1))
LAMP_L=$((LAMP_A + 4))
LAMP_WALL=$((LAMP_L + 1))
ctl command "execute in minecraft:the_end run fill $((LAMP_A - 1)) 100 -2 $((LAMP_WALL + 1)) 100 2 minecraft:stone" >/dev/null
ctl command "execute in minecraft:the_end run fill $((LAMP_A - 1)) 101 -2 $((LAMP_WALL + 1)) 104 2 minecraft:air" >/dev/null
ctl wait 6 >/dev/null
# Stand on the pad beside the rig, so every read in this leg is within a player's reach even
# if the run's operator level is not what the earlier legs assumed.
ctl command "execute in minecraft:the_end run tp @s $((LAMP_A + 2)).5 101 0.5" >/dev/null
ctl wait 4 >/dev/null
place_accumulator "$LAMP_A"
place_connector "$LAMP_C1" west
ctl command "execute in minecraft:the_end run setblock $LAMP_L 100 0 createaddition:small_light_connector[facing=east,mode=none,powered=false]" >/dev/null
ctl wait 6 >/dev/null
[ "$(block_name "$LAMP_L" 100 0)" = "createaddition:small_light_connector" ] \
    || fail "the lamp did not place at $LAMP_L,100 (got '$(block_name "$LAMP_L" 100 0)')"

# The jar's own period for this block position, as the block entity computes it.
lamp_period_at() { # X Y Z -> period in ticks
    local lx=$1 ly=$2 lz=$3 seed
    seed=$(( lx * 31 + ly * 45 + lz * 33 ))
    [ "$seed" -ge 0 ] || seed=$(( -seed ))
    echo $(( 10 + (seed % 7) * 3 ))
}
# `ctl light` answers one hex digit of stored block light (f = 15).
lamp_light() { ctl light "$1" "$2" "$3" "$1" "$2" "$3" | jq -r '.block // "unknown"'; }
# The lamp's state string and its own light, asserted together: the bit and the light engine
# must agree, and `ctl light` reads the engine, not the mod's readout.
expect_lamp() { # expect_lamp powered(true|false) light(hex)
    local state light
    state=$(block_state "$LAMP_L" 100 0)
    case "$state" in
        *"powered=$1"*) ;;
        *) fail "the lamp reads '$state', expected powered=$1" ;;
    esac
    light=$(lamp_light "$LAMP_L" 100 0)
    [ "$light" = "$2" ] || fail "the lamp's block light is '$light', expected '$2'"
    echo "  $state, block light $light" >&2
}

LAMP_PERIOD=$(lamp_period_at "$LAMP_L" 100 0)
echo "  the lamp at $LAMP_L,100,0 cycles in $LAMP_PERIOD ticks (the jar's position-seeded period)" >&2
info "$LAMP_L" 100 0
[ "$kind" = light ] || fail "the lamp reads as '$kind'"
[ "$capacity" = 1000 ] || fail "a lamp's window is $capacity, expected the small connector's 1000"
[ "$lamp_period" = "$LAMP_PERIOD" ] \
    || fail "the lamp's readout reports a period of $lamp_period, the jar's formula says $LAMP_PERIOD"
[ "$mode" = none ] || fail "a lamp mounted on stone settled mode '$mode', expected none"

echo "  -- at 0 FE the lamp is dark and its counter sits at the bottom clamp"
ctl wait "$((LAMP_PERIOD + 5))"
expect_lamp false 0
info "$LAMP_L" 100 0
[ "$lit" = 0 ] || fail "an unpaid lamp reports itself lit"
[ "$lamp_timer" = "-$LAMP_PERIOD" ] \
    || fail "an unpaid lamp's counter is $lamp_timer, expected the bottom clamp -$LAMP_PERIOD"

echo "  -- seeding 5,000 FE and linking the wire lights it"
seed_at "$LAMP_A" 100 5000
link "$LAMP_C1" "$LAMP_L" copper
[ "$LAST_REPLY" = "Nodes linked." ] || fail "the lamp's wire was not laid: $LAST_REPLY"
# The dark leg left the counter at the bottom clamp, so it needs a full downswing
# (`2 * period` ticks) to reach the top one: upstream clamps at ±posTimeOffset, and only the
# top clamp lights the block.
ctl wait "$((2 * LAMP_PERIOD + 5))"
expect_lamp true f
info "$LAMP_L" 100 0
[ "$lit" = 1 ] || fail "a paid lamp reports itself dark"
[ "$lamp_timer" = "$LAMP_PERIOD" ] \
    || fail "a lit lamp's counter is $lamp_timer, expected the top clamp $LAMP_PERIOD"

echo "  -- it draws exactly one FE a tick, and the network's pool holds"
info "$LAMP_A" 100 0
LAMP_S1=$energy
LAMP_T1=$t
# The network's buffer lives in its lowest member's compound: the connector, not the lamp.
info "$LAMP_C1" 100 0
LAMP_P1=$((pool_ready + pool_next))
[ "$links" = 1 ] || fail "the lamp's connector holds $links wire(s), expected one"
ctl wait "$TICKS"
info "$LAMP_C1" 100 0
LAMP_P2=$((pool_ready + pool_next))
info "$LAMP_A" 100 0
LAMP_S2=$energy
LAMP_T2=$t
LAMP_TICKS=$((LAMP_T2 - LAMP_T1))
LAMP_DRAWN=$((LAMP_S1 - LAMP_S2))
[ "$LAMP_TICKS" -ge "$TICKS" ] || fail "only $LAMP_TICKS ticks passed across the lamp's reading"
[ "$LAMP_P1" = "$LAMP_P2" ] \
    || fail "the lamp's network pool moved $LAMP_P1->$LAMP_P2 FE across the reading, so the source is not the only account"
[ "$LAMP_DRAWN" = "$LAMP_TICKS" ] \
    || fail "the source lost $LAMP_DRAWN FE over $LAMP_TICKS ticks, expected exactly one a tick"
[ "$LAMP_S2" -le 5000 ] || fail "the source holds $LAMP_S2 FE of the 5000 seeded"
expect_lamp true f
echo "  source $LAMP_S1->$LAMP_S2 over $LAMP_TICKS ticks: $LAMP_DRAWN FE, one a tick, pool steady at $LAMP_P2" >&2

echo "  -- CONTROL: unlinked, the lamp draws nothing and the source does not move"
unlink "$LAMP_C1" "$LAMP_L"
[ "$LAST_REPLY" = "Wire removed." ] || fail "the lamp's wire was not removed: $LAST_REPLY"
info "$LAMP_A" 100 0
LAMP_S3=$energy
ctl wait "$((2 * LAMP_PERIOD + 5))"
expect_lamp false 0
info "$LAMP_A" 100 0
LAMP_S4=$energy
[ "$LAMP_S4" = "$LAMP_S3" ] \
    || fail "the unlinked lamp drew $((LAMP_S3 - LAMP_S4)) FE from the source"
echo "  unlinked: the lamp went dark and the source stayed at $LAMP_S4 FE" >&2
if [ "${LAMP_SELFTEST:-0}" = 1 ]; then
    # The deliberately wrong expectation: an unlinked lamp MUST have drawn the source. It did
    # not, so this leg fails — the counterfactual for the assertion above.
    [ "$LAMP_S3" != "$LAMP_S4" ] || fail "LAMP_SELFTEST: the unlinked lamp drew nothing, so the assertion is alive"
fi

echo "== K: the accumulator answers a comparator with the jar's fraction =="
# Upstream (`ModularAccumulatorBlock.hasAnalogOutputSignal`/`getAnalogOutputSignal`): a comparator
# reads the controller's `getFillState()` and maps it with
# `ComparatorUtil.fractionToRedstoneLevel`, so the answer is the **tower's** fraction, not one
# block's. The assertion is the wire's own `power` state from `ctl block`, so it is the world's
# answer and not the mod's readout, and the middle seed is the jar's real midpoint (8): a mapping
# that only told empty from non-empty would read 15 there, and the un-offset fraction reads 7.
CMP_A=$((X0 + 60))        # the tower's x: a 1x3 column, so the pool is 6,000,000 FE
CMP_S=$((CMP_A + 3))      # the differential: the same rig behind a stone block
CMP_Y=100
ctl command "execute in minecraft:the_end run fill $((CMP_A - 1)) 100 -2 $((CMP_S + 1)) 100 3 minecraft:stone" >/dev/null
ctl command "execute in minecraft:the_end run fill $((CMP_A - 1)) 101 -2 $((CMP_S + 1)) 104 3 minecraft:air" >/dev/null
ctl wait 6 >/dev/null
ctl command "execute in minecraft:the_end run tp @s $((CMP_A + 1)).5 101 2.5" >/dev/null
ctl wait 4 >/dev/null
place_accumulator "$CMP_A" 100
place_accumulator "$CMP_A" 101
place_accumulator "$CMP_A" 102
ctl wait 6 >/dev/null
info "$CMP_A" "$CMP_Y" 0
[ "$kind" = accumulator ] || fail "the comparator rig's tower reads as '$kind'"
[ "$capacity" = 6000000 ] || fail "the comparator rig's tower holds $capacity, expected a 1x3 tower's 6,000,000"
# A comparator reads the block on its `facing` side and drives the block behind it: the CMP_ONLY
# probe measured both mirror orientations and the wire on the opposite side of `facing` is the one
# that carries the signal.
ctl command "execute in minecraft:the_end run setblock $CMP_A $((CMP_Y + 1)) 1 minecraft:comparator[facing=north]" >/dev/null
ctl command "execute in minecraft:the_end run setblock $CMP_A $((CMP_Y + 1)) 2 minecraft:redstone_wire" >/dev/null
ctl command "execute in minecraft:the_end run setblock $CMP_S 101 0 minecraft:stone" >/dev/null
ctl command "execute in minecraft:the_end run setblock $CMP_S 101 1 minecraft:comparator[facing=north]" >/dev/null
ctl command "execute in minecraft:the_end run setblock $CMP_S 101 2 minecraft:redstone_wire" >/dev/null
ctl wait 8 >/dev/null
[ "$(block_name "$CMP_A" 101 1)" = minecraft:comparator ] \
    || fail "the comparator did not place at $CMP_A,101,1"
[ "$(block_name "$CMP_A" 101 2)" = minecraft:redstone_wire ] \
    || fail "the comparator's wire did not place at $CMP_A,101,2"
[ "$(block_name "$CMP_S" 101 1)" = minecraft:comparator ] \
    || fail "the differential comparator did not place at $CMP_S,101,1"
[ "$(block_name "$CMP_S" 101 2)" = minecraft:redstone_wire ] \
    || fail "the differential wire did not place at $CMP_S,101,2"

echo "  -- an empty 1x3 tower reads power=0"
expect_wire "$CMP_A" 101 2 0 "an empty tower"

echo "  -- 3,000,000 of 6,000,000 reads the jar's 8 (7 is the un-offset fraction, 15 a 0/15 mapping)"
seed_at "$CMP_A" "$CMP_Y" 3000000
ctl wait 8 >/dev/null
info "$CMP_A" "$CMP_Y" 0
[ "$energy" = 3000000 ] || fail "the tower holds $energy FE after seeding 3,000,000"
expect_wire "$CMP_A" 101 2 8 "a half-full tower"

echo "  -- a full tower reads power=15"
seed_at "$CMP_A" "$CMP_Y" 6000000
ctl wait 8 >/dev/null
info "$CMP_A" "$CMP_Y" 0
[ "$energy" = 6000000 ] || fail "the tower holds $energy FE after seeding 6,000,000"
expect_wire "$CMP_A" 101 2 15 "a full tower"
CMP_FULL=$(wire_power "$CMP_A" 101 2)

echo "  -- the differential: the same comparator rig behind a stone block reads 0"
[ "$(block_name "$CMP_S" 101 0)" = minecraft:stone ] \
    || fail "the differential block is not stone"
expect_wire "$CMP_S" 101 2 0 "a stone block"

echo "  -- emptied with /createaddition energy, the wire falls back to 0"
seed_at "$CMP_A" "$CMP_Y" 0
ctl wait 8 >/dev/null
CMP_EMPTY=$(wire_power "$CMP_A" 101 2)
[ -n "$CMP_EMPTY" ] || fail "the comparator's wire vanished: '$(block_state "$CMP_A" 101 2)'"
[ "$CMP_EMPTY" = 0 ] || fail "emptying the tower left the wire at power=$CMP_EMPTY"
echo "  emptied: redstone_wire at $CMP_A,101,2 power=$CMP_EMPTY" >&2
if [ "${COMPARATOR_SELFTEST:-0}" = 1 ]; then
    # The deliberately wrong expectation: emptying the tower must have left the wire where it was.
    # It did not, so this leg fails — the counterfactual for the assertion above.
    [ "$CMP_EMPTY" = "$CMP_FULL" ] \
        || fail "COMPARATOR_SELFTEST: emptying the tower moved the wire from power=$CMP_FULL to power=$CMP_EMPTY, so the assertion is alive"
fi

echo "== L: the creative generator gives without limit; a wire carries 1,000 FE a tick =="
# Upstream's `CreativeEnergyStorage` answers `Integer.MAX_VALUE` stored and capacity, gives
# whatever is asked and takes nothing (`canExtract` true, `canReceive` false), and
# `CreativeEnergyBlockEntity.tick` offers every neighbouring storage
# `receiveEnergy(Integer.MAX_VALUE, false)` each tick. A connector mounted on the generator
# therefore settles `pull`, and the wire carries the connector's 1,000 FE a tick to the sink.
L_CREATIVE=$((X0 - 30))
L_C1=$((L_CREATIVE + 1))
L_C2=$((L_C1 + 4))
L_ACC=$((L_C2 + 1))
L_DIRECT_CREATIVE=$((L_CREATIVE - 4))
L_DIRECT_ACC=$((L_DIRECT_CREATIVE + 1))
ctl command "execute in minecraft:the_end run fill $((L_DIRECT_CREATIVE - 2)) 100 -2 $((L_ACC + 1)) 100 2 minecraft:stone" >/dev/null
ctl command "execute in minecraft:the_end run fill $((L_DIRECT_CREATIVE - 2)) 101 -2 $((L_ACC + 1)) 104 2 minecraft:air" >/dev/null
ctl wait 6 >/dev/null
ctl command "execute in minecraft:the_end run tp @s $((L_C1 + 2)).5 101 0.5" >/dev/null
ctl wait 4 >/dev/null
place_block "$L_CREATIVE" 100 createaddition:creative_energy createaddition:creative_energy
place_connector "$L_C1" west
place_accumulator "$L_ACC"
place_connector "$L_C2" east
info "$L_C1" 100 0
[ "$kind" = connector ] || fail "the generator's connector has no block entity (info says $kind)"
[ "$mode" = pull ] \
    || fail "a connector beside the creative generator took mode '$mode', expected pull: the generator gives and cannot take"
case "$(block_state "$L_C1" 100 0)" in
    *"mode=pull"*) echo "  the connector's own state reads $(block_state "$L_C1" 100 0)" >&2 ;;
    *) fail "the connector beside the generator shows '$(block_state "$L_C1" 100 0)', expected mode=pull" ;;
esac
info "$L_CREATIVE" 100 0
[ "$kind" = creative ] || fail "the generator's own readout says '$kind', expected the Creative Generator"
info "$L_ACC" 100 0
[ "$kind" = accumulator ] || fail "the phase-L sink is a '$kind'"
[ "$energy" = 0 ] || fail "the sink accumulator starts at $energy FE, expected empty"
set_mode "$L_C2" push
link "$L_C1" "$L_C2" gold
[ "$LAST_REPLY" = "Nodes linked." ] || fail "the generator's wire was not laid: $LAST_REPLY"
# Prime the pool's double buffer so the one-time window fill is outside the window below: the
# first tick delivers nothing and every tick after it delivers the connector's 1,000 FE.
ctl wait 10 >/dev/null
info "$L_ACC" 100 0
L_BEFORE=$energy
L_T0=$t
ctl wait "$TICKS"
info "$L_ACC" 100 0
L_GAIN=$((energy - L_BEFORE))
L_ELAPSED=$((t - L_T0))
[ "$L_ELAPSED" -ge "$TICKS" ] \
    || fail "only $L_ELAPSED game ticks passed between the two reads, expected at least $TICKS"
[ "$L_GAIN" -ge $((RATE * TICKS)) ] \
    || fail "the sink gained $L_GAIN FE over $L_ELAPSED ticks, less than the ${RATE}/tick the connector carries in $TICKS ticks"
[ "$L_GAIN" = $((RATE * L_ELAPSED)) ] \
    || fail "the sink gained $L_GAIN FE over $L_ELAPSED game ticks, not exactly the ${RATE}/tick a small connector carries"
if [ "${CREATIVE_SELFTEST:-0}" = 1 ]; then
    # The deliberately wrong expectation: the generator MUST have moved nothing. It did, so this
    # leg fails — the counterfactual for the assertion above.
    [ "$L_GAIN" = 0 ] \
        || fail "CREATIVE_SELFTEST: the creative generator supplied $L_GAIN FE, so the assertion is alive"
fi
echo "  the connector pulls from the generator and the sink gained $L_GAIN FE over $L_ELAPSED game ticks (exactly ${RATE}/tick; $((RATE * TICKS)) FE in the $TICKS-tick window)" >&2

echo "  -- the differential: stone where the generator was, the same wire, moves nothing"
place_block "$L_CREATIVE" 100 minecraft:stone minecraft:stone
seed_at "$L_ACC" 100 0
ctl wait 6 >/dev/null
info "$L_ACC" 100 0
L_STONE_BEFORE=$energy
L_STONE_T0=$t
ctl wait "$TICKS"
info "$L_ACC" 100 0
L_STONE_ELAPSED=$((t - L_STONE_T0))
[ "$L_STONE_ELAPSED" -ge "$TICKS" ] \
    || fail "only $L_STONE_ELAPSED ticks passed in the stone window"
[ "$energy" = "$L_STONE_BEFORE" ] \
    || fail "the sink moved $L_STONE_BEFORE->$energy FE with stone where the generator was"
echo "  stone in the generator's place: the sink stayed at $energy FE over $L_STONE_ELAPSED game ticks" >&2

echo "  -- the generator's own tick: an accumulator beside it fills with no connector at all"
ctl command "execute in minecraft:the_end run tp @s $((L_DIRECT_ACC + 1)).5 101 0.5" >/dev/null
ctl wait 4 >/dev/null
place_block "$L_DIRECT_CREATIVE" 100 createaddition:creative_energy createaddition:creative_energy
place_accumulator "$L_DIRECT_ACC"
info "$L_DIRECT_ACC" 100 0
D_BEFORE=$energy
D_T0=$t
ctl wait "$TICKS"
info "$L_DIRECT_ACC" 100 0
D_GAIN=$((energy - D_BEFORE))
D_ELAPSED=$((t - D_T0))
[ "$D_GAIN" -gt 0 ] \
    || fail "an accumulator beside the generator took nothing: the per-tick push did not run"
[ "$D_GAIN" = $((5000 * D_ELAPSED)) ] \
    || fail "the accumulator beside the generator gained $D_GAIN FE over $D_ELAPSED ticks, not the 5,000/tick its own insert caps the generator's offer at"
echo "  the direct pair: the accumulator gained $D_GAIN FE over $D_ELAPSED game ticks at 5,000/tick, no connector involved" >&2

echo "== M: the festive spool lays the jar's fourth wire; festive is low-current like copper =="
# `WireType.FESTIVE(3, 256, 26, 94, 12, copper_wire x4, festive_spool)` is the jar's fourth wire:
# it transfers 256 like copper, `IWireNode.connect` refuses `COPPER` **or** `FESTIVE` between two
# large connectors, and coiling one back up returns a festive spool (`getSourceDrop()`). `IRON(4,
# ...)` is commented out upstream together with `IRON_SPOOL`, so `createaddition:iron_wire` is an
# item with no wire kind and no spool.
#
# The rig hangs at **eye height** (y=102 with the player's feet on the pad at y=101), not in the
# floor row: a connector's outline is the nub on the face it is mounted on (4 x 4 pixels), so a
# click from the front has to be almost horizontal. A player standing beside a rig in the floor
# row is 1.1 blocks above its nub and the descending ray crosses the nub's thin band **above** it,
# hitting nothing at all (two probe runs, `probe-festive.log`); with the rig at y=102 the eye and
# the nub are 0.12 apart and the ray goes through it.
M_A=$((X0 + 68))
M_C1=$((M_A + 1))
M_C2=$((M_C1 + 4))
M_B=$((M_C2 + 1))
M_BIG_A=$((X0 + 80))
M_BIG_B=$((M_BIG_A + 4))
M_Y=102
# The pad is this far from the arena, so its chunks are **not** loaded while the player stands at
# phase L's rig: a `fill`/`setblock` in an unloaded chunk is refused (`ctl` exits 1). Stand on
# phase J's stone pad first — the teleport loads the neighbourhood — then lay this pad.
ctl command "execute in minecraft:the_end run tp @s $((X0 + 52)).5 101 0.5" >/dev/null
ctl wait 10 >/dev/null
ctl command "execute in minecraft:the_end run fill $((M_A - 3)) 100 -2 $((M_BIG_B + 1)) 100 2 minecraft:stone" >/dev/null
ctl command "execute in minecraft:the_end run fill $((M_A - 3)) 101 -2 $((M_BIG_B + 1)) 104 2 minecraft:air" >/dev/null
ctl wait 6 >/dev/null
ctl command "execute in minecraft:the_end run tp @s $((M_C1 + 2)).5 101 0.5" >/dev/null
ctl wait 4 >/dev/null
[ "$(block_name "$((M_A - 3))" 100 0)" = "minecraft:stone" ] \
    || fail "the festive pad was not laid at $((M_A - 3)),100 (got '$(block_name "$((M_A - 3))" 100 0)')"
[ "$(block_name "$M_C1" "$M_Y" 0)" = "minecraft:air" ] \
    || fail "the festive rig's connector cell is not air before placement"
place_accumulator "$M_A" "$M_Y"
place_accumulator "$M_B" "$M_Y"
place_connector "$M_C1" west createaddition:connector "$M_Y"
place_connector "$M_C2" east createaddition:connector "$M_Y"
# The spool is the item a player links with: put it in the hotbar, select it, and click the near
# connector (the first click selects its node) and then the far one (the second lays the wire).
ctl command "item replace entity @s hotbar.0 with createaddition:festive_spool 1" >/dev/null
ctl select 1 >/dev/null
ctl wait 4 >/dev/null
[ "$(item_held createaddition:festive_spool)" = 1 ] \
    || fail "the festive spool item did not register: $(ctl inventory | jq -c '{hotbar,main}')"
ctl interact "$M_C1" "$M_Y" 0 east >/dev/null
ctl wait 2 >/dev/null
ctl interact "$M_C2" "$M_Y" 0 west >/dev/null
ctl wait 6 >/dev/null
info "$M_C1" "$M_Y" 0
[ "$links" = 1 ] || fail "the festive spool laid no wire (the connector holds $links)"
case "$INFO_LINE" in
    *")/0/festive]"*) echo "  the two clicks laid a festive wire: $INFO_LINE" >&2 ;;
    *) fail "the wire the festive spool laid is not saved as festive: $INFO_LINE" ;;
esac
info "$M_C2" "$M_Y" 0
case "$INFO_LINE" in
    *")/0/festive]"*) ;;
    *) fail "the far end of the festive wire does not name festive: $INFO_LINE" ;;
esac
# The player is in survival from phase H's break leg, so this is the real spend path: the last
# wire spool becomes the empty spool **in the hand** and is not handed over a second time (the
# 0.1.0 defect was one spool returning two empty ones).
[ "$(item_held createaddition:festive_spool)" = 0 ] \
    || fail "the festive spool was not used up laying the wire (held $(item_held createaddition:festive_spool))"
[ "$(item_held createaddition:spool)" = 1 ] \
    || fail "the last festive spool did not become exactly one empty spool (held $(item_held createaddition:spool))"
echo "  both ends save type 'festive'; the one festive spool became exactly one empty spool" >&2

echo "  -- it carries exactly 1,000 FE a tick, the same rate as copper"
seed_at "$M_A" "$M_Y" 100000
command_reply "createaddition mode $M_C1 $M_Y 0 pull" "$MODE_REPLY"
command_reply "createaddition mode $M_C2 $M_Y 0 push" "$MODE_REPLY"
# Prime the network's double buffer: the first tick delivers nothing, every tick after it 1,000 FE.
ctl wait 10 >/dev/null
info "$M_B" "$M_Y" 0
M_BEFORE=$energy
M_T0=$t
ctl wait "$TICKS"
info "$M_B" "$M_Y" 0
M_GAIN=$((energy - M_BEFORE))
M_ELAPSED=$((t - M_T0))
[ "$M_ELAPSED" -ge "$TICKS" ] \
    || fail "only $M_ELAPSED game ticks passed between the two reads, expected at least $TICKS"
[ "$M_GAIN" -ge $((RATE * TICKS)) ] \
    || fail "the festive wire delivered $M_GAIN FE over $M_ELAPSED ticks, less than ${RATE}/tick over $TICKS ticks"
[ "$M_GAIN" = $((RATE * M_ELAPSED)) ] \
    || fail "the festive wire delivered $M_GAIN FE over $M_ELAPSED game ticks, not exactly the ${RATE}/tick a small connector carries"
echo "  the festive wire carried $M_GAIN FE over $M_ELAPSED game ticks (exactly ${RATE}/tick; $((RATE * TICKS)) FE in the $TICKS-tick window)" >&2

echo "  -- the differential: two large connectors refuse festive, and accept gold"
ctl command "execute in minecraft:the_end run tp @s $((M_BIG_A + 2)).5 101 0.5" >/dev/null
ctl wait 4 >/dev/null
place_connector "$M_BIG_A" west createaddition:large_connector "$M_Y"
place_connector "$M_BIG_B" east createaddition:large_connector "$M_Y"
info "$M_BIG_A" "$M_Y" 0
[ "$capacity" = 5000 ] || fail "a large connector's window is $capacity, expected 5000"
command_reply "createaddition link $M_BIG_A $M_Y 0 $M_BIG_B $M_Y 0 festive" "$LINK_REPLY"
LAST_REPLY=$REPLY
echo "  link $M_BIG_A-$M_BIG_B (festive): $LAST_REPLY" >&2
case "$LAST_REPLY" in
    *"gold or electrum"*) echo "  festive refused between two large connectors, as copper is" >&2 ;;
    *) fail "festive was not refused between two large connectors: $LAST_REPLY" ;;
esac
if [ "${FESTIVE_SELFTEST:-0}" = 1 ]; then
    # The deliberately wrong expectation: the wire MUST have been accepted. It was refused, so this
    # leg fails — the counterfactual for the assertion above.
    fail "FESTIVE_SELFTEST: festive was refused between two large connectors ($LAST_REPLY), so the refusal assertion is alive"
fi
info "$M_BIG_A" "$M_Y" 0
[ "$links" = 0 ] || fail "the refused festive wire was recorded anyway"
command_reply "createaddition link $M_BIG_A $M_Y 0 $M_BIG_B $M_Y 0 gold" "$LINK_REPLY"
[ "$REPLY" = "Nodes linked." ] || fail "gold was refused between two large connectors: $REPLY"

echo "  -- the iron wire is content only: the item exists and no wire kind does"
ctl command "item replace entity @s hotbar.1 with createaddition:iron_wire 1" >/dev/null
ctl wait 4 >/dev/null
[ "$(item_held createaddition:iron_wire)" = 1 ] \
    || fail "the iron wire item did not register: $(ctl inventory | jq -c '{hotbar,main}')"
command_reply "createaddition link $M_BIG_A $M_Y 0 $M_BIG_B $M_Y 0 iron" "$WIRE_WORD_REFUSAL"
[ "$REPLY" = "The wire is copper, gold, electrum or festive." ] \
    || fail "an iron wire kind was accepted: $REPLY"
echo "  createaddition:iron_wire is held and /createaddition link refuses 'iron', as 1.7.2 has no IRON wire" >&2

echo "  -- the empty spool coils a festive wire back onto a festive spool"
ctl command "execute in minecraft:the_end run tp @s $((M_C1 + 2)).5 101 0.5" >/dev/null
ctl wait 4 >/dev/null
ctl command "gamemode survival" >/dev/null
ctl command "item replace entity @s hotbar.1 with createaddition:spool 1" >/dev/null
ctl select createaddition:spool >/dev/null
ctl wait 4 >/dev/null
M_SPOOLS_BEFORE=$(item_held createaddition:festive_spool)
ctl interact "$M_C1" "$M_Y" 0 east >/dev/null
ctl wait 6 >/dev/null
M_SPOOLS_AFTER=$(item_held createaddition:festive_spool)
[ "$((M_SPOOLS_AFTER - M_SPOOLS_BEFORE))" = 1 ] \
    || fail "the empty spool took the festive wire out and handed back $((M_SPOOLS_AFTER - M_SPOOLS_BEFORE)) festive spools, expected one"
info "$M_C1" "$M_Y" 0
[ "$links" = 0 ] || fail "the festive wire is still saved after the empty spool took it"
echo "  the empty spool removed the festive wire and gave back one festive spool ($M_SPOOLS_BEFORE -> $M_SPOOLS_AFTER)" >&2

echo "PASS: FE crossed the wire at 1,000/t and was conserved, an unlinked connector moved nothing, the 16-block limit held, copper was refused between two large connectors, an accumulator fed the block above it, a three-high tower formed one 6,000,000 FE pool, a broken tower re-formed at 4,000,000, separated accumulators stayed singles, a doctored saved tower was refused as its block's own single, every block drops itself, the redstone relay passed energy only while powered, the small light connector lit from its network's one FE a tick and went dark when unlinked, a comparator on the formed tower carried the jar's fraction (0, 8, 15) on its own output wire while the same rig on stone read 0, and the creative generator supplied a wire at exactly 1,000 FE a tick (20,000 FE in the window) while stone in its place moved nothing and a generator beside an accumulator filled it at 5,000 a tick, and the festive spool laid a wire both ends save as 'festive' which carried exactly 1,000 FE a tick while two large connectors refused it and the empty spool gave a festive spool back"
