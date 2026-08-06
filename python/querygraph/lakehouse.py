from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class TableSpec:
    logical_name: str
    bare_name: str
    rows: int
    location: Path


def load_table_specs(
    manifest: str | Path,
    warehouse: str | Path,
) -> list[TableSpec]:
    report = json.loads(Path(manifest).read_text())
    warehouse_path = Path(warehouse).resolve()
    specs: list[TableSpec] = []
    for dataset in report.get("datasets", []):
        for file in dataset.get("files", []):
            table = file.get("table")
            rows = file.get("rows")
            if not table or rows is None:
                continue
            bare = table.split(".", 1)[-1]
            specs.append(
                TableSpec(
                    logical_name=table,
                    bare_name=bare,
                    rows=int(rows),
                    location=find_latest_parquet_dir(warehouse_path, bare),
                )
            )
    return specs


def find_latest_parquet_dir(warehouse: Path, table: str) -> Path:
    matches = sorted(
        [path for path in warehouse.iterdir() if path.is_dir() and path.name.startswith(f"{table}-")],
        key=lambda path: path.stat().st_mtime,
        reverse=True,
    )
    if not matches:
        raise FileNotFoundError(f"no Parquet directory found for {table} in {warehouse}")
    return matches[0]


def spark_session(remote: str = "sc://127.0.0.1:50051"):
    try:
        from pyspark.sql import SparkSession
    except ImportError as exc:  # pragma: no cover - depends on optional extra.
        raise RuntimeError("Install querygraph[lakehouse] to use PySpark helpers.") from exc
    return SparkSession.builder.remote(remote).getOrCreate()


def register_lakehouse(
    *,
    manifest: str | Path = ".querygraph/lakehouse/manifest/load-report.json",
    warehouse: str | Path = "spark-warehouse",
    remote: str = "sc://127.0.0.1:50051",
    create_global_temp: bool = True,
) -> list[dict[str, Any]]:
    spark = spark_session(remote)
    results: list[dict[str, Any]] = []
    for spec in load_table_specs(manifest, warehouse):
        df = spark.read.parquet(str(spec.location))
        df.createOrReplaceTempView(spec.bare_name)
        if create_global_temp:
            df.createOrReplaceGlobalTempView(spec.bare_name)
        observed = df.count()
        results.append(
            {
                "table": spec.bare_name,
                "logicalName": spec.logical_name,
                "rows": observed,
                "expectedRows": spec.rows,
                "location": str(spec.location),
                "status": "ok" if observed == spec.rows else "mismatch",
            }
        )
    return results


def register_audit(
    *,
    warehouse: str | Path = "spark-warehouse",
    remote: str = "sc://127.0.0.1:50051",
    create_global_temp: bool = True,
    tables: tuple[str, ...] = ("openlineage_events", "openlineage_attestations"),
) -> list[dict[str, Any]]:
    spark = spark_session(remote)
    warehouse_path = Path(warehouse).resolve()
    results: list[dict[str, Any]] = []
    for table in tables:
        location = find_latest_parquet_dir(warehouse_path, table)
        df = spark.read.parquet(str(location))
        df.createOrReplaceTempView(table)
        if create_global_temp:
            df.createOrReplaceGlobalTempView(table)
        results.append({"table": table, "rows": df.count(), "location": str(location)})
    return results


def example_queries(scope: str = "global_temp") -> list[str]:
    return [
        f"SELECT COUNT(*) AS rows FROM {scope}.government_finance__countydata",
        f"SELECT COUNT(*) AS rows FROM {scope}.codata_constants_2022__codata_constants_2022",
        f"SELECT quantity, value, unit FROM {scope}.codata_constants_2022__codata_constants_2022 LIMIT 5",
        f"SELECT event_hash, event_type, job_name FROM {scope}.openlineage_events LIMIT 10",
    ]
