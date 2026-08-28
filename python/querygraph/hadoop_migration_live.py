"""Stock Spark HadoopCatalog-to-Iceberg-REST migration verification."""

from __future__ import annotations

import argparse
import hashlib
import json
from dataclasses import asdict, dataclass
from typing import Any, Sequence


@dataclass(frozen=True)
class LegacyMigrationState:
    schema: str
    specs: tuple[tuple[int, str], ...]
    current_spec_id: int
    snapshots: tuple[int, ...]
    current_snapshot_id: int
    refs: tuple[tuple[str, str], ...]
    metadata_location: str


@dataclass(frozen=True)
class LegacyMigrationVerification:
    source_digest: str
    destination_digest: str
    mismatches: tuple[str, ...]
    snapshot_count: int
    spec_count: int
    ref_count: int

    @property
    def preserved(self) -> bool:
        return not self.mismatches


def verify_legacy_migration(
    source: LegacyMigrationState, destination: LegacyMigrationState
) -> LegacyMigrationVerification:
    fields = (
        "schema",
        "specs",
        "current_spec_id",
        "snapshots",
        "current_snapshot_id",
        "refs",
        "metadata_location",
    )
    return LegacyMigrationVerification(
        source_digest=_digest(asdict(source)),
        destination_digest=_digest(asdict(destination)),
        mismatches=tuple(
            field
            for field in fields
            if getattr(source, field) != getattr(destination, field)
        ),
        snapshot_count=len(source.snapshots),
        spec_count=len(source.specs),
        ref_count=len(source.refs),
    )


def canonical_rows(rows: Sequence[Any]) -> tuple[int, str]:
    values = sorted(
        ([value for value in row] for row in rows),
        key=lambda row: (row[0], *["" if item is None else str(item) for item in row[1:]]),
    )
    encoded = json.dumps(values, separators=(",", ":"), ensure_ascii=False).encode()
    return len(values), "sha256:" + hashlib.sha256(encoded).hexdigest()


def _digest(value: Any) -> str:
    encoded = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode()
    return "sha256:" + hashlib.sha256(encoded).hexdigest()


def _java_map_items(value: Any) -> list[tuple[Any, Any]]:
    return [(entry.getKey(), entry.getValue()) for entry in value.entrySet()]


def _state(table: Any) -> LegacyMigrationState:
    metadata = table.operations().current()
    specs = tuple(
        sorted(
            (int(spec_id), str(spec))
            for spec_id, spec in _java_map_items(metadata.specsById())
        )
    )
    snapshots = tuple(sorted(int(snapshot.snapshotId()) for snapshot in metadata.snapshots()))
    refs = tuple(
        sorted(
            (str(name), str(ref)) for name, ref in _java_map_items(metadata.refs())
        )
    )
    return LegacyMigrationState(
        schema=str(metadata.schema()),
        specs=specs,
        current_spec_id=int(metadata.defaultSpecId()),
        snapshots=snapshots,
        current_snapshot_id=int(metadata.currentSnapshot().snapshotId()),
        refs=refs,
        metadata_location=str(metadata.metadataFileLocation()),
    )


def run(fixture: str) -> dict[str, Any]:
    from pyspark.sql import SparkSession

    namespace = f"qg_hadoop_{fixture}"
    identifier = f"{namespace}.events"
    spark = (
        SparkSession.builder.appName("querygraph-hadoop-migration")
        .config(
            "spark.sql.extensions",
            "org.apache.iceberg.spark.extensions.IcebergSparkSessionExtensions",
        )
        .config("spark.sql.catalog.hadoop", "org.apache.iceberg.spark.SparkCatalog")
        .config("spark.sql.catalog.hadoop.type", "hadoop")
        .config("spark.sql.catalog.hadoop.warehouse", "file:///migration/hadoop")
        .config("spark.sql.catalog.lakecat", "org.apache.iceberg.spark.SparkCatalog")
        .config("spark.sql.catalog.lakecat.type", "rest")
        .config("spark.sql.catalog.lakecat.uri", "http://lakecat:8181/catalog")
        .config("spark.sql.catalog.lakecat.warehouse", "local")
        .config(
            "spark.sql.catalog.lakecat.io-impl",
            "org.apache.iceberg.hadoop.HadoopFileIO",
        )
        .getOrCreate()
    )
    spark.sparkContext.setLogLevel("WARN")
    try:
        spark.sql(f"CREATE NAMESPACE hadoop.{namespace}")
        spark.sql(
            f"""CREATE TABLE hadoop.{identifier} (
                    id BIGINT NOT NULL,
                    category STRING NOT NULL
                ) USING iceberg
                PARTITIONED BY (bucket(4, id))
                TBLPROPERTIES ('format-version'='2')"""
        )
        spark.sql(
            f"INSERT INTO hadoop.{identifier} VALUES (1, 'aa'), (2, 'bb')"
        )
        spark.sql(f"ALTER TABLE hadoop.{identifier} ADD COLUMN note STRING")
        spark.sql(
            f"INSERT INTO hadoop.{identifier} VALUES (3, 'cc', 'evolved')"
        )
        spark.sql(
            f"ALTER TABLE hadoop.{identifier} ADD PARTITION FIELD truncate(2, category)"
        )
        spark.sql(f"ALTER TABLE hadoop.{identifier} CREATE BRANCH audit")

        spark.sql(f"CREATE NAMESPACE lakecat.{namespace}")
        metadata_location = spark.sql(
            f"SELECT file FROM hadoop.{identifier}.metadata_log_entries "
            "ORDER BY timestamp DESC LIMIT 1"
        ).first()[0]
        spark.sql(
            f"CALL lakecat.system.register_table(table => 'lakecat.{identifier}', "
            f"metadata_file => '{metadata_location}')"
        ).collect()

        load_table = spark._jvm.org.apache.iceberg.spark.Spark3Util.loadIcebergTable
        source_table = load_table(spark._jsparkSession, f"hadoop.{identifier}")
        destination_table = load_table(spark._jsparkSession, f"lakecat.{identifier}")
        source_state = _state(source_table)
        destination_state = _state(destination_table)
        verification = verify_legacy_migration(source_state, destination_state)
        source_rows = spark.sql(f"SELECT * FROM hadoop.{identifier}").collect()
        destination_rows = spark.sql(f"SELECT * FROM lakecat.{identifier}").collect()
        source_count, source_digest = canonical_rows(source_rows)
        destination_count, destination_digest = canonical_rows(destination_rows)
        result = {
            "contract": "querygraph/hadoop-to-rest-migration/v1",
            "fixture": fixture,
            "source_catalog": "Apache Iceberg HadoopCatalog",
            "destination_catalog": "LakeCat Iceberg REST",
            "semantic": {
                **asdict(verification),
                "preserved": verification.preserved,
                "metadata_location_digest": _digest(source_state.metadata_location),
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
            and verification.snapshot_count >= 2
            and verification.spec_count >= 2
            and verification.ref_count >= 2
            and result["data"]["preserved"]
        ):
            raise RuntimeError("legacy migration verification failed: " + json.dumps(result))
        return result
    finally:
        for catalog in ("lakecat", "hadoop"):
            try:
                spark.sql(f"DROP TABLE IF EXISTS {catalog}.{identifier}")
                spark.sql(f"DROP NAMESPACE IF EXISTS {catalog}.{namespace}")
            except Exception:  # noqa: BLE001 - preserve the primary proof failure
                pass
        spark.stop()


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture", required=True)
    args = parser.parse_args(argv)
    print(json.dumps(run(args.fixture), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
