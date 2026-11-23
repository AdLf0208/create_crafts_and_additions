#!/usr/bin/env python3
"""Turn a `/createaddition info` line into shell assignments.

    Connector at 11,100,0: mode pull, buffer 0/1000 FE, links [(15,100,0)/0/gold], pool 0/0, demand 0, t=1234
    Modular Accumulator at 10,100,0: 95000/2000000 FE (t=1234)
    Redstone Relay at 234,100,0: powered false, relay 0/1000 FE, input [], output [], pool 0/0, demand 0, t=1234

Prints `key=value` lines for `eval`; `ok=1` when the line parsed. Values are
plain integers or bare words, so the output is safe to `eval` in bash.
"""
from __future__ import annotations

import re
import sys

# The lamp: `Small Light Connector at 254,100,0: mode none, powered true, lamp timer
# 25/25, draw 1 FE/t, buffer 0/1000 FE, links [...], pool 26/0, demand 1, t=1234`.
LIGHT = re.compile(
    r"^Small Light Connector at (-?\d+),(-?\d+),(-?\d+): mode (\w+), powered (true|false), "
    r"lamp timer (-?\d+)/(\d+), draw (\d+) FE/t, buffer (\d+)/(\d+) FE, links \[(.*?)\], "
    r"pool (\d+)/(\d+), demand (\d+), t=(\d+)$"
)
CONNECTOR = re.compile(
    r"^(Connector|Large Connector) at (-?\d+),(-?\d+),(-?\d+): mode (\w+), "
    r"buffer (\d+)/(\d+) FE, links \[(.*?)\], pool (\d+)/(\d+), demand (\d+), t=(\d+)$"
)
# The relay: `Redstone Relay at 234,100,0: powered false, relay 0/1000 FE, input
# [(233,100,0)/0/copper], output [(236,100,0)/4/copper], t=1234`.
RELAY = re.compile(
    r"^Redstone Relay at (-?\d+),(-?\d+),(-?\d+): powered (true|false), "
    r"relay (\d+)/(\d+) FE, input \[(.*?)\], output \[(.*?)\], "
    r"pool (\d+)/(\d+), demand (\d+), t=(\d+)$"
)
ACCUMULATOR = re.compile(
    r"^Modular Accumulator at (-?\d+),(-?\d+),(-?\d+): (\d+)/(\d+) FE \(t=(\d+)\)$"
)
# The creative generator: `Creative Generator at 180,100,0: infinite FE (t=1234)`.
CREATIVE = re.compile(
    r"^Creative Generator at (-?\d+),(-?\d+),(-?\d+): infinite FE \(t=(\d+)\)$"
)
# A formed tower: `... 5000000/6000000 FE, tower 1x3 at 220,100,0 (t=1234)`.
TOWER = re.compile(
    r"^Modular Accumulator at (-?\d+),(-?\d+),(-?\d+): (\d+)/(\d+) FE, "
    r"tower (\d+)x(\d+) at (-?\d+),(-?\d+),(-?\d+) \(t=(\d+)\)$"
)


TAG = re.compile(r" \[#\w+\]$")


def main() -> int:
    # A tagged reply ends with ` [#tag]`; the tag is the harness's, not the
    # block's.
    line = TAG.sub("", sys.argv[1].strip())
    match = RELAY.match(line)
    if match:
        x, y, z, powered, buffer, capacity, input_, output, ready, next_, demand, ticks = (
            match.groups()
        )
        values = {
            "kind": "relay",
            "large": 0,
            "x": x,
            "y": y,
            "z": z,
            "mode": "none",
            "powered": 1 if powered == "true" else 0,
            "energy": buffer,
            "capacity": capacity,
            "links": (0 if input_.strip() == "" else input_.count("("))
            + (0 if output.strip() == "" else output.count("(")),
            "input_links": 0 if input_.strip() == "" else input_.count("("),
            "output_links": 0 if output.strip() == "" else output.count("("),
            "pool_ready": ready,
            "pool_next": next_,
            "demand": demand,
            "t": ticks,
        }
        for name, value in values.items():
            print(f"{name}={value}")
        print("ok=1")
        return 0
    match = LIGHT.match(line)
    if match:
        (
            x,
            y,
            z,
            mode,
            lit,
            timer,
            period,
            draw,
            buffer,
            capacity,
            links,
            ready,
            next_,
            demand,
            ticks,
        ) = match.groups()
        # `lit`, not `powered`: a shell function that also used a local named `powered`
        # once shadowed the relay's parsed field through bash's dynamic scoping.
        values = {
            "kind": "light",
            "large": 0,
            "x": x,
            "y": y,
            "z": z,
            "mode": mode,
            "lit": 1 if lit == "true" else 0,
            "lamp_timer": timer,
            "lamp_period": period,
            "draw": draw,
            "energy": buffer,
            "capacity": capacity,
            "links": 0 if links.strip() == "" else links.count("("),
            "pool_ready": ready,
            "pool_next": next_,
            "demand": demand,
            "t": ticks,
        }
        for name, value in values.items():
            print(f"{name}={value}")
        print("ok=1")
        return 0
    match = CREATIVE.match(line)
    if match:
        x, y, z, ticks = match.groups()
        values = {
            "kind": "creative",
            "large": 0,
            "x": x,
            "y": y,
            "z": z,
            "mode": "none",
            "energy": 0,
            "capacity": 0,
            "links": 0,
            "pool_ready": 0,
            "pool_next": 0,
            "demand": 0,
            "t": ticks,
        }
        for name, value in values.items():
            print(f"{name}={value}")
        print("ok=1")
        return 0
    match = CONNECTOR.match(line)
    if match:
        kind, x, y, z, mode, buffer, capacity, links, ready, next_, demand, ticks = match.groups()
        count = 0 if links.strip() == "" else links.count("(")
        values = {
            "kind": "connector",
            "large": 1 if kind == "Large Connector" else 0,
            "x": x,
            "y": y,
            "z": z,
            "mode": mode,
            "energy": buffer,
            "capacity": capacity,
            "links": count,
            "pool_ready": ready,
            "pool_next": next_,
            "demand": demand,
            "t": ticks,
        }
    else:
        tower = TOWER.match(line)
        if tower:
            x, y, z, energy, capacity, width, height, cx, cy, cz, ticks = tower.groups()
            values = {
                "kind": "accumulator",
                "large": 0,
                "x": x,
                "y": y,
                "z": z,
                "mode": "none",
                "energy": energy,
                "capacity": capacity,
                "links": 0,
                "pool_ready": 0,
                "pool_next": 0,
                "demand": 0,
                "tower_w": width,
                "tower_h": height,
                "tower_x": cx,
                "tower_y": cy,
                "tower_z": cz,
                "t": ticks,
            }
            for name, value in values.items():
                print(f"{name}={value}")
            print("ok=1")
            return 0
        match = ACCUMULATOR.match(line)
        if not match:
            print("ok=0")
            return 0
        x, y, z, energy, capacity, ticks = match.groups()
        values = {
            "kind": "accumulator",
            "large": 0,
            "x": x,
            "y": y,
            "z": z,
            "mode": "none",
            "energy": energy,
            "capacity": capacity,
            "links": 0,
            "pool_ready": 0,
            "pool_next": 0,
            "demand": 0,
            "tower_w": 1,
            "tower_h": 1,
            "tower_x": x,
            "tower_y": y,
            "tower_z": z,
            "t": ticks,
        }
    for name, value in values.items():
        print(f"{name}={value}")
    print("ok=1")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
