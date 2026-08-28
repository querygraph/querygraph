#!/usr/bin/env python3
"""Fetch the accepted Apache Ossie surfaces and reject byte drift."""

from __future__ import annotations

import argparse
import hashlib
import json
import urllib.request
from pathlib import Path


def fetch(manifest_path: Path, output: Path) -> None:
    manifest = json.loads(manifest_path.read_text())
    revision = manifest["revision"]
    base = f"https://raw.githubusercontent.com/apache/ossie/{revision}"
    for relative, expected in manifest["artifacts"].items():
        target = output / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        data = urllib.request.urlopen(f"{base}/{relative}", timeout=30).read()
        actual = hashlib.sha256(data).hexdigest()
        if actual != expected:
            raise RuntimeError(f"Ossie artifact drift for {relative}: {actual}")
        target.write_bytes(data)


def verify(manifest_path: Path, output: Path) -> None:
    manifest = json.loads(manifest_path.read_text())
    for relative, expected in manifest["artifacts"].items():
        data = (output / relative).read_bytes()
        actual = hashlib.sha256(data).hexdigest()
        if actual != expected:
            raise RuntimeError(f"Ossie artifact drift for {relative}: {actual}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("fetch", "verify"))
    parser.add_argument("output", type=Path)
    parser.add_argument("--manifest", type=Path, default=Path("ossie/upstream.json"))
    args = parser.parse_args()
    (fetch if args.command == "fetch" else verify)(args.manifest, args.output)


if __name__ == "__main__":
    main()
