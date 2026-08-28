"""Live, stock-PyIceberg migration proof for REST catalogs.

This optional operational module is intentionally absent from QueryGraph's base
dependencies.  Run it in the catalog-bench pinned PyIceberg image and mount the
QueryGraph Python source at ``/querygraph``.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from dataclasses import asdict, dataclass
from typing import Any, Mapping, Sequence

from querygraph.catalog_migration import CatalogTableState, verify_migration


@dataclass(frozen=True)
class CatalogEndpoint:
    name: str
    uri: str
    warehouse: str | None = None
    credential_env: str | None = None
    oauth_uri: str | None = None
    scope: str | None = None

    def properties(self) -> dict[str, str]:
        value = {
            "uri": self.uri,
            "s3.endpoint": os.environ["CATALOG_BENCH_S3_ENDPOINT"],
            "s3.region": os.environ.get("CATALOG_BENCH_S3_REGION", "us-east-1"),
            "s3.access-key-id": os.environ["CATALOG_BENCH_S3_ACCESS_KEY_ID"],
            "s3.secret-access-key": os.environ["CATALOG_BENCH_S3_SECRET_ACCESS_KEY"],
        }
        if self.warehouse:
            value["warehouse"] = self.warehouse
        if self.credential_env:
            value["credential"] = os.environ[self.credential_env]
        if self.oauth_uri:
            value["oauth2-server-uri"] = self.oauth_uri
        if self.scope:
            value["scope"] = self.scope
        return value


def table_state(table: Any) -> CatalogTableState:
    """Convert a stock PyIceberg table into the transport-neutral verifier state."""

    metadata = table.metadata.model_dump(mode="json", by_alias=True)
    return CatalogTableState.from_rest_response(
        {"metadata": metadata, "metadata-location": table.metadata_location}
    )


def canonical_rows(rows: Sequence[Mapping[str, Any]]) -> tuple[int, str]:
    encoded = json.dumps(
        sorted((dict(row) for row in rows), key=lambda row: row["id"]),
        sort_keys=True,
        separators=(",", ":"),
    ).encode()
    return len(rows), "sha256:" + hashlib.sha256(encoded).hexdigest()


def migrate(
    source_endpoint: CatalogEndpoint,
    destination_endpoint: CatalogEndpoint,
    fixture: str,
    table_location: str | None = None,
) -> dict[str, Any]:
    import pyarrow
    from pyiceberg.catalog.rest import RestCatalog
    from pyiceberg.schema import Schema
    from pyiceberg.types import LongType, NestedField, StringType

    source = RestCatalog(source_endpoint.name, **source_endpoint.properties())
    destination = RestCatalog(
        destination_endpoint.name, **destination_endpoint.properties()
    )
    namespace = (f"qg_migration_{fixture}",)
    identifier = (*namespace, "events")
    rows = [
        {"id": 1, "event": "created"},
        {"id": 2, "event": "migrated"},
        {"id": 3, "event": "verified"},
    ]
    try:
        source.create_namespace(namespace)
        create_options: dict[str, Any] = {
            "schema": Schema(
                NestedField(1, "id", LongType(), required=True),
                NestedField(2, "event", StringType(), required=True),
            ),
        }
        if table_location:
            create_options["location"] = table_location
        source_table = source.create_table(identifier, **create_options)
        arrow_schema = pyarrow.schema(
            [
                pyarrow.field("id", pyarrow.int64(), nullable=False),
                pyarrow.field("event", pyarrow.string(), nullable=False),
            ]
        )
        source_table.append(pyarrow.Table.from_pylist(rows, schema=arrow_schema))
        source_table.refresh()
        destination.create_namespace(namespace)
        destination.register_table(identifier, source_table.metadata_location)
        destination_table = destination.load_table(identifier)

        source_state = table_state(source_table)
        destination_state = table_state(destination_table)
        verification = verify_migration(source_state, destination_state)
        source_rows = source_table.scan().to_arrow().to_pylist()
        destination_rows = destination_table.scan().to_arrow().to_pylist()
        source_count, source_digest = canonical_rows(source_rows)
        destination_count, destination_digest = canonical_rows(destination_rows)
        result = {
            "contract": "querygraph/catalog-migration-live/v1",
            "source": source_endpoint.name,
            "destination": destination_endpoint.name,
            "fixture": fixture,
            "semantic": {
                **asdict(verification),
                "mismatches": [item.value for item in verification.mismatches],
                "preserved": verification.preserved,
                "proves_nonempty_history": verification.proves_nonempty_history,
            },
            "data": {
                "source_count": source_count,
                "destination_count": destination_count,
                "source_digest": source_digest,
                "destination_digest": destination_digest,
                "preserved": source_count == destination_count
                and source_digest == destination_digest,
            },
        }
        if not (
            verification.preserved
            and verification.proves_nonempty_history
            and result["data"]["preserved"]
        ):
            raise RuntimeError("migration verification failed: " + json.dumps(result))
        return result
    finally:
        for catalog in (destination, source):
            try:
                catalog.drop_table(identifier)
            except Exception:  # noqa: BLE001 - cleanup is best effort after proof
                pass
            try:
                catalog.drop_namespace(namespace)
            except Exception:  # noqa: BLE001 - cleanup is best effort after proof
                pass
            catalog.close()


def _endpoint(prefix: str) -> CatalogEndpoint:
    credential_env = os.environ.get(f"{prefix}_CREDENTIAL_ENV")
    return CatalogEndpoint(
        name=os.environ[f"{prefix}_NAME"],
        uri=os.environ[f"{prefix}_URI"],
        warehouse=os.environ.get(f"{prefix}_WAREHOUSE"),
        credential_env=credential_env,
        oauth_uri=os.environ.get(f"{prefix}_OAUTH_URI"),
        scope=os.environ.get(f"{prefix}_SCOPE"),
    )


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture", required=True)
    args = parser.parse_args(argv)
    print(
        json.dumps(
            migrate(
                _endpoint("SOURCE"),
                _endpoint("DESTINATION"),
                args.fixture,
                os.environ.get("SOURCE_TABLE_LOCATION"),
            ),
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
