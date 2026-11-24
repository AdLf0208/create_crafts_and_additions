#!/usr/bin/env bash
# Measure the accumulator gauge before drawing it: the model's UV/geometry and the jar's needle
# maths, both read from the audited artefact (never guessed). Evidence:
# ports/_shared/evidence/createaddition/probe-gauge.log
set -Eeuo pipefail
export LC_ALL=C
PORT=$(cd "$(dirname "$0")/.." && pwd)
JAR=${JAR:-$PORT/../_shared/licence-cache/createaddition-1.7.2.jar}
TMP=$(mktemp -d /tmp/opencode/ca-gauge-probe.XXXXXX)
trap 'rm -rf "$TMP"' EXIT

echo "== artefact =="
sha256sum "$JAR"
echo

echo "== guage.json (the gauge back, UV + block-local geometry) =="
unzip -p "$JAR" assets/createaddition/models/block/modular_accumulator/guage.json \
  | python3 -c 'import json,sys; d=json.load(sys.stdin); [print(e["name"], "from",e["from"],"to",e["to"],"uv",e["faces"]["east"]["uv"]) for e in d["elements"]]'
echo

echo "== dial.json (the needle: pivot + UV) =="
unzip -p "$JAR" assets/createaddition/models/block/modular_accumulator/dial.json \
  | python3 -c 'import json,sys; d=json.load(sys.stdin); e=d["elements"][0]; print("needle",e["name"],"from",e["from"],"to",e["to"],"rotation",e["rotation"],"uv",e["faces"]["east"]["uv"])'
echo

echo "== block model / blockstate: the block faces the gauge is NOT on =="
echo "block_single.json elements: $(unzip -p "$JAR" assets/createaddition/models/block/modular_accumulator/block_single.json | python3 -c 'import json,sys; print(len(json.load(sys.stdin)["elements"]))') (none named gauge/dial)"
echo "blockstate models: $(unzip -p "$JAR" assets/createaddition/blockstates/modular_accumulator.json | python3 -c 'import json,sys; print([w["apply"]["model"] for w in json.load(sys.stdin)["multipart"]])')"
echo

echo "== ModularAccumulatorRenderer.renderDial (javap -c, the jar's own class) =="
unzip -q -o "$JAR" 'com/mrh0/createaddition/blocks/modular_accumulator/ModularAccumulatorRenderer.class' -d "$TMP"
javap -c -p "$TMP/com/mrh0/createaddition/blocks/modular_accumulator/ModularAccumulatorRenderer.class" \
  | grep -E 'horizontalDirections|ACCUMULATOR_GUAGE|ACCUMULATOR_DIAL|toYRot|rotateXDegrees|fconst|ldc.*float|getValue|translate|uncenter' \
  | sed -n '1,40p'
echo
echo "needle angle: rotateXDegrees(-180.0f * gauge.getValue(partialTicks))  =>  -180 * fill degrees"
echo "pivot: (0.375, 0.5) in (y,z) of the model (dial.json origin [16,6,8]); dial reaches z 8..11.25"
echo

echo "== gauge screen box is the GAUGE's, not the block's =="
python3 - <<'PY'
# The gauge back on the east face (block-local, in blocks, from the lower north-west corner).
back = dict(x=(14/16, 15.95/16), y=(3.05/16, 10.95/16), z=(3.05/16, 12.95/16))
pivot = (1.0, 6/16, 8/16)             # dial.json rotation origin, model coords
needle = dict(y=(5.5/16, 6.5/16), z=(8/16, 11.25/16), half_w=0.5/16)
# After the renderer's translate(width/2,height-0.5,width/2) / uncenter / translate(width/2-0.375,0,0)
face = dict(x=tuple(v + 0.125 for v in back["x"]), y=back["y"], z=back["z"])
print("gauge back, east face, world-relative to the block:", face)
print("dial pivot, east face, world-relative to the block:", (pivot[0]+0.125, pivot[1], pivot[2]))
sweep = (pivot[1] - (needle["z"][1]-needle["z"][0]), pivot[1] + (needle["z"][1]-needle["z"][0]))
print("needle sweep y-range:", tuple(round(v,4) for v in sweep), "z-range:",
      (round(pivot[2]-(needle["z"][1]-needle["z"][0]),4), round(pivot[2]+(needle["z"][1]-needle["z"][0]),4)))
print("=> the screenshot box must cover only this ~0.5x0.6 block face patch, not the whole accumulator")
PY
