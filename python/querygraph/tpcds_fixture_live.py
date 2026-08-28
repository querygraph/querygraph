from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from pyspark.sql import SparkSession

from querygraph.tpcds_fixture import fixture_plan, sql_literal


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--rest-uri", required=True)
    parser.add_argument("--warehouse", required=True)
    parser.add_argument("--s3-endpoint", required=True)
    parser.add_argument("--namespace", default="tpcds")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    model_bytes = args.model.read_bytes()
    if args.model.suffix != ".json":
        raise ValueError("live fixture input must be verified JSON")
    plan = fixture_plan(json.loads(model_bytes))
    spark = (SparkSession.builder.appName("querygraph-tpcds-fixture")
        .config("spark.sql.catalog.lakecat", "org.apache.iceberg.spark.SparkCatalog")
        .config("spark.sql.catalog.lakecat.type", "rest")
        .config("spark.sql.catalog.lakecat.uri", args.rest_uri)
        .config("spark.sql.catalog.lakecat.warehouse", args.warehouse)
        .config("spark.sql.catalog.lakecat.s3.endpoint", args.s3_endpoint)
        .config("spark.sql.catalog.lakecat.s3.path-style-access", "true")
        .getOrCreate())
    spark.sql(f"CREATE NAMESPACE IF NOT EXISTS lakecat.{args.namespace}")
    tables = []
    for fixture in plan:
        target = f"lakecat.{args.namespace}.{fixture.name}"
        spark.sql(f"DROP TABLE IF EXISTS {target}")
        spark.sql(f"CREATE TABLE {target} ({', '.join(f'{name} {kind}' for name, kind in fixture.columns)}) USING iceberg")
        values = ",".join("(" + ",".join(sql_literal(value, fixture.columns[i][1]) for i, value in enumerate(row)) + ")" for row in fixture.rows)
        spark.sql(f"INSERT INTO {target} VALUES {values}")
        rows = spark.sql(f"SELECT * FROM {target} ORDER BY 1").toJSON().collect()
        tables.append({"name": fixture.name, "columns": [name for name, _ in fixture.columns], "row-count": len(rows), "data-hash": "sha256:" + hashlib.sha256("\n".join(rows).encode()).hexdigest()})
    output = {"status": "verified", "model-hash": "sha256:" + hashlib.sha256(model_bytes).hexdigest(), "tables": tables}
    args.output.write_text(json.dumps(output, sort_keys=True, separators=(",", ":")) + "\n")
    spark.stop()


if __name__ == "__main__": main()
