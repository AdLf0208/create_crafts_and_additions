#!/usr/bin/env bash
# Probe 2: what survives a relaunch of the same world? Is the accumulator still there, is its block
# entity ticking, and does a freshly placed one record an answer?
set -Eeuo pipefail
export LC_ALL=C
CTL=${CTL:-}
GAME=${GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
TMP=${E2E_TMP:-/tmp/opencode}
mkdir -p "$TMP"
SOCK=${SOCK:-$TMP/createaddition-probe2.sock}
SCRATCH=$(mktemp -d "$TMP/createaddition-probe2-home.XXXXXX")
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

echo "== session 1: place an accumulator at 300,101,0 and seed it =="
launch CreateAdditionProbe2
ctl command "execute in minecraft:the_end run tp @s 301.5 101 3.5" >/dev/null
ctl wait 20 >/dev/null
ctl command "execute in minecraft:the_end run setblock 300 100 0 minecraft:stone" >/dev/null
ctl command "execute in minecraft:the_end run setblock 300 101 0 createaddition:modular_accumulator" >/dev/null
ctl wait 8 >/dev/null
ctl command "execute in minecraft:the_end run forceload add 300 0" >/dev/null
ctl chat "/createaddition energy 300 101 0 1000000" >/dev/null
ctl wait 20 >/dev/null
echo -n "  block: "; ctl block 300 101 0
echo -n "  signals: "; signals
ctl command "save-all flush" >/dev/null
ctl wait 10 >/dev/null
echo -n "  saved, signals: "; signals
ctl quit >/dev/null 2>&1 || true
sleep 2

echo "== session 2: relaunch the same world =="
launch CreateAdditionProbe2
echo -n "  player: "; ctl state | jq -c '.player.position,.dimension'
ctl command "execute in minecraft:the_end run tp @s 301.5 101 3.5" >/dev/null
ctl wait 20 >/dev/null
echo -n "  block at 300,101,0: "; ctl block 300 101 0
echo -n "  data get block: "; ctl command "execute in minecraft:the_end run data get block 300 101 0" | jq -c '.chat'
echo -n "  signals (right after the relaunch): "; signals
ctl wait 40 >/dev/null
echo -n "  signals (after 40 ticks): "; signals
echo "== a freshly placed accumulator in the same session =="
ctl command "execute in minecraft:the_end run setblock 310 100 0 minecraft:stone" >/dev/null
ctl command "execute in minecraft:the_end run setblock 310 101 0 createaddition:modular_accumulator" >/dev/null
ctl wait 20 >/dev/null
ctl chat "/createaddition energy 310 101 0 1000000" >/dev/null
ctl wait 20 >/dev/null
echo -n "  data get block 310: "; ctl command "execute in minecraft:the_end run data get block 310 101 0" | jq -c '.chat'
echo -n "  signals: "; signals
echo "PROBE2: done"
