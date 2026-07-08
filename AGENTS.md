# QueryGraph Agent Notes

## Python Environment

Use the shared Python bootstrap helper instead of Homebrew or system Python:

```bash
publishing/scripts/ensure-python-env.sh qg-python
```

The helper resolves the project root, prefers `asdf which python` when
available, and runs `uv sync` for the selected uv project. Use the printed
interpreter path only when a tool needs an explicit Python binary; otherwise run
commands through `uv run` from the target project:

```bash
cd qg-python
uv run python -m pytest
```

For the Sail lakehouse client, use the same helper against the lakehouse uv
project:

```bash
publishing/scripts/ensure-python-env.sh qg-rust/lakehouse/python
uv run --project qg-rust/lakehouse/python python qg-rust/lakehouse/python/register_lakehouse.py --help
```

Do not use the Homebrew Python interpreter for QueryGraph Python work; it has
caused `pyexpat` and venv failures on this machine.
