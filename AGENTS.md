# QueryGraph Agent Notes

## Stack Dependency Graph

[`QUERYGRAPH.md`](QUERYGRAPH.md) is the authoritative record of which released
sibling versions every repository in the QueryGraph family pins. Before any
release of QueryGraph, Grust, TypeSec, Marciana, or LakeCat, run

```bash
scripts/check-stack-dependencies.py --check
```

and regenerate the matrix with `--write` in the same change that bumps a pin.
Release order is the graph order (`grust → typesec → marciana → lakecat →
querygraph`); a stale pin anywhere in that chain is a release blocker, and
committed manifests never reach a sibling through a `path` or `git`
dependency.

## Rust Engineering

Before changing Rust source, tests, benchmarks, Cargo configuration, or a
Rust-facing wire contract, read [`RUST.md`](RUST.md) in full and follow it as the
repository's normative Rust engineering guide. In particular, all Rust test
bodies belong in separate files, domain alternatives should use algebraic data
types, advanced type-level abstractions must improve real guarantees without
type gymnastics, and performance claims require optimized representative
benchmarks.

## Python Environment

Use the shared Python bootstrap helper instead of Homebrew or system Python:

```bash
publishing/scripts/ensure-python-env.sh python
```

The helper resolves the project root, prefers `asdf which python` when
available, and runs `uv sync` for the selected uv project. Use the printed
interpreter path only when a tool needs an explicit Python binary; otherwise run
commands through `uv run` from the target project:

```bash
cd python
uv run python -m pytest
```

For the Sail lakehouse client, use the same helper against the lakehouse uv
project:

```bash
publishing/scripts/ensure-python-env.sh lakehouse/python
uv run --project lakehouse/python python \
  lakehouse/python/register_lakehouse.py --help
```

Do not use the Homebrew Python interpreter for QueryGraph Python work; it has
caused `pyexpat` and venv failures on this machine.
