"""Semantic identity verification for Iceberg REST catalog migration.

The module is deliberately transport-free. Adapters load source and destination
table responses, while this boundary canonicalizes the Iceberg state that must
survive registration or federation and returns an explicit loss report.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from enum import StrEnum
from typing import Any, Mapping


class MigrationDimension(StrEnum):
    TABLE_UUID = "table-uuid"
    FORMAT_VERSION = "format-version"
    SCHEMAS = "schemas"
    CURRENT_SCHEMA = "current-schema-id"
    PARTITION_SPECS = "partition-specs"
    DEFAULT_SPEC = "default-spec-id"
    SORT_ORDERS = "sort-orders"
    DEFAULT_SORT_ORDER = "default-sort-order-id"
    SNAPSHOTS = "snapshots"
    CURRENT_SNAPSHOT = "current-snapshot-id"
    REFS = "refs"
    METADATA_LOCATION = "metadata-location"


@dataclass(frozen=True)
class CatalogTableState:
    table_uuid: str
    format_version: int
    schemas: tuple[str, ...]
    current_schema_id: int
    partition_specs: tuple[str, ...]
    default_spec_id: int
    sort_orders: tuple[str, ...]
    default_sort_order_id: int
    snapshots: tuple[str, ...]
    current_snapshot_id: int | None
    refs: tuple[tuple[str, str], ...]
    metadata_location: str

    @classmethod
    def from_rest_response(cls, response: Mapping[str, Any]) -> CatalogTableState:
        metadata = _mapping(response, "metadata")
        return cls(
            table_uuid=_string(metadata, "table-uuid"),
            format_version=_integer(metadata, "format-version"),
            schemas=_canonical_array(metadata, "schemas"),
            current_schema_id=_integer(metadata, "current-schema-id"),
            partition_specs=_canonical_array(metadata, "partition-specs"),
            default_spec_id=_integer(metadata, "default-spec-id"),
            sort_orders=_canonical_array(metadata, "sort-orders"),
            default_sort_order_id=_integer(metadata, "default-sort-order-id"),
            snapshots=_canonical_array(metadata, "snapshots"),
            current_snapshot_id=_optional_integer(metadata, "current-snapshot-id"),
            refs=_canonical_refs(metadata),
            metadata_location=_string(response, "metadata-location"),
        )

    def digest(self) -> str:
        return _sha256(
            {
                "table-uuid": self.table_uuid,
                "format-version": self.format_version,
                "schemas": self.schemas,
                "current-schema-id": self.current_schema_id,
                "partition-specs": self.partition_specs,
                "default-spec-id": self.default_spec_id,
                "sort-orders": self.sort_orders,
                "default-sort-order-id": self.default_sort_order_id,
                "snapshots": self.snapshots,
                "current-snapshot-id": self.current_snapshot_id,
                "refs": self.refs,
                "metadata-location": self.metadata_location,
            }
        )


@dataclass(frozen=True)
class MigrationVerification:
    source_digest: str
    destination_digest: str
    mismatches: tuple[MigrationDimension, ...]
    snapshot_count: int
    ref_count: int

    @property
    def preserved(self) -> bool:
        return not self.mismatches

    @property
    def proves_nonempty_history(self) -> bool:
        return self.snapshot_count > 0 and self.ref_count > 0


def verify_migration(
    source: CatalogTableState, destination: CatalogTableState
) -> MigrationVerification:
    comparisons = (
        (MigrationDimension.TABLE_UUID, source.table_uuid, destination.table_uuid),
        (MigrationDimension.FORMAT_VERSION, source.format_version, destination.format_version),
        (MigrationDimension.SCHEMAS, source.schemas, destination.schemas),
        (MigrationDimension.CURRENT_SCHEMA, source.current_schema_id, destination.current_schema_id),
        (MigrationDimension.PARTITION_SPECS, source.partition_specs, destination.partition_specs),
        (MigrationDimension.DEFAULT_SPEC, source.default_spec_id, destination.default_spec_id),
        (MigrationDimension.SORT_ORDERS, source.sort_orders, destination.sort_orders),
        (
            MigrationDimension.DEFAULT_SORT_ORDER,
            source.default_sort_order_id,
            destination.default_sort_order_id,
        ),
        (MigrationDimension.SNAPSHOTS, source.snapshots, destination.snapshots),
        (
            MigrationDimension.CURRENT_SNAPSHOT,
            source.current_snapshot_id,
            destination.current_snapshot_id,
        ),
        (MigrationDimension.REFS, source.refs, destination.refs),
        (
            MigrationDimension.METADATA_LOCATION,
            source.metadata_location,
            destination.metadata_location,
        ),
    )
    return MigrationVerification(
        source_digest=source.digest(),
        destination_digest=destination.digest(),
        mismatches=tuple(dimension for dimension, left, right in comparisons if left != right),
        snapshot_count=len(source.snapshots),
        ref_count=len(source.refs),
    )


def _mapping(value: Mapping[str, Any], key: str) -> Mapping[str, Any]:
    candidate = value.get(key)
    if not isinstance(candidate, Mapping):
        raise ValueError(f"{key} must be an object")
    return candidate


def _string(value: Mapping[str, Any], key: str) -> str:
    candidate = value.get(key)
    if not isinstance(candidate, str) or not candidate:
        raise ValueError(f"{key} must be a non-empty string")
    return candidate


def _integer(value: Mapping[str, Any], key: str) -> int:
    candidate = value.get(key)
    if isinstance(candidate, bool) or not isinstance(candidate, int):
        raise ValueError(f"{key} must be an integer")
    return candidate


def _optional_integer(value: Mapping[str, Any], key: str) -> int | None:
    candidate = value.get(key)
    if candidate is None:
        return None
    if isinstance(candidate, bool) or not isinstance(candidate, int):
        raise ValueError(f"{key} must be an integer or null")
    return candidate


def _canonical_array(value: Mapping[str, Any], key: str) -> tuple[str, ...]:
    candidate = value.get(key)
    if not isinstance(candidate, list):
        raise ValueError(f"{key} must be an array")
    return tuple(_canonical(item) for item in candidate)


def _canonical_refs(metadata: Mapping[str, Any]) -> tuple[tuple[str, str], ...]:
    candidate = metadata.get("refs")
    if not isinstance(candidate, Mapping):
        raise ValueError("refs must be an object")
    return tuple(sorted((str(name), _canonical(ref)) for name, ref in candidate.items()))


def _canonical(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def _sha256(value: Any) -> str:
    return "sha256:" + hashlib.sha256(_canonical(value).encode()).hexdigest()
