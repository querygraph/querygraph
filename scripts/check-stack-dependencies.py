#!/usr/bin/env python3
"""Stack-wide dependency graph status for the QueryGraph family of repositories.

Reads every sibling checkout's Cargo manifests, derives the dependency edges
between the repositories, and compares each pinned requirement with the
sibling's current workspace version. `--check` fails when a requirement lags
the sibling line, when a committed manifest reaches a sibling through a `path`
or `git` dependency, or when `QUERYGRAPH.md` no longer matches the derived
matrix. `--write` regenerates the matrix block in `QUERYGRAPH.md`.

Standard library only (Python 3.11+ for `tomllib`).
"""
from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path

REPOS: dict[str, dict[str, object]] = {
    # repo -> {crates: [...], root: env override, kind}
    "grust": {"env": "QG_STACK_GRUST", "default": "../grust"},
    "typesec": {"env": "QG_STACK_TYPESEC", "default": "../typesec"},
    "marciana": {"env": "QG_STACK_MARCIANA", "default": "../marciana"},
    "lakecat": {"env": "QG_STACK_LAKECAT", "default": "../lakecat"},
    "querygraph": {"env": "QG_STACK_QUERYGRAPH", "default": "."},
}
ORDER = ["grust", "typesec", "marciana", "lakecat", "querygraph"]
BEGIN = "<!-- stack-dependency-matrix:begin -->"
END = "<!-- stack-dependency-matrix:end -->"


def repo_root(name: str) -> Path:
    spec = REPOS[name]
    override = os.environ.get(str(spec["env"]))
    here = Path(__file__).resolve().parent.parent
    return Path(override).resolve() if override else (here / str(spec["default"])).resolve()


def load_toml(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def workspace_version(root: Path) -> str:
    manifest = load_toml(root / "Cargo.toml")
    if "workspace" in manifest and "package" in manifest["workspace"]:
        return str(manifest["workspace"]["package"]["version"])
    return str(manifest["package"]["version"])


def workspace_manifests(root: Path) -> list[Path]:
    manifest = load_toml(root / "Cargo.toml")
    paths = [root / "Cargo.toml"]
    members = manifest.get("workspace", {}).get("members", [])
    for member in members:
        if "*" in member:
            paths.extend(sorted((root / member.rstrip("/*")).glob("*/Cargo.toml")))
        else:
            candidate = root / member / "Cargo.toml"
            if candidate.exists():
                paths.append(candidate)
    return paths


def owned_crates(root: Path) -> set[str]:
    names: set[str] = set()
    for path in workspace_manifests(root):
        manifest = load_toml(path)
        package = manifest.get("package")
        if package and "name" in package:
            names.add(str(package["name"]))
    return names


def dependency_entries(manifest: dict):
    """Yield (name, spec) for every dependency table in a manifest."""
    tables = ["dependencies", "dev-dependencies", "build-dependencies"]
    scopes = [manifest]
    if "workspace" in manifest:
        scopes.append(manifest["workspace"])
    for scope in scopes:
        for table in tables:
            for name, spec in scope.get(table, {}).items():
                yield name, spec
        for target in scope.get("target", {}).values():
            for table in tables:
                for name, spec in target.get(table, {}).items():
                    yield name, spec


def crate_name(name: str, spec) -> str:
    if isinstance(spec, dict) and "package" in spec:
        return str(spec["package"])
    return name


def requirement(spec) -> tuple[str | None, str]:
    """Return (version requirement, source kind)."""
    if isinstance(spec, str):
        return spec, "registry"
    if not isinstance(spec, dict):
        return None, "unknown"
    if spec.get("workspace"):
        return None, "workspace"
    kind = "registry"
    if "path" in spec:
        kind = "path"
    if "git" in spec:
        kind = "git"
    return (str(spec["version"]) if "version" in spec else None), kind


def parse_version(text: str) -> tuple[int, int, int, str]:
    core, _, pre = text.partition("-")
    parts = [int(p) for p in core.split(".")[:3]]
    while len(parts) < 3:
        parts.append(0)
    return parts[0], parts[1], parts[2], pre


def caret_satisfied(req: str, version: str) -> bool:
    """Cargo caret semantics for the plain `x.y.z` requirements the stack uses."""
    req = req.strip()
    if req.startswith("="):
        return parse_version(req[1:])[:3] == parse_version(version)[:3]
    req = req.lstrip("^")
    rmaj, rmin, rpatch, _ = parse_version(req)
    vmaj, vmin, vpatch, _ = parse_version(version)
    if (vmaj, vmin, vpatch) < (rmaj, rmin, rpatch):
        return False
    if rmaj > 0:
        return vmaj == rmaj
    if rmin > 0:
        return vmaj == 0 and vmin == rmin
    return vmaj == 0 and vmin == 0 and vpatch == rpatch


def git_short(root: Path) -> str:
    try:
        return subprocess.run(
            ["git", "-C", str(root), "rev-parse", "--short=8", "HEAD"],
            capture_output=True, text=True, check=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return "?"


def collect():
    roots = {name: repo_root(name) for name in ORDER}
    missing = [name for name, root in roots.items() if not (root / "Cargo.toml").exists()]
    if missing:
        sys.exit(f"missing sibling checkouts: {', '.join(missing)} "
                 f"(set QG_STACK_<REPO> to point at them)")
    versions = {name: workspace_version(root) for name, root in roots.items()}
    owners: dict[str, str] = {}
    for name, root in roots.items():
        for crate in owned_crates(root):
            owners[crate] = name
    edges = []  # (consumer, manifest, crate, owner, req, kind, owner_version, ok, published)
    problems = []
    warnings = []
    for consumer, root in roots.items():
        for manifest_path in workspace_manifests(root):
            manifest = load_toml(manifest_path)
            package = manifest.get("package", {})
            published = package.get("publish", True) is not False
            for name, spec in dependency_entries(manifest):
                crate = crate_name(name, spec)
                owner = owners.get(crate)
                if owner is None or owner == consumer:
                    continue
                req, kind = requirement(spec)
                if kind == "workspace":
                    continue  # resolved through the consumer's own [workspace.dependencies]
                owner_version = versions[owner]
                rel = manifest_path.relative_to(root)
                sink = problems if published else warnings
                if kind in ("path", "git"):
                    ok = False
                    sink.append(
                        f"{consumer}/{rel}: {crate} reaches {owner} through a {kind} "
                        f"dependency; committed manifests must use released versions")
                elif req is None:
                    ok = False
                    sink.append(f"{consumer}/{rel}: {crate} has no version requirement")
                else:
                    ok = caret_satisfied(req, owner_version)
                    if not ok:
                        sink.append(
                            f"{consumer}/{rel}: {crate} requires {req} but {owner} is at "
                            f"{owner_version}")
                edges.append((consumer, str(rel), crate, owner, req or f"<{kind}>", kind,
                              owner_version, ok, published))
    return roots, versions, edges, problems, warnings


def render(roots, versions, edges) -> str:
    lines = []
    lines.append("| Repository | Workspace version | HEAD | Depends on |")
    lines.append("|---|---|---|---|")
    for name in ORDER:
        deps = sorted({e[3] for e in edges if e[0] == name})
        lines.append(f"| {name} | `{versions[name]}` | `{git_short(roots[name])}` | "
                     f"{', '.join(deps) if deps else '—'} |")
    lines.append("")
    lines.append("| Consumer | Manifest | Crate | Owner | Required | Owner version | Status |")
    lines.append("|---|---|---|---|---|---|---|")
    seen = set()
    for consumer, rel, crate, owner, req, kind, owner_version, ok, published in sorted(
        edges, key=lambda e: (ORDER.index(e[0]), e[1], e[2])
    ):
        key = (consumer, rel, crate, req)
        if key in seen:
            continue
        seen.add(key)
        if ok:
            status = "aligned"
        elif published:
            status = "**LAGGING**"
        else:
            status = "lagging (unpublished crate)"
        lines.append(f"| {consumer} | `{rel}` | `{crate}` | {owner} | `{req}` | "
                     f"`{owner_version}` | {status} |")
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="regenerate the matrix block")
    parser.add_argument("--check", action="store_true", help="fail on lag or a stale matrix")
    parser.add_argument("--strict", action="store_true",
                        help="also fail on lag inside publish = false crates")
    parser.add_argument("--doc", default=str(Path(__file__).resolve().parent.parent / "QUERYGRAPH.md"))
    args = parser.parse_args()

    roots, versions, edges, problems, warnings = collect()
    if args.strict:
        problems.extend(warnings)
        warnings = []
    matrix = render(roots, versions, edges)
    doc = Path(args.doc)
    text = doc.read_text() if doc.exists() else ""
    pattern = re.compile(re.escape(BEGIN) + r".*?" + re.escape(END), re.S)
    block = f"{BEGIN}\n{matrix}{END}"

    if args.write:
        if pattern.search(text):
            text = pattern.sub(lambda _m: block, text)
        else:
            text = text.rstrip("\n") + "\n\n" + block + "\n"
        doc.write_text(text)
        print(f"wrote {doc}")

    if not args.write and not args.check:
        print(matrix, end="")

    if args.check:
        match = pattern.search(text)
        current = match.group(0) if match else ""
        stale_lines = [
            line for line in current.splitlines()
            if line.startswith("| ") and "HEAD" not in line and "`" in line
        ]
        expected_lines = [
            line for line in block.splitlines()
            if line.startswith("| ") and "HEAD" not in line and "`" in line
        ]
        # Ignore the HEAD column when comparing the repository table rows.
        strip_head = lambda rows: [re.sub(r"\| `[0-9a-f?]{1,8}` \|", "| … |", r) for r in rows]
        if strip_head(stale_lines) != strip_head(expected_lines):
            problems.append(f"{doc.name} matrix is stale; run scripts/check-stack-dependencies.py --write")
        for warning in warnings:
            print(f"stack dependency warning (unpublished crate): {warning}", file=sys.stderr)
        for problem in problems:
            print(f"stack dependency drift: {problem}", file=sys.stderr)
        if problems:
            return 1
        print("QueryGraph stack dependency graph is aligned.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
