from __future__ import annotations

import json
from functools import lru_cache
from pathlib import Path
from typing import Any

# The official OpenLineage spec schema, vendored so validation works offline.
OPENLINEAGE_SCHEMA_PATH = (
    Path(__file__).parent / "schemas" / "OpenLineage-2-0-2.json"
)


@lru_cache(maxsize=1)
def _openlineage_validator():
    try:
        import jsonschema
    except ImportError as exc:  # pragma: no cover - depends on optional extra.
        raise RuntimeError(
            "Install querygraph[validation] for official-schema validation."
        ) from exc
    schema = json.loads(OPENLINEAGE_SCHEMA_PATH.read_text(encoding="utf-8"))
    # The format checker enforces `format: uuid` on run.runId (and any other
    # format whose checker is installed); unknown formats are skipped per spec.
    return jsonschema.Draft202012Validator(
        schema, format_checker=jsonschema.FormatChecker()
    )


def validate_openlineage_schema(value: dict[str, Any]) -> list[str]:
    """Validate an event against the official OpenLineage 2-0-2 JSON Schema.

    Returns human-readable error strings; empty means conformant. Unlike the
    shape checks below, this proves interop with OSS consumers (Marquez,
    openlineage-python) rather than asserting it.
    """
    return [
        f"{'/'.join(str(part) for part in error.absolute_path) or '<root>'}: {error.message}"
        for error in _openlineage_validator().iter_errors(value)
    ]


def validate_croissant(value: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    _require(value, "@type", "cr:Dataset", errors)
    _require_present(value, "@id", errors)
    _require_present(value, "recordSet", errors)
    return errors


def validate_cdif(value: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    _require(value, "@type", "dcat:Dataset", errors)
    _require_present(value, "cdif:profile", errors)
    _require_present(value, "dct:accessRights", errors)
    _require_present(value, "cdif:dataElement", errors)
    return errors


def validate_openlineage(value: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    _require_present(value, "eventType", errors)
    _require_present(value, "eventTime", errors)
    _require_present(value, "run", errors)
    _require_present(value, "job", errors)
    _require_present(value, "inputs", errors)
    _require_present(value, "outputs", errors)
    return errors


def _require(value: dict[str, Any], key: str, expected: Any, errors: list[str]) -> None:
    if value.get(key) != expected:
        errors.append(f"{key} must be {expected!r}")


def _require_present(value: dict[str, Any], key: str, errors: list[str]) -> None:
    if key not in value or value[key] is None:
        errors.append(f"{key} is required")
