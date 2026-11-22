#!/usr/bin/env bash
# A probe, not evidence: shows whether the relay's `powered` bit follows a redstone block
# that is placed, removed and placed again, and how far the rig's charge moves in 10 ticks.
set -Eeuo pipefail
export LC_ALL=C

CLONE=${CLONE:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CLONE/ports/createaddition
CTL=${CTL:?CTL is required}
GAME=${GAME:?GAME is required}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
TMP=${E2E_TMP:-/tmp/opencode}
mkdir -p "$TMP"
SOCK=${SOCK:-$TMP/createaddition-probe.sock}

SCRATCH=$(mktemp -d "$TMP/createaddition-probe-home.XXXXXX")
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"
ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
trap 'status=$?; ctl quit >/dev/null 2>&1 || true; sleep 1; rm -rf "$SCRATCH" 2>/dev/null || true; echo "EXIT:$status"' EXIT

X0=260
AX=$((X0 - 2))
R=$X0
CX1=$((X0 - 1))
CX2=$((X0 + 2))
B=$((X0 + 3))

cd "$SCRATCH"
"$CTL" --socket "$SOCK" launch --game "$GAME" --world RelayProbe --seed 20261105 \
    --creative --headless --wait --resources "$RES" >/dev/null
ctl wait-ready 120
ctl wait 40
ctl command "execute in minecraft:the_end run tp @s $X0.5 104 0.5" >/dev/null
for _ in $(seq 1 120); do
    ctl wait 5 >/dev/null
    [ "$(ctl block "$X0" 100 0 | jq -r '.loaded // false')" = true ] && break
done
ctl command "execute in minecraft:the_end run fill $((AX - 1)) 100 -3 $((B + 1)) 100 3 minecraft:stone" >/dev/null
ctl command "execute in minecraft:the_end run fill $((AX - 1)) 101 -3 $((B + 1)) 104 3 minecraft:air" >/dev/null
ctl wait 6 >/dev/null
ctl command "execute in minecraft:the_end run tp @s $X0.5 101 0.5" >/dev/null
ctl wait 10
ctl command "execute in minecraft:the_end run setblock $AX 100 0 createaddition:modular_accumulator" >/dev/null
ctl command "execute in minecraft:the_end run setblock $B 100 0 createaddition:modular_accumulator" >/dev/null
ctl command "execute in minecraft:the_end run setblock $CX1 100 0 createaddition:connector[facing=west,mode=none]" >/dev/null
ctl command "execute in minecraft:the_end run setblock $CX2 100 0 createaddition:connector[facing=east,mode=none]" >/dev/null
ctl command "execute in minecraft:the_end run setblock $R 100 0 createaddition:redstone_relay[facing=east,vertical=false,powered=false]" >/dev/null
ctl wait 8
say "createaddition mode $CX2 100 0 push"
say "createaddition link $CX1 100 0 $R 100 0 copper"
ctl wait 4
say "createaddition link $R 100 0 $CX2 100 0 copper"
ctl wait 4
say "createaddition energy $AX 100 0 100000"
ctl wait 4
show() {
    echo "--- $1"
    ctl block "$R" 100 0 | jq -r '.state // "?"'
    ctl messages 40 | jq -r '.chat[]' | grep -E 'at (2[0-9]{2}),100,0:' | tail -5 || true
    echo "redstone at z=1: $(ctl block "$R" 100 1 | jq -r '.name // "?"')"
}
TAG=0
read_info() { # read_info X Y
    TAG=$((TAG + 1))
    local tag="p$TAG"
    say "createaddition info $1 $2 0 #$tag"
    for _ in $(seq 1 30); do
        ctl wait 3 >/dev/null
        local line
        line=$(ctl messages 60 | jq -r '.chat[]' | grep -F "[#$tag]" | tail -1 || true)
        if [ -n "$line" ]; then echo "  $line"; return 0; fi
    done
    echo "  (no reply to info $1 $2 #$tag)"
}
read_all() { # read_all LABEL
    echo "--- $1  (relay $(ctl block "$R" 100 0 | jq -r '.state'))"
    read_info "$AX" 100
    read_info "$B" 100
    read_info "$CX1" 100
    read_info "$R" 100
    ctl command "execute in minecraft:the_end run data get block $R 100 0" \
        | jq -r '.chat[0] // .chat' | tail -1
}
show "wired, unpowered"
ctl wait 10
read_all "unpowered"
ctl command "execute in minecraft:the_end run setblock $R 100 1 minecraft:redstone_block" >/dev/null
ctl wait 6
read_all "powered +6"
ctl wait 20
read_all "powered +26"
# The e2e's measurement sequence: break the power, park both connectors, read, restore.
ctl command "execute in minecraft:the_end run setblock $R 100 1 minecraft:air" >/dev/null
ctl wait 4
say "createaddition mode $CX1 100 0 none"
say "createaddition mode $CX2 100 0 none"
ctl wait 4
read_all "measure: power off, modes none"
say "createaddition mode $CX1 100 0 pull"
say "createaddition mode $CX2 100 0 push"
ctl command "execute in minecraft:the_end run setblock $R 100 1 minecraft:redstone_block" >/dev/null
ctl wait 4
read_all "measure: power restored"
ctl wait 20
read_all "measure: +20"
echo "PASS: probe finished"
