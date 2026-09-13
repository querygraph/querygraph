"""Real authenticated owner execution acceptance, optionally through both MCP transports."""
import argparse
import asyncio
import copy
from contextlib import nullcontext
import hashlib
import json
import os
from pathlib import Path
import select
import secrets
import subprocess
import sys
import tempfile
from urllib.error import HTTPError
from urllib.request import Request, urlopen

import pyarrow as pa
from pyiceberg.catalog.sql import SqlCatalog
from run import build_fixture, run, write_json, registry


async def check_mcp(directory, binary, ready, allowed):
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
    from mcp import ClientSession, StdioServerParameters
    from mcp.client.stdio import stdio_client
    from querygraph.crypto import Ed25519Signer
    from querygraph.typedid import TypeDidEnvelope

    # Fixed fixture key, derived exactly as TypeSec does; never production material.
    key = hashlib.sha256(b"typesec-ed25519-signing\0" + bytes([7]) * 32).digest()
    signer = Ed25519Signer(Ed25519PrivateKey.from_private_bytes(key))
    assert signer.did_key() == ready["subject"]
    config = directory / "registry-service.json"
    revision = json.loads((directory / "registry.json").read_text())["tables"][0]["revision"]
    digest = run([str(binary), "pinax", "catalog-plan", "--registry", str(directory / "registry.json"), "--warehouse", "warehouse"])[0]["body"]["properties"]["pinax.registry-digest"]
    write_json(config, {"registry":"registry.json","warehouse":"warehouse","policy":"policy.yaml", "policy_id":"enterprise","policy_revision":1,
                       "server_did":"did:example:registry","lakecat_origin":ready["origin"],"lakecat_subject":ready["subject"],"lakecat_identity":"identity.json",
                       "activation":{"expected_registry_digest":digest}})
    (directory / "policy.yaml").write_text(f'''roles:
  - name: analyst
    permissions: [read]
    resources: [pinax/acme/rows/revisions/{revision}, pinax/acme/rows/revisions/{revision}/columns/1]
assignments:
  - subject: "{ready['subject']}"
    roles: [analyst]
''')
    def arguments(intent, resource="/mcp/execute_pinax_scan"):
        text = json.dumps(intent)
        envelope = TypeDidEnvelope.create(sender=signer.did_key(), recipient="did:example:registry", action="invoke", resource=resource,
                                         payload={"bodySha256":hashlib.sha256(text.encode()).hexdigest()}, signer=signer)
        return {"intent":text,"envelope":envelope.model_dump(mode="json")}

    sessions = [StdioServerParameters(command=str(binary), args=["mcp-serve","--registry-config",str(config)]),
                StdioServerParameters(command=sys.executable, args=["-m","querygraph","mcp-serve","--rust-backend",str(binary),"--registry-config",str(config)])]
    for transport, parameters in zip(("direct", "bridge"), sessions):
        print(f"MCP fixture: {transport}, execution allowed={allowed}", flush=True)
        async with stdio_client(parameters) as (read, write):
            async with ClientSession(read, write) as session:
                await session.initialize()
                assert "execute_pinax_scan" in [tool.name for tool in (await session.list_tools()).tools]
                intent = {"table":"rows","columns":["id"],"purpose":"analytics","limit":10}
                result = await session.call_tool("execute_pinax_scan", arguments(intent))
                assert result.isError == (not allowed), result
                if allowed:
                    assert result.structuredContent["rows"] == [{"id":2}]
                    assert result.structuredContent["summary"]["columns"] == ["id"]
                    assert result.structuredContent["summary"]["revision"] == revision
                    assert "private_email" not in json.dumps(result.model_dump(mode="json"))
                for rejected in [arguments(intent | {"columns":["private_email"]}), arguments(intent | {"purpose":"marketing"}),
                                 arguments(intent, "/mcp/plan_pinax_scan")]:
                    failure = await session.call_tool("execute_pinax_scan", rejected)
                    assert failure.isError is True
                    assert failure.structuredContent is None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sail-root", type=Path, required=True)
    parser.add_argument("--lakecat-root", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--querygraph-bin", type=Path)
    parser.add_argument("--prepare-demo", type=Path,
                        help="Create a new retained demo directory, verify its MCP reads, and stop the owner")
    args = parser.parse_args()
    if args.prepare_demo:
        if not args.querygraph_bin or not 1 <= int(os.environ.get("QG_FIXTURE_PORT", "0")) <= 65535:
            parser.error("--prepare-demo requires --querygraph-bin and a nonzero QG_FIXTURE_PORT")
        args.prepare_demo = args.prepare_demo.resolve()
        args.prepare_demo.mkdir(parents=True, exist_ok=False)
    context = nullcontext(args.prepare_demo) if args.prepare_demo else tempfile.TemporaryDirectory(prefix="pinax-owner-rows-")
    with context as name:
        directory = Path(name).resolve()
        catalog = SqlCatalog("fixture", uri=f"sqlite:///{directory / 'catalog.db'}", warehouse=(directory / "warehouse").as_uri())
        catalog.create_namespace("acme")
        schema = pa.schema([pa.field("id", pa.int64(), nullable=False), pa.field("tenant", pa.string()), pa.field("private_email", pa.string())])
        properties = {}
        if args.querygraph_bin:
            args.querygraph_bin = args.querygraph_bin.resolve(strict=True)
            document = registry()
            contract = document["tables"][0]
            contract["name"] = "rows"
            contract["columns"] = [{"id":number,"name":field,"data_type":{"kind":"int64" if number == 1 else "string"},
                                    "nullability":"required" if number == 1 else "optional", "semantic":f"row.{field}","description":field,"classification":"internal"}
                                   for number, field in [(1,"id"),(2,"tenant"),(3,"private_email")]]
            registry_path = directory / "registry.json"
            write_json(registry_path, document)
            properties = run([str(args.querygraph_bin),"pinax","catalog-plan","--registry",str(registry_path),"--warehouse","warehouse"])[0]["body"]["properties"]
        table = catalog.create_table("acme.rows", schema=schema, properties=properties)
        table.append(pa.Table.from_pylist([{"id": 1, "tenant": "other", "private_email": "hidden"}, {"id": 2, "tenant": "acme", "private_email": "hidden"}], schema=schema))
        seed = {"location": table.location(), "metadata_location": table.metadata_location, "metadata": table.metadata.model_dump(mode="json", by_alias=True)}
        snapshot = table.current_snapshot().snapshot_id
        table.append(pa.Table.from_pylist([{"id": 3, "tenant": "acme", "private_email": "hidden"}], schema=schema))
        seed |= {"successor_metadata": table.metadata.model_dump(mode="json", by_alias=True), "successor_location": table.metadata_location}
        binary = build_fixture(directory, args.lakecat_root.resolve(strict=True), args.sail_root.resolve(strict=True))
        if args.prepare_demo:
            write_json(directory / "seed.json", seed | {"execute_allowed": True})
            process = subprocess.Popen([str(binary), str(directory)], stdout=subprocess.PIPE, text=True,
                                       env=os.environ | {"LAKECAT_PLAN_TASK_SIGNING_KEY": secrets.token_hex(32)})
            try:
                assert select.select([process.stdout], [], [], 60)[0], "owner did not become ready"
                ready = json.loads(process.stdout.readline())
                asyncio.run(check_mcp(directory, args.querygraph_bin, ready, True))
                report = {"prepared_demo": str(directory), "registry_config": str(directory / "registry-service.json"),
                          "owner_binary": str(binary), "owner": ready, "real_mcp_reads_verified": True,
                          "catalog_restart_behavior": "reloads the original seed contract and snapshot"}
                write_json(args.report, report)
                write_json(directory / "ready.json", report)
            finally:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=10)
            print(json.dumps(report, indent=2))
            return
        table_path = "/catalog/v1/warehouse/namespaces/acme/tables/rows"
        execution_path = "/querygraph/v1/warehouse/namespaces/acme/tables/rows/execute"
        body = {"projection": ["id"], "purpose": "analytics", "limit": 10, "snapshot_id": snapshot}
        evidence = None
        mutations_verified = []
        scenarios = [(False, None), (True, "purpose_revoked"), (True, "registry_drift"), (True, "snapshot_drift"), (True, None)]
        for allowed, mutation in scenarios:
            marker = directory / "read-mutation-count"
            marker.unlink(missing_ok=True)
            write_json(directory / "seed.json", seed | {"execute_allowed": allowed} | ({"read_mutation": mutation} if mutation else {}))
            process = subprocess.Popen([str(binary), str(directory)], stdout=subprocess.PIPE, text=True,
                                       env=os.environ | {"LAKECAT_PLAN_TASK_SIGNING_KEY": secrets.token_hex(32)})
            try:
                assert select.select([process.stdout], [], [], 60)[0], "owner did not become ready"
                ready = json.loads(process.stdout.readline())

                def http(path, value=None, *, token=None, status=200):
                    data = None if value is None else json.dumps(value).encode()
                    credential = run([str(binary), str(directory), "credential", "GET" if data is None else "POST", path, "sha256:" + hashlib.sha256(data or b"").hexdigest()])
                    headers = {"Content-Type": "application/json", "x-lakecat-agent-did": ready["subject"], "x-lakecat-typedid-envelope": json.dumps(credential, separators=(",", ":"))}
                    if token is not None:
                        headers["x-lakecat-expected-table-state"] = token
                    try:
                        response = urlopen(Request(ready["origin"].rstrip("/") + path, data=data, headers=headers), timeout=40)
                    except HTTPError as error:
                        response = error
                    with response:
                        value = json.loads(response.read(8 * 1024 * 1024 + 1))
                        assert response.status == status, f"owner status {response.status}, expected {status}: {value}"
                        if status != 200:
                            assert "rows" not in value
                        return value, response.headers

                loaded, headers = http(table_path)
                token = headers["x-lakecat-table-state"]
                planned, _ = http(table_path + "/plan", {"projection": ["id"], "limit": 10, "snapshot-id": body["snapshot_id"]})
                assert planned["governed-scan-proof"]
                if mutation:
                    http(execution_path, body, token=token, status=409)
                    assert marker.read_text() == "1", "real Sail rows were not buffered before mutation"
                    if mutation == "snapshot_drift":
                        changed, _ = http(table_path)
                        assert changed["metadata"]["current-snapshot-id"] == table.current_snapshot().snapshot_id
                    mutations_verified.append(mutation)
                    continue
                if args.querygraph_bin:
                    asyncio.run(check_mcp(directory, args.querygraph_bin, ready, allowed))
                if not allowed:
                    http(execution_path, body, token=token, status=403)
                    continue
                result, _ = http(execution_path, body, token=token)
                assert result["columns"] == ["id"] and result["rows"] == [{"id": 2}]
                evidence = result["evidence"]
                def digest(value):
                    return "sha256:" + hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()).hexdigest()
                assert evidence["row_digest"] == digest(["lakecat.executed-rows.v1", result["columns"], result["rows"]])
                assert result["evidence_digest"] == digest(["lakecat.executed-result-evidence.v1", evidence])
                assert evidence["proof"]["purpose"] == "analytics"
                assert evidence["row_digest"].startswith("sha256:")
                assert evidence["lineage"]["event_hash"].startswith("sha256:")
                http(execution_path, body | {"projection": ["private_email"]}, token=token, status=403)
                http(execution_path, body | {"purpose": "marketing"}, token=token, status=403)
                http(execution_path, body, token="sha256:" + "0" * 64, status=409)
                http(execution_path, body, status=400)
                if args.querygraph_bin:
                    # Both old MCP sessions have closed before the reviewed write.
                    current_registry = directory / "registry.json"
                    successor = copy.deepcopy(json.loads(current_registry.read_text()))
                    successor["tables"][0]["revision"] = 2
                    successor["tables"][0]["columns"].append({"id":4,"name":"note","data_type":{"kind":"string"},
                        "nullability":"optional","semantic":"row.note","description":"Optional note","classification":"internal"})
                    next_registry = directory / "registry-next.json"
                    write_json(next_registry, successor)
                    review = run([str(args.querygraph_bin),"registry-deployment","plan","--current",str(current_registry),
                                  "--target",str(next_registry),"--warehouse","warehouse"])
                    applied = run([str(args.querygraph_bin),"registry-deployment","apply","--registry-config",str(directory / "registry-service.json"),
                                   "--target",str(next_registry),"--expected-target-digest",review["target_digest"]])
                    assert applied["status"] == "ready"
                    write_json(current_registry, successor)
                    asyncio.run(check_mcp(directory, args.querygraph_bin, ready, True))
                    _, refreshed_headers = http(table_path)
                    token = refreshed_headers["x-lakecat-table-state"]
                for path in (directory / "warehouse").rglob("*.parquet"):
                    path.unlink()
                http(execution_path, body, token=token, status=500)
            finally:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=10)
    report = {"real_authenticated_owner_rows": True, "mandatory_filter_applied": True,
              "planning_permission_does_not_authorize_execution": True,
              "denied_projection_purpose_and_stale_state": True, "missing_data_releases_no_rows": True,
              "lineage_and_authorization_evidence": evidence,
              "row_and_evidence_digests_verified": True,
              "real_buffer_discarded_after_mutations": mutations_verified,
              "real_rows_after_reviewed_consumer_cutover": bool(args.querygraph_bin),
              "temporary_data_removed": not directory.exists(), "mcp_execution": "direct_and_bridged_passed" if args.querygraph_bin else "not_tested"}
    args.report.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({key: value for key, value in report.items() if key != "lineage_and_authorization_evidence"}, indent=2))


if __name__ == "__main__":
    main()
