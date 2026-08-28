from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
import urllib.request
from pathlib import Path

from pyspark.sql import SparkSession

_FIXTURE_PATH = Path(__file__).with_name("tpcds_fixture.py")
_SPEC = importlib.util.spec_from_file_location("querygraph_tpcds_fixture", _FIXTURE_PATH)
if _SPEC is None or _SPEC.loader is None:
    raise RuntimeError("cannot load the QueryGraph TPC-DS fixture planner")
_FIXTURE = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = _FIXTURE
_SPEC.loader.exec_module(_FIXTURE)
fixture_plan = _FIXTURE.fixture_plan
sql_literal = _FIXTURE.sql_literal

_PROOF_PATH = Path(__file__).with_name("semantic_answer_proof.py")
_PROOF_SPEC = importlib.util.spec_from_file_location("querygraph_semantic_answer_proof", _PROOF_PATH)
if _PROOF_SPEC is None or _PROOF_SPEC.loader is None:
    raise RuntimeError("cannot load the QueryGraph semantic answer verifier")
_PROOF = importlib.util.module_from_spec(_PROOF_SPEC)
sys.modules[_PROOF_SPEC.name] = _PROOF
_PROOF_SPEC.loader.exec_module(_PROOF)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--rest-uri", required=True)
    parser.add_argument("--warehouse", required=True)
    parser.add_argument("--s3-endpoint", required=True)
    parser.add_argument("--namespace", default="tpcds")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--artifact-uri", required=True)
    parser.add_argument("--artifact-hash", required=True)
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
    physical_bindings = {}
    snapshots = {}
    for fixture in plan:
        target = f"lakecat.{args.namespace}.{fixture.name}"
        spark.sql(f"DROP TABLE IF EXISTS {target}")
        spark.sql(f"CREATE TABLE {target} ({', '.join(f'{name} {kind}' for name, kind in fixture.columns)}) USING iceberg")
        values = ",".join("(" + ",".join(sql_literal(value, fixture.columns[i][1]) for i, value in enumerate(row)) + ")" for row in fixture.rows)
        spark.sql(f"INSERT INTO {target} VALUES {values}")
        rows = spark.sql(f"SELECT * FROM {target} ORDER BY 1").toJSON().collect()
        realized = [(field.name, field.dataType.simpleString(), field.nullable) for field in spark.table(target).schema.fields]
        expected_names = [name for name, _ in fixture.columns]
        if [name for name, _, _ in realized] != expected_names:
            raise RuntimeError(f"physical schema drift for {fixture.name}")
        schema_hash = "sha256:" + hashlib.sha256(json.dumps(realized, separators=(",", ":")).encode()).hexdigest()
        physical_bindings[fixture.name] = {"table": f"local.{args.namespace}.{fixture.name}", "schema-hash": schema_hash}
        snapshots[fixture.name] = spark.sql(f"SELECT snapshot_id FROM {target}.snapshots ORDER BY committed_at DESC LIMIT 1").first()[0]
        tables.append({"name": fixture.name, "columns": [name for name, _ in fixture.columns], "row-count": len(rows), "data-hash": "sha256:" + hashlib.sha256("\n".join(rows).encode()).hexdigest()})
    base = args.rest_uri.removesuffix("/catalog")
    def request(method: str, path: str, body: dict | None = None):
        encoded = None if body is None else json.dumps(body).encode()
        req = urllib.request.Request(base + path, data=encoded, method=method, headers={"content-type": "application/json", "x-lakecat-principal": "querygraph-tpcds-publisher"})
        with urllib.request.urlopen(req, timeout=30) as response: return json.load(response)
    policy = request("PUT", "/management/v1/warehouses/local/policies/tpcds-semantic", {"enforced": True, "odrl": {"uid": "policy:tpcds-semantic", "permission": [{"action": "read"}]}})
    publication = request("POST", "/management/v1/warehouses/local/models/tpcds_retail_model", {"version": 1, "expected-current-version": None, "artifact-uri": args.artifact_uri, "artifact-hash": args.artifact_hash, "physical-bindings": physical_bindings, "policy-binding-ids": ["tpcds-semantic"]})
    admitted = request("GET", "/management/v1/warehouses/local/models/tpcds_retail_model")["publications"]
    if admitted != [publication]: raise RuntimeError("model publication read-after-write drift")
    drain = request("POST", "/management/v1/lineage/drain")
    model_events = [event for event in drain["events"] if event["event-type"] == "model.published"]
    if len(model_events) != 1 or model_events[0]["graph-events"] < 1 or model_events[0]["lineage-events"] != 1:
        raise RuntimeError("model publication did not drain to graph and lineage")
    plans = {
        "customer-lifetime-value": f"SELECT SUM(ss_ext_sales_price) / COUNT(DISTINCT ss_customer_sk) AS value FROM lakecat.{args.namespace}.store_sales",
        "sales-by-brand": f"SELECT i.i_brand AS brand, SUM(s.ss_ext_sales_price) AS value FROM lakecat.{args.namespace}.store_sales s JOIN lakecat.{args.namespace}.item i ON s.ss_item_sk=i.i_item_sk GROUP BY i.i_brand ORDER BY i.i_brand",
        "store-productivity": f"SELECT SUM(s.ss_ext_sales_price) / SUM(st.s_number_employees) AS value FROM lakecat.{args.namespace}.store_sales s JOIN lakecat.{args.namespace}.store st ON s.ss_store_sk=st.s_store_sk",
        "total-profit": f"SELECT SUM(ss_net_profit) AS value FROM lakecat.{args.namespace}.store_sales",
        "total-sales": f"SELECT SUM(ss_ext_sales_price) AS value FROM lakecat.{args.namespace}.store_sales",
    }
    answers = {}
    for name, sql in plans.items():
        rows = [row.asDict(recursive=True) for row in spark.sql(sql).collect()]
        answers[name] = rows
    model_hash = "sha256:" + hashlib.sha256(model_bytes).hexdigest()
    answer_proof = _PROOF.build_answer_proof(model_hash=model_hash, artifact_hash=args.artifact_hash, policy=policy, snapshots=snapshots, plans=plans, answers=answers, drain=drain)
    _PROOF.verify_answer_proof(answer_proof)
    drift = _PROOF.drift_rejection_report(answer_proof)
    output = {"status": "verified", "model-input-hash": model_hash, "artifact-hash": args.artifact_hash, "policy-id": policy["policy-id"], "publication": publication, "tables": tables, "snapshots": snapshots, "lineage-drain": drain, "metric-plans": plans, "answer-proof": answer_proof, "drift-rejections": drift}
    args.output.write_text(json.dumps(output, sort_keys=True, separators=(",", ":")) + "\n")
    spark.stop()


if __name__ == "__main__": main()
