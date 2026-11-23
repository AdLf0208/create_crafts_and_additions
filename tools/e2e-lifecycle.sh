#!/usr/bin/env bash
# The lifecycle leg: an accumulator's comparator answer must not outlive its block, its chunk or
# its level.
#
# The reviewer's finding (0.2.6–0.2.10): the answers the level pass records for `analog_output`
# were kept "until the process ends", so a broken accumulator, an unloaded chunk and a closed level
# each left a position-keyed entry behind. 0.2.11 forgets the answer on the three events:
#
#   block removed or replaced   BlockPart::on_block_state_change (this block's own states)
#   chunk unloaded              Hooks::chunk_unload   (dimension + chunk [x,z] from the event)
#   level closed                Hooks::level_unload   (the dimension's whole map, the tick queue)
#
# A spool's first click (the player-keyed pending selection) is dropped by `level_unload` and by
# `player_logged_out` as well.
#
# The assertion is `/createaddition signals`'s own count — `answers=<n> dimensions=<dim>:<n>` — plus
# the world's own reading of the **survivor's** comparator wire, which must still carry its answer.
# The count is the only observable for an entry whose block is gone: the block is air, so a
# comparator beside it reads 0 whether the entry is stale or not, and the evicted chunk is not
# loaded, so nothing there can be read at all.
#
#   two accumulators, 800 blocks apart   answers=2
#   one broken with /setblock air        answers=1, the survivor's wire still 8, the broken one's 0
#   the far chunk unloaded (teleport)    answers=1, the survivor's wire still 8
#   back, the pass re-records            answers=2
#   /save-all                            answers=2 (a save is not an unload: `world_saved` is not
#                                        the clearing event, and clearing here would blank a live
#                                        comparator answer until the next pass)
#   relaunch on the same world           answers=1 at the survivor's chunk: a fresh session carries
#                                        no answer it did not compute itself, and rebuilds it from
#                                        the saved block
#
# `LIFECYCLE_SELFTEST=1` is the counterfactual: the chunk unload must have left the answer in place,
# and it did not, so the leg exits 1 — which is what proves the default leg's "the answer went"
# assertion is a real assertion and not something any package would pass.
#
# `CONTROL_GAME=` runs the same setup on the Nightly without the package: `createaddition:
# modular_accumulator` is unknown there, the rig cannot be built, and the leg fails.
#
# Traps this leg pays for (the same ones 0.2.10's drop leg paid for): a `fill` in an unloaded chunk
# reports success and changes nothing, so the pad's chunk is loaded and the floor read back before
# the rig is built; every reply is tagged; `/tmp` is a small tmpfs, so the scratch root honours
# `E2E_TMP`; and the comparator reads the block on its `facing` side and drives the wire behind it
# (the geometry 0.2.6's probe measured).
set -Eeuo pipefail
export LC_ALL=C
[ -z "${CA_DEBUG:-}" ] || set -x

SELFTEST=${LIFECYCLE_SELFTEST:-0}
CONTROL=${CONTROL_GAME:+1}
CONTROL=${CONTROL:-0}
CTL=${CTL:-}
GAME=${GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
# `/tmp` is a small tmpfs on this box: honour the runner's scratch root when it sets one.
TMP=${E2E_TMP:-/tmp/opencode}
mkdir -p "$TMP"
SOCK=${SOCK:-$TMP/createaddition-lifecycle.sock}
KEEP=${KEEP:-0}
WORLD=CreateAdditionLifecycle
CONTROL_WORLD=CreateAdditionLifecycleControl

Y=101
Z=0
# Two rigs: 800 blocks apart, so one chunk can unload while the other stays loaded even at the
# largest view distance the launcher can set (32 chunks = 512 blocks). The far rig's chunk is
# **forceloaded**, because a chunk with no player near it would unload on its own and its block
# entity would never tick — the count would never reach 2 and the "one evicted, one kept" step
# would prove nothing. The near rig has no ticket, so it is the one the teleport evicts.
AX=300
BX=1100
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

SCRATCH=$(mktemp -d "$TMP/createaddition-lifecycle-home.XXXXXX")
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
block_state() { ctl block "$1" "$2" "$3" | jq -r '.state // empty'; }
wire_power() { block_state "$1" "$2" "$3" | sed -n 's/.*power=\([0-9][0-9]*\).*/\1/p'; }

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
# A pattern must not end in a space: `command_reply` appends " <anything> [#tag]" to it, so a
# trailing space would demand two and never match (0.2.10's harness lesson).
SIGNALS_REPLY='^Signals: answers=[0-9]+ dimensions=[^ ]* pending=[0-9]+'
ENERGY_REPLY='^Energy set to [0-9]+ FE\.'

# `/createaddition signals` into ANSWERS, DIMENSIONS and PENDING.
signals() {
    command_reply "createaddition signals" "$SIGNALS_REPLY"
    ANSWERS=$(sed -n 's/^Signals: answers=\([0-9][0-9]*\) .*/\1/p' <<<"$REPLY")
    DIMENSIONS=$(sed -n 's/^Signals: answers=[0-9][0-9]* dimensions=\([^ ]*\) .*/\1/p' <<<"$REPLY")
    PENDING=$(sed -n 's/.* pending=\([0-9][0-9]*\)$/\1/p' <<<"$REPLY")
    [ -n "$ANSWERS" ] || fail "could not parse the signals reply: $REPLY"
    [ -n "$DIMENSIONS" ] || fail "could not parse the signals reply: $REPLY"
    [ -n "$PENDING" ] || fail "could not parse the signals reply: $REPLY"
    [ "$PENDING" = 0 ] || fail "a spool selection is in progress with no spool in hand: pending=$PENDING"
}

expect_wire() { # expect_wire X Y Z LEVEL WHAT
    local power
    power=$(wire_power "$1" "$2" "$3")
    [ -n "$power" ] \
        || fail "no redstone wire at $1,$2,$3 for $5 (state '$(block_state "$1" "$2" "$3")')"
    [ "$power" = "$4" ] || fail "$5: the wire at $1,$2,$3 reads power=$power, expected $4"
    echo "  $5: redstone_wire at $1,$2,$3 power=$power" >&2
}

# A stone pad under one rig, with air above it, and its chunk loaded first (a `fill` in an
# unloaded chunk changes nothing).
pad() { # pad X
    local x=$1 loaded=false
    ctl command "execute in minecraft:the_end run tp @s $((x + 2)).5 104 0.5" >/dev/null
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(ctl block "$x" 100 0 | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the chunk at $x,100,0 never loaded"
    ctl command "execute in minecraft:the_end run fill $((x - 8)) 100 -6 $((x + 8)) 100 6 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run fill $((x - 8)) 101 -6 $((x + 8)) 112 6 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run kill @e[type=!player]" >/dev/null
    ctl wait 6 >/dev/null
    [ "$(block_name "$x" 100 0)" = "minecraft:stone" ] || fail "the pad floor at $x is not stone"
    [ "$(block_name "$x" $Y $Z)" = "minecraft:air" ] || fail "the air above the pad at $x is not air"
}

# One accumulator with the comparator rig 0.2.6 measured: the comparator stands on the block's +z
# side facing north (so it reads the accumulator) and drives the wire behind it at z=2.
#
# The placement and its read-back are separate on purpose: `ctl block` answers through the
# **client**, whose copy of a chunk 800 blocks away is not loaded (`loaded:false`), while the
# server's copy is (the far rig's chunk is force-loaded and its block entity ticks — the probe
# `probe-lifecycle.log` measured exactly that). So a rig is placed from anywhere and read back only
# where the player stands.
rig_place() { # rig_place X
    local x=$1
    ctl command "execute in minecraft:the_end run setblock $x $Y $Z createaddition:modular_accumulator" >/dev/null
    ctl wait 5 >/dev/null
    ctl command "execute in minecraft:the_end run setblock $x $Y 1 minecraft:comparator[facing=north]" >/dev/null
    ctl command "execute in minecraft:the_end run setblock $x $Y 2 minecraft:redstone_wire" >/dev/null
    ctl wait 8 >/dev/null
}

rig_verify() { # rig_verify X  (only while the player stands beside it)
    local x=$1
    [ "$(block_name "$x" $Y $Z)" = "createaddition:modular_accumulator" ] \
        || fail "the accumulator did not place at $x,$Y,$Z (got '$(block_name "$x" $Y $Z)')"
    [ "$(block_name "$x" $Y 1)" = "minecraft:comparator" ] || fail "the comparator did not place at $x,$Y,1"
    [ "$(block_name "$x" $Y 2)" = "minecraft:redstone_wire" ] || fail "the wire did not place at $x,$Y,2"
}

# Wait, bounded, until the mod holds `want` answers. The count *is* the eviction assertion: an
# entry can only leave a chunk's column when the server really unloaded that chunk.
await_answers() { # await_answers WANT WHAT
    local want=$1 what=$2
    for _ in $(seq 1 40); do
        signals
        if [ "$ANSWERS" = "$want" ]; then
            return 0
        fi
        ctl wait 5 >/dev/null
    done
    echo "  last readout: answers=$ANSWERS dimensions=$DIMENSIONS" >&2
    return 1
}

# Half of a single block's 2,000,000 FE capacity is the jar's midpoint: 8, not 7 and not 15.
seed() { # seed X AMOUNT
    command_reply "createaddition energy $1 $Y $Z $2" "$ENERGY_REPLY"
    ctl wait 8 >/dev/null
}

# Wait, bounded, until the accumulator at `x` answers the jar's 8 again (its chunk reloaded and the
# pass recorded it).
await_loaded_answer() { # await_loaded_answer X
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(wire_power "$1" $Y 2)" = 8 ]; then
            return 0
        fi
    done
    return 1
}

stand() { # stand X
    # One block east and one south of the rig (the wire is at z=2), so the player stands on the pad
    # beside it and never inside a block.
    ctl command "execute in minecraft:the_end run tp @s $((1 + $1)).5 $Y 3.5" >/dev/null
    local settled=false
    for _ in $(seq 1 40); do
        ctl wait 5 >/dev/null
        if [ "$(ctl state | jq -r '.player.on_ground // false')" = true ]; then
            settled=true
            break
        fi
    done
    [ "$settled" = true ] || fail "the player never stood on the pad at $1"
    ctl wait 4 >/dev/null
}

control_leg() {
    echo "== control: the same rig on a Nightly without the package =="
    launch "$CONTROL_GAME" "$CONTROL_WORLD"
    pad "$AX"
    ctl command "execute in minecraft:the_end run setblock $AX $Y $Z createaddition:modular_accumulator" >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    got=$(block_name "$AX" $Y $Z)
    [ "$got" != "createaddition:modular_accumulator" ] \
        || fail "the control game placed an accumulator, so the block id was known"
    fail "the control game does not know createaddition:modular_accumulator (the setblock left '$got'), so no answer can be recorded or forgotten: the counts this leg reads come from this package"
}

if [ "$CONTROL" = 1 ]; then
    control_leg
fi

echo "== launch (The End, launcher-built game) =="
launch "$GAME" "$WORLD"

pad "$AX"
pad "$BX"
# The far rig ticks from its own ticket, so both answers exist at once (the near one loaded by the
# player, the far one force-loaded) and only the near one can be evicted by a teleport.
ctl command "execute in minecraft:the_end run forceload add $BX $Z" >/dev/null
ctl wait 6 >/dev/null
stand "$AX"

echo "== N: an accumulator's comparator answer is held while it is there =="
signals
echo "  before anything is placed: $REPLY" >&2
[ "$ANSWERS" = 0 ] || fail "the mod already holds $ANSWERS answer(s) before a block is placed: $REPLY"
rig_place "$AX"
rig_verify "$AX"
rig_place "$BX"
seed "$AX" 1000000
seed "$BX" 1000000
expect_wire "$AX" $Y 2 8 "the first accumulator's own comparator"
signals
echo "  two accumulators: $REPLY" >&2
[ "$ANSWERS" = 2 ] || fail "two placed accumulators must be two answers, not $ANSWERS: $REPLY"

echo "== N1: a removed accumulator's answer is forgotten, its neighbour's is not =="
ctl command "execute in minecraft:the_end run setblock $AX $Y $Z minecraft:air" >/dev/null
ctl wait 8 >/dev/null
[ "$(block_name "$AX" $Y $Z)" = "minecraft:air" ] || fail "the first accumulator is still at $AX,$Y,$Z"
signals
echo "  after the removal: $REPLY" >&2
[ "$ANSWERS" = 1 ] || fail "the removed accumulator's answer outlived it (answers=$ANSWERS): $REPLY"
expect_wire "$AX" $Y 2 0 "the removed accumulator's comparator (it now reads air)"

echo "== N2: a block placed where one was broken answers from a pass, not from the old entry =="
rig_place "$AX"
rig_verify "$AX"
seed "$AX" 1000000
expect_wire "$AX" $Y 2 8 "the re-placed accumulator"
signals
echo "  re-placed: $REPLY" >&2
[ "$ANSWERS" = 2 ] || fail "the pass did not record the re-placed accumulator (answers=$ANSWERS): $REPLY"

echo "== N3: a chunk unload forgets that chunk's answers and keeps the loaded one's =="
stand "$BX"
rig_verify "$BX"
await_answers 1 "the unloaded chunk" \
    || fail "the first rig's answer never left with its chunk (the player is 800 blocks away, so it unloaded)"
signals
UNLOADED_ANSWERS=$ANSWERS
echo "  after the unload: $REPLY" >&2
if [ "$SELFTEST" = 1 ]; then
    # The deliberately wrong expectation: the unloaded chunk's answer must still be there. It is
    # not, so this leg fails — the counterfactual for the assertion below.
    [ "$UNLOADED_ANSWERS" = 2 ] \
        || fail "LIFECYCLE_SELFTEST: the chunk unload forgot the answer (answers=$UNLOADED_ANSWERS), so the eviction assertion is alive"
fi
[ "$UNLOADED_ANSWERS" = 1 ] \
    || fail "the unloaded chunk's answer outlived its chunk (answers=$UNLOADED_ANSWERS): $REPLY"
expect_wire "$BX" $Y 2 8 "the accumulator whose chunk is still loaded"

echo "== N4: back on the pad, the pass records the answer again =="
stand "$AX"
await_loaded_answer "$AX" || fail "the first accumulator never answered 8 again after its chunk reloaded"
signals
echo "  back: $REPLY" >&2
[ "$ANSWERS" = 2 ] || fail "the reloaded accumulator was not recorded again (answers=$ANSWERS): $REPLY"

echo "== N5: a save is not an unload — /save-all keeps the live answers =="
ctl command "save-all flush" >/dev/null
ctl wait 10 >/dev/null
signals
echo "  after save-all: $REPLY" >&2
[ "$ANSWERS" = 2 ] || fail "a world save dropped the answers (answers=$ANSWERS): $REPLY"
expect_wire "$AX" $Y 2 8 "the comparator after a save"

echo "== N6: a relaunch carries no answer the new session did not compute =="
ctl quit >/dev/null 2>&1 || true
sleep 3
launch "$GAME" "$WORLD"
# No `pad` here: the pads and the rigs are part of the saved world, and a `pad`'s air fill would
# erase the rig it is meant to read (the first attempt failed exactly that way — `pad "$BX"`
# wiped the far accumulator, so the count read 0). `rig_verify` is the assertion that the world
# came back with its blocks, and the answer is then the new session's own.
stand "$BX"
rig_verify "$BX"
# The player's saved position is the first pad; leaving it unloads that chunk (and forgets its
# answer) before the count is read, so the number is what this session computed and nothing else.
await_answers 1 "the relaunched session" \
    || fail "the relaunched session holds answers for a chunk it is not loading, so state crossed the session"
expect_wire "$BX" $Y 2 8 "the survivor after the relaunch"
signals
echo "  after the relaunch, at the survivor: $REPLY" >&2
[ "$ANSWERS" = 1 ] \
    || fail "a fresh session holds $ANSWERS answer(s) with only one accumulator's chunk loaded, so state crossed the session: $REPLY"
stand "$AX"
await_loaded_answer "$AX" || fail "the other accumulator never answered again in the new session"
signals
echo "  both chunks loaded in the new session: $REPLY" >&2
[ "$ANSWERS" = 2 ] || fail "the new session recorded $ANSWERS answers for two loaded accumulators: $REPLY"

echo "PASS: an accumulator's comparator answer is forgotten when its block is removed, its chunk unloads and (in the rules) its level closes — the survivor's answer and its comparator wire survive each of them, a save does not drop them, and a relaunch carries none"
