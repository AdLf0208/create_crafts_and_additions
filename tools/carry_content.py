#!/usr/bin/env python3
"""Carry Create Crafts & Additions' FE-half content from the ATM10 jar.

The jar is the evidence (`ports/_shared/licence-cache/createaddition-1.7.2.jar`,
MIT); this script copies the models, textures, item definitions, loot tables and
lang for the three blocks and seven items of the slice, writes the block and
item declarations in Horizon's 0.2 schema, and patches the two things the jar
cannot give Horizon:

* block models name `create:block/andesite_casing` as their particle texture;
  Create is not installed here, so it points at the block's own texture;
* the blockstates are rewritten for the properties this port declares
  (`facing` + `mode` for the connectors, none for the accumulator), because the
  jar's `variant` property (girder mounts) is not part of the slice.

Everything else is byte-for-byte the jar's. `--check` re-derives the tree in a
temporary folder and exits 1 when the tree on disk differs.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import shutil
import sys
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
JAR = ROOT.parent / "_shared/licence-cache/createaddition-1.7.2.jar"
NS = "createaddition"

# The slice's four blocks and seven items (FINDINGS.md, "Jar inventory").
BLOCKS = ["connector", "large_connector", "modular_accumulator", "redstone_relay"]
ITEMS = [
    "copper_wire",
    "gold_wire",
    "electrum_wire",
    "spool",
    "copper_spool",
    "gold_spool",
    "electrum_spool",
]

# Models the jar ships that this slice draws.
BLOCK_MODELS = [
    "block/connector/small/none",
    "block/connector/small/pull",
    "block/connector/small/push",
    "block/connector/small/item",
    "block/connector/small/block",
    "block/connector/large/none",
    "block/connector/large/pull",
    "block/connector/large/push",
    "block/connector/large/item",
    "block/modular_accumulator/block_single",
    "block/modular_accumulator/block_bottom",
    "block/modular_accumulator/block_middle",
    "block/modular_accumulator/block_top",
    "block/modular_accumulator/item",
    "block/redstone_relay/redstone_relay_off",
    "block/redstone_relay/redstone_relay_on",
    "block/redstone_relay/item",
]
ITEM_MODELS = [f"item/{item}" for item in ITEMS] + [
    "item/connector",
    "item/large_connector",
    "item/modular_accumulator",
    "item/redstone_relay",
]
# Textures no carried model names (the accumulator's gauge is drawn by its
# block entity in upstream; the connector's bulb is for the light variant).
EXTRA_TEXTURES = [
    "block/connector/bulb",
    "block/modular_accumulator/gauge",
]
# The relay's blockstate is the jar's own 16-variant table: it names only this
# mod's models and the `facing`/`powered`/`vertical` properties this port declares.
VERBATIM_BLOCKSTATES = ["modular_accumulator", "redstone_relay"]


def model_textures(jar: zipfile.ZipFile, models: list[str]) -> list[str]:
    """Every texture of our namespace a carried model names, in model order."""
    found: list[str] = []
    for model in models:
        member = f"{NS}/models/{model}.json"
        try:
            parsed = json.loads(jar.read(f"assets/{member}"))
        except KeyError:
            continue
        for value in (parsed.get("textures") or {}).values():
            if isinstance(value, str) and value.startswith(f"{NS}:"):
                path = value[len(f"{NS}:") :]
                if path not in found:
                    found.append(path)
    return found

# The connector model points one way and is rotated per facing, as the jar's
# own `connector.json` blockstate does.
FACING_ROTATION = {
    "down": {"y": 90},
    "up": {"x": 180, "y": 90},
    "north": {"x": 90, "y": 180},
    "south": {"x": 90},
    "west": {"x": 90, "y": 90},
    "east": {"x": 90, "y": 270},
}
FACING_VALUES = ["down", "up", "north", "south", "west", "east"]
MODES = ["none", "pull", "push"]

# A 4x4-pixel nub on the face the connector is mounted to.
FACING_SHAPE = {
    "down": [0.375, 0.0, 0.375, 0.625, 0.25, 0.625],
    "up": [0.375, 0.75, 0.375, 0.625, 1.0, 0.625],
    "north": [0.375, 0.375, 0.0, 0.625, 0.625, 0.25],
    "south": [0.375, 0.375, 0.75, 0.625, 0.625, 1.0],
    "west": [0.0, 0.375, 0.375, 0.25, 0.625, 0.625],
    "east": [0.75, 0.375, 0.375, 1.0, 0.625, 0.625],
}

LANG_KEYS = [
    "block.createaddition.connector",
    "block.createaddition.large_connector",
    "block.createaddition.modular_accumulator",
    "block.createaddition.redstone_relay",
    "item.createaddition.copper_wire",
    "item.createaddition.gold_wire",
    "item.createaddition.electrum_wire",
    "item.createaddition.spool",
    "item.createaddition.copper_spool",
    "item.createaddition.gold_spool",
    "item.createaddition.electrum_spool",
    "item.createaddition.spool.nbt",
    "createaddition.tooltip.connector.info",
    "createaddition.tooltip.accumulator.info",
    "createaddition.tooltip.relay.info",
    "createaddition.tooltip.energy.mode",
    "createaddition.tooltip.energy.pull",
    "createaddition.tooltip.energy.push",
    "createaddition.tooltip.energy.none",
    "createaddition.tooltip.energy.stored",
    "createaddition.tooltip.energy.capacity",
    "statusbar.createaddition.wire.linked",
    "statusbar.createaddition.wire.long",
    "statusbar.createaddition.wire.exists",
    "statusbar.createaddition.wire.count",
    "statusbar.createaddition.wire.removed",
    "statusbar.createaddition.wire.no_connection",
    "statusbar.createaddition.wire.invalid",
    "statusbar.createaddition.wire.requires_high_current",
]


def write(root: Path, relative: str, text: str) -> None:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def copy(root: Path, jar: zipfile.ZipFile, member: str) -> None:
    path = root / "assets" / member
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(jar.read(f"assets/{member}"))


def patched_model(jar: zipfile.ZipFile, member: str) -> str:
    """A block model with Create's particle texture replaced by our own."""
    model = json.loads(jar.read(f"assets/{member}"))
    textures = model.get("textures")
    if isinstance(textures, dict):
        for key, value in list(textures.items()):
            if isinstance(value, str) and value.startswith("create:"):
                textures[key] = f"{NS}:block/connector/connector"
    return json.dumps(model, indent=2) + "\n"


def loot_table_26_3(jar: zipfile.ZipFile, block: str) -> str:
    """One block loot table, rewritten from 1.21.1's shape into 26.3's.

    The jar is correct for 1.21.1: its pools carry `"conditions": [ {...} ]`
    whose object names its kind with `"condition"`, plus `"bonus_rolls": 0.0`
    and `"rolls": 1.0`. 26.3's own block tables (the prepared resources'
    `data/minecraft/loot_table/blocks/*.json`) have a singular
    `"condition": {"type": ...}`, no `bonus_rolls`, and `"rolls": 1`; carried
    verbatim, the 1.21.1 form makes the block drop nothing at all.
    """
    table = json.loads(jar.read(f"data/{NS}/loot_table/blocks/{block}.json"))
    for pool in table.get("pools", []):
        pool.pop("bonus_rolls", None)
        conditions = pool.pop("conditions", None)
        if conditions:
            condition = dict(conditions[0])
            if "condition" in condition and "type" not in condition:
                condition["type"] = condition.pop("condition")
            pool["condition"] = condition
        rolls = pool.get("rolls")
        if isinstance(rolls, float) and rolls.is_integer():
            pool["rolls"] = int(rolls)
    # 26.3's tables lay their keys out alphabetically; so does this.
    return json.dumps(table, indent=2, sort_keys=True) + "\n"


def blockstate_connector(size: str) -> str:
    parts = []
    for facing in FACING_VALUES:
        for mode in MODES:
            apply = {"model": f"{NS}:block/connector/{size}/{mode}"}
            apply.update(FACING_ROTATION[facing])
            parts.append({"when": {"facing": facing, "mode": mode}, "apply": apply})
    return json.dumps({"multipart": parts}, indent=2) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--out", type=Path, default=ROOT)
    args = parser.parse_args()

    if not JAR.is_file():
        print(f"missing jar: {JAR}", file=sys.stderr)
        return 1

    target = args.out
    if args.check:
        with tempfile.TemporaryDirectory() as scratch:
            generated = Path(scratch) / "package"
            build(generated)
            return compare(target, generated)
    build(target)
    print(f"carried {len(BLOCKS)} blocks, {len(ITEMS)} items into {target}")
    return 0


def compare(disk: Path, generated: Path) -> int:
    problems = []
    for path in sorted(generated.rglob("*")):
        if not path.is_file():
            continue
        relative = path.relative_to(generated)
        on_disk = disk / relative
        if not on_disk.is_file():
            problems.append(f"missing: {relative}")
        elif on_disk.read_bytes() != path.read_bytes():
            problems.append(f"differs: {relative}")
    for path in sorted(disk.rglob("*")):
        if path.is_file() and not (generated / path.relative_to(disk)).exists():
            relative = path.relative_to(disk)
            if relative.parts[0] in ("assets", "data"):
                problems.append(f"not carried: {relative}")
    if problems:
        print("\n".join(problems), file=sys.stderr)
        return 1
    print("assets/ and data/ match the jar carry")
    return 0


def build(root: Path) -> None:
    root.mkdir(parents=True, exist_ok=True)
    jar = zipfile.ZipFile(JAR)

    # The package's own `horizon-mod.toml` is not carried content: the SDK manifest is the
    # package's one identity (id, version, api, [native]) and is written by hand.

    # --- blocks ------------------------------------------------------------
    for block in ("connector", "large_connector"):
        entry = {
            "template": "minecraft:iron_block",
            "item": True,
            "properties": {
                "hardness": 1.5,
                "explosion_resistance": 6.0,
                "requires_tool": True,
                "occludes": False,
                "sounds": "minecraft:copper_block",
            },
            "state_properties": {
                "facing": {"type": "enum", "values": FACING_VALUES},
                "mode": {"type": "enum", "values": MODES},
            },
            "default_state": {"facing": "north", "mode": "none"},
            "shapes": {
                "properties": ["facing"],
                "values": {facing: [box] for facing, box in FACING_SHAPE.items()},
            },
        }
        write(root, f"data/{NS}/horizon/block/{block}.json", json.dumps(entry, indent=2) + "\n")

    # The accumulator declares the jar's own `top`/`bottom` multiblock bits: the
    # tower's highest block has `top`, its lowest `bottom`, and a single block both
    # (`ModularAccumulatorBlock.notifyMultiUpdated`).
    accumulator = {
        "template": "minecraft:iron_block",
        "item": True,
        "properties": {
            "hardness": 2.0,
            "explosion_resistance": 6.0,
            "requires_tool": True,
            "sounds": "minecraft:copper_block",
        },
        "state_properties": {
            "top": {"type": "boolean"},
            "bottom": {"type": "boolean"},
        },
        "default_state": {"top": "true", "bottom": "true"},
    }
    write(
        root,
        f"data/{NS}/horizon/block/modular_accumulator.json",
        json.dumps(accumulator, indent=2) + "\n",
    )

    # The relay: the jar's `horizontal_facing`, `vertical` and `powered` state bits, its
    # 2-pixel plate shape, and no energy buffer of its own (its 1,000 FE is the bridge's
    # in-flight buffer, written by the port's pass).
    relay = {
        "template": "minecraft:smooth_stone",
        "item": True,
        "properties": {
            "hardness": 1.5,
            "explosion_resistance": 6.0,
            "requires_tool": True,
            "occludes": False,
            "sounds": "minecraft:stone",
        },
        "state_properties": {
            "facing": {
                "type": "enum",
                "values": ["north", "south", "west", "east"],
            },
            "vertical": {"type": "boolean"},
            "powered": {"type": "boolean"},
        },
        "default_state": {"facing": "north", "vertical": "false", "powered": "false"},
        "shapes": {"default": [[0.0, 0.0, 0.0, 1.0, 0.125, 1.0]]},
        "collision_shapes": {"default": [[0.0, 0.0, 0.0, 1.0, 0.125, 1.0]]},
    }
    write(
        root,
        f"data/{NS}/horizon/block/redstone_relay.json",
        json.dumps(relay, indent=2) + "\n",
    )

    # --- items -------------------------------------------------------------
    # The jar's items carry no components; the templates only set the stack
    # size and the look, and the look is overridden by the carried model.
    for item in ITEMS:
        write(
            root,
            f"data/{NS}/horizon/item/{item}.json",
            json.dumps({"template": "minecraft:copper_ingot"}, indent=2) + "\n",
        )

    # --- blockstates -------------------------------------------------------
    write(root, f"assets/{NS}/blockstates/connector.json", blockstate_connector("small"))
    write(root, f"assets/{NS}/blockstates/large_connector.json", blockstate_connector("large"))
    # The accumulator's and the relay's blockstates are the jar's own: the accumulator's
    # multipart over `top`/`bottom`, the relay's 16-variant table over
    # `facing`/`powered`/`vertical`. Both name only this mod's models.
    for block in VERBATIM_BLOCKSTATES:
        write(
            root,
            f"assets/{NS}/blockstates/{block}.json",
            jar.read(f"assets/{NS}/blockstates/{block}.json").decode(),
        )

    # --- models and textures ----------------------------------------------
    for model in BLOCK_MODELS:
        write(root, f"assets/{NS}/models/{model}.json", patched_model(jar, f"{NS}/models/{model}.json"))
    for model in ITEM_MODELS:
        copy(root, jar, f"{NS}/models/{model}.json")
    textures = model_textures(jar, BLOCK_MODELS + ITEM_MODELS) + EXTRA_TEXTURES
    for texture in textures:
        member = f"{NS}/textures/{texture}.png"
        if f"assets/{member}" in jar.namelist():
            copy(root, jar, member)
        else:
            print(f"texture not in the jar: {member}", file=sys.stderr)

    # --- 26.3 item definitions --------------------------------------------
    for item in ITEMS + [
        "connector",
        "large_connector",
        "modular_accumulator",
        "redstone_relay",
    ]:
        write(
            root,
            f"assets/{NS}/items/{item}.json",
            json.dumps(
                {"model": {"type": "minecraft:model", "model": f"{NS}:item/{item}"}},
                indent=2,
            )
            + "\n",
        )

    # --- loot tables, in 26.3's own shape (the jar's are 1.21.1's) ---------
    for block in BLOCKS:
        write(
            root,
            f"data/{NS}/loot_table/blocks/{block}.json",
            loot_table_26_3(jar, block),
        )

    # --- the harvest tag ---------------------------------------------------
    # The jar lists all 17 of its blocks in `minecraft:mineable/pickaxe`; this package ships
    # three of them. Without the tag an iron pickaxe is not "correct for drops" on our blocks,
    # and `requires_tool: true` then makes a real break drop **nothing** (found in game:
    # `items_left_on_ground: 0`). Only the carried blocks are listed: a tag value naming an
    # unregistered id is dead data.
    jar_tag = json.loads(jar.read("data/minecraft/tags/block/mineable/pickaxe.json"))
    values = [name for name in jar_tag["values"] if name.split(":", 1)[1] in BLOCKS]
    write(
        root,
        "data/minecraft/tags/block/mineable/pickaxe.json",
        json.dumps({"values": values}, indent=2) + "\n",
    )

    # --- lang --------------------------------------------------------------
    jar_lang = json.loads(jar.read(f"assets/{NS}/lang/en_us.json"))
    carried = {key: jar_lang[key] for key in LANG_KEYS if key in jar_lang}
    missing = [key for key in LANG_KEYS if key not in jar_lang]
    if missing:
        print(f"lang keys absent from the jar: {missing}", file=sys.stderr)
    write(root, f"assets/{NS}/lang/en_us.json", json.dumps(carried, indent=2) + "\n")


if __name__ == "__main__":
    raise SystemExit(main())
