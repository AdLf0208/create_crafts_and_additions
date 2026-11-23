#!/usr/bin/env bash
# Probe 3: replicate the lifecycle leg's N..N5 exactly, quit, relaunch, and dump what the second
# session sees (blocks, saved data, forceload, the mod's answers, and the console's own lines).
set -Eeuo pipefail
export LC_ALL=C
CTL=${CTL:-}
GAME=${GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
TMP=${E2E_TMP:-/tmp/opencode}
mkdir -p "$TMP"
SOCK=${SOCK:-$TMP/createaddition-probe3.sock}
SCRATCH=$(mktemp -d "$TMP/createaddition-probe3-home.XXXXXX")
export HORIZON_HOME=$SCRATCH/game HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data XDG_CONFIG_HOME=$SCRATCH/config HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"
ctl() { "$CTL" --socket "$SOCK" "$@"; }
cleanup() { local s=$?; ctl quit >/dev/null 2>&1 || true; rm -rf "$SCRATCH"; echo "EXIT:$s"; }
trap cleanup EXIT
launch() {
    rm -f "$SOCK"
    cd "$SCRATCH"
    "$CTL" --socket "$SOCK" launch --game "$GAME" --world "$1" --seed 20261105 \
        --creative --headless --wait --resources "$RES" >/dev/null
    ctl wait-ready 120
    ctl wait 40
}
signals() { ctl chat "/createaddition signals" >/dev/null; ctl wait 10 >/dev/null; ctl messages 10 | jq -r '.chat[]' | grep -m1 "Signals"; }
pad() {
    ctl command "execute in minecraft:the_end run tp @s $(( $1 + 2 )).5 104 0.5" >/dev/null
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        [ "$(ctl block "$1" 100 0 | jq -r '.loaded // false')" = true ] && break
    done
    ctl command "execute in minecraft:the_end run fill $(( $1 - 8 )) 100 -6 $(( $1 + 8 )) 100 6 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run fill $(( $1 - 8 )) 101 -6 $(( $1 + 8 )) 112 6 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
}
AX=300
BX=1100

echo "== session 1 =="
launch CreateAdditionProbe3
pad "$AX"
pad "$BX"
ctl command "execute in minecraft:the_end run forceload add $BX 0" >/dev/null
ctl wait 6 >/dev/null
ctl command "execute in minecraft:the_end run tp @s $((AX + 1)).5 101 3.5" >/dev/null
ctl wait 20 >/dev/null
ctl command "execute in minecraft:the_end run setblock $AX 101 0 createaddition:modular_accumulator" >/dev/null
ctl command "execute in minecraft:the_end run setblock $BX 101 0 createaddition:modular_accumulator" >/dev/null
ctl wait 10 >/dev/null
ctl chat "/createaddition energy $AX 101 0 1000000" >/dev/null
ctl chat "/createaddition energy $BX 101 0 1000000" >/dev/null
ctl wait 20 >/dev/null
echo -n "  A block: "; ctl block $AX 101 0
echo -n "  B data: "; ctl command "execute in minecraft:the_end run data get block $BX 101 0" | jq -c '.chat'
echo -n "  signals: "; signals
ctl command "save-all flush" >/dev/null
ctl wait 10 >/dev/null
echo -n "  saved: "; signals
ctl quit >/dev/null 2>&1 || true
sleep 3

echo "== session 2 =="
launch CreateAdditionProbe3
echo -n "  player: "; ctl state | jq -c '[.player.position,.dimension]'
echo -n "  block_entities: "; ctl state | jq -c '.block_entities'
ctl command "execute in minecraft:the_end run forceload add $BX 0" >/dev/null
ctl wait 10 >/dev/null
echo -n "  A data: "; ctl command "execute in minecraft:the_end run data get block $AX 101 0" | jq -c '.chat'
echo -n "  B data: "; ctl command "execute in minecraft:the_end run data get block $BX 101 0" | jq -c '.chat'
echo -n "  signals: "; signals
ctl wait 40 >/dev/null
echo -n "  signals after 40: "; signals
echo "== the console's own lines =="
ctl logs --lines 400 | grep -iE "createaddition|panic|stale|block entit" | tail -30 || true
echo "PROBE3: done"
