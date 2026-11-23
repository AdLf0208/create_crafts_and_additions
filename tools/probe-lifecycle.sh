#!/usr/bin/env bash
# Probe for the lifecycle leg: does `/forceload add` keep the far rig's chunk loaded while the
# player stands 800 blocks away, and does an accumulator placed there survive and tick?
set -Eeuo pipefail
export LC_ALL=C
CTL=${CTL:-}
GAME=${GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
TMP=${E2E_TMP:-/tmp/opencode}
mkdir -p "$TMP"
SOCK=${SOCK:-$TMP/createaddition-probe.sock}
SCRATCH=$(mktemp -d "$TMP/createaddition-probe-home.XXXXXX")
export HORIZON_HOME=$SCRATCH/game HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data XDG_CONFIG_HOME=$SCRATCH/config HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"
ctl() { "$CTL" --socket "$SOCK" "$@"; }
cleanup() { local s=$?; ctl quit >/dev/null 2>&1 || true; rm -rf "$SCRATCH"; echo "EXIT:$s"; }
trap cleanup EXIT
cd "$SCRATCH"
"$CTL" --socket "$SOCK" launch --game "$GAME" --world CreateAdditionProbe --seed 20261105 \
    --creative --headless --wait --resources "$RES" >/dev/null
ctl wait-ready 120
ctl wait 40
ctl command "execute in minecraft:the_end run tp @s 302.5 104 0.5" >/dev/null
ctl wait 10 >/dev/null
echo "== forceload the far chunk =="
ctl command "execute in minecraft:the_end run forceload add 1100 0" | tee /dev/stderr
ctl wait 6 >/dev/null
echo "== far chunk loaded? =="
ctl block 1100 100 0
echo "== setblock an accumulator 800 blocks away =="
ctl command "execute in minecraft:the_end run setblock 1100 101 0 createaddition:modular_accumulator" | tee /dev/stderr
ctl wait 8 >/dev/null
ctl block 1100 101 0
echo "== forceload query =="
ctl command "execute in minecraft:the_end run forceload query 1100 0" | tee /dev/stderr
echo "== signals =="
ctl chat "/createaddition signals" >/dev/null
ctl wait 10 >/dev/null
ctl messages 20 | jq -r '.chat[]' | head -5
ctl command "execute in minecraft:the_end run forceload remove 1100 0" >/dev/null
echo "PROBE: done"
