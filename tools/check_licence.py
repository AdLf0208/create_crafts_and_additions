#!/usr/bin/env python3
"""Check that the built package carries the upstream MIT notice verbatim.

The package must ship the upstream `LICENSE` text itself (copyright line and
permission notice), not only this port's reworded grant: the MIT notice has to
travel with the redistributed code and art. `LICENSE-UPSTREAM` in the port tree
is the file to compare against, so a drift in either copy fails.

    python3 tools/check_licence.py [createaddition.hmod]

Exit 0 when the package carries the notice; 1 (with the reason) when it does not.
"""
from __future__ import annotations

import sys
import zipfile
from pathlib import Path

PORT = Path(__file__).resolve().parents[1]
WANT_COPYRIGHT = "Copyright (c) 2024 MRH"


def main(argv: list[str]) -> int:
    package = Path(argv[1]) if len(argv) > 1 else PORT / "createaddition.hmod"
    if not package.is_file():
        print(f"FAIL: no package at {package}")
        return 1
    upstream = PORT / "LICENSE-UPSTREAM"
    if not upstream.is_file():
        print("FAIL: the port has no LICENSE-UPSTREAM to compare against")
        return 1
    want = upstream.read_bytes()
    with zipfile.ZipFile(package) as archive:
        names = archive.namelist()
        if "LICENSE-UPSTREAM" not in names:
            print(f"FAIL: {package.name} carries no LICENSE-UPSTREAM (the upstream MIT notice)")
            print(f"      entries: {', '.join(sorted(names))[:400]}")
            return 1
        packed = archive.read("LICENSE-UPSTREAM")
        for required in ("LICENSE", "LICENSE-NOTICE.md", "README.md"):
            if required not in names:
                print(f"FAIL: {package.name} carries no {required}")
                return 1
    if packed != want:
        print(f"FAIL: packed LICENSE-UPSTREAM differs from {upstream}")
        return 1
    text = packed.decode("utf-8", "replace")
    if WANT_COPYRIGHT not in text or "Permission is hereby granted" not in text:
        print(f"FAIL: packed LICENSE-UPSTREAM lacks '{WANT_COPYRIGHT}' or the MIT grant")
        return 1
    print(f"ok   {package.name} carries the upstream MIT notice verbatim "
          f"({len(packed)} B, '{WANT_COPYRIGHT}')")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
