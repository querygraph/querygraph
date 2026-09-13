"""Exercise registry deployment against isolated, authenticated LakeCat and Sail.

Run with the shared project environment. Local sibling dependencies are generated
only in a temporary fixture manifest; no consumer manifest gains a path dependency.
This verifies catalog deployment, not governed row execution.
"""

from __future__ import annotations

import argparse
import copy
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import select
import shutil
import subprocess
import tempfile
import tomllib
from urllib.error import HTTPError
from urllib.parse import urlparse, unquote
from urllib.request import Request, urlopen


ROOT = Path(__file__).resolve().parents[2]


def run(command, *, expected=0):
    result = subprocess.run(command, capture_output=True, text=True, timeout=120)
    if expected == 0 and result.returncode:
        raise RuntimeError(f"command failed: {result.stderr}")
    if expected != 0 and result.returncode == 0:
        raise AssertionError("command unexpectedly succeeded")
    return json.loads(result.stdout) if result.stdout.strip() else None


def registry():
    return {"version": "pinax.v1", "enterprise": "acme", "tables": [{
        "name": "customers", "revision": 1, "profile": "custom",
        "metadata": {"owner": "platform", "steward": "data", "description": "Customer identifiers", "retention_days": 30},
        "security": {"policy": "enterprise", "revision": 1, "classification": "internal", "purposes": ["analytics"]},
        "columns": [{"id": 1, "name": "customer_id", "data_type": {"kind": "string"}, "nullability": "required",
                     "semantic": "customer.id", "description": "Customer identifier", "classification": "internal"}],
        "primary_key": [1],
    }]}


def write_json(path, value):
    path.write_text(json.dumps(value))


def initialize_message():
    return {"jsonrpc":"2.0","id":0,"method":"initialize","params":{
        "protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"cutover-fixture","version":"1"}}}


def rejects_consumer_start(binary, config):
    result = subprocess.run([str(binary), "mcp-serve", "--registry-config", str(config)],
                            input=json.dumps(initialize_message()) + "\n", capture_output=True, text=True, timeout=40)
    assert result.returncode != 0 and result.stdout == "", "unready consumer accepted MCP initialization"
    assert "consumer" in result.stderr and ("activation" in result.stderr or "reviewed" in result.stderr)


@contextmanager
def consumer_registry(binary, config, subject):
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
    from querygraph.crypto import Ed25519Signer
    from querygraph.typedid import TypeDidEnvelope

    process = subprocess.Popen([str(binary), "mcp-serve", "--registry-config", str(config)],
                               stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
    try:
        def send(message):
            process.stdin.write(json.dumps(message) + "\n")
            process.stdin.flush()
        def receive():
            assert select.select([process.stdout], [], [], 35)[0], "consumer response timed out"
            return json.loads(process.stdout.readline(1024 * 1024))
        send(initialize_message())
        assert receive()["id"] == 0
        send({"jsonrpc":"2.0","method":"notifications/initialized"})
        key = hashlib.sha256(b"typesec-ed25519-signing\0" + bytes([7]) * 32).digest()
        signer = Ed25519Signer(Ed25519PrivateKey.from_private_bytes(key))
        assert signer.did_key() == subject
        intent = json.dumps({"purpose":"analytics","limit":100})
        envelope = TypeDidEnvelope.create(sender=subject, recipient="did:example:registry", action="invoke",
            resource="/mcp/discover_pinax_tables", payload={"bodySha256":hashlib.sha256(intent.encode()).hexdigest()}, signer=signer)
        send({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"discover_pinax_tables",
             "arguments":{"intent":intent,"envelope":envelope.model_dump(mode="json")}}})
        response = receive()
        assert response["result"]["isError"] is False
        yield response["result"]["structuredContent"]
    finally:
        if process.stdin and not process.stdin.closed:
            process.stdin.close()
        try:
            assert process.wait(timeout=10) == 0
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=10)
            raise


def build_fixture(directory, lakecat, sail=None):
    manifest = directory / "Cargo.toml"
    dependencies = "\n".join(
        f'{crate} = {{ path = {json.dumps(str(lakecat / "crates" / crate))}'
        + (', features = ["sail-local", "typesec-local"]' if crate == "lakecat-service" else "") + " }"
        for crate in ["lakecat-core", "lakecat-store", "lakecat-security", "lakecat-service", "lakecat-graph", "lakecat-lineage"]
    )
    manifest.write_text(f'''[package]
name = "querygraph-registry-live-fixture"
version = "0.0.0"
edition = "2024"
publish = false
[[bin]]
name = "querygraph-registry-live-fixture"
path = {json.dumps(str(ROOT / "integration/pinax/server.rs"))}
[dependencies]
{dependencies}
typesec = {{ version = "=0.14.0", default-features = false, features = ["integrations", "rbac"] }}
bs58 = "0.5"
uuid = {{ version = "1", features = ["v4"] }}
tokio = {{ version = "1", features = ["macros", "rt-multi-thread", "net"] }}
axum = "0.8"
async-trait = "0.1"
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
url = "2"
''')
    if sail is not None:
        lock = tomllib.loads((lakecat / "Cargo.lock").read_text())
        patches = [f'{package["name"]} = {{ path = {json.dumps(str(sail / "crates" / package["name"]))} }}'
                   for package in lock["package"] if "github.com/querygraph/sail" in package.get("source", "")]
        with manifest.open("a") as output:
            output.write('\n[patch."https://github.com/querygraph/sail.git"]\n' + "\n".join(patches) + "\n")
    shutil.copyfile(lakecat / "Cargo.lock", directory / "Cargo.lock")
    # Reuse the owner's build cache, while preserving its source/lockfile.
    target = lakecat / "target"
    if os.environ.get("QG_FIXTURE_FETCH") == "1":
        subprocess.run(["cargo", "fetch", "--manifest-path", str(manifest)], check=True, timeout=1200)
    subprocess.run(["cargo", "build", "--offline", "--manifest-path", str(manifest), "--target-dir", str(target)],
                   check=True, timeout=int(os.environ.get("QG_FIXTURE_BUILD_TIMEOUT", "1200")))
    return target / "debug/querygraph-registry-live-fixture"


def exercise(directory, binary, ready, fixture):

    def http(path, body=None, *, extra=None, expected=200):
        data = None if body is None else json.dumps(body).encode()
        credential = run([str(fixture), str(directory), "credential", "GET" if data is None else "POST", path,
                          "sha256:" + hashlib.sha256(data or b"").hexdigest()])
        headers = {"Content-Type": "application/json", "x-lakecat-agent-did": ready["subject"],
                   "x-lakecat-typedid-envelope": json.dumps(credential, separators=(",", ":"))}
        request = Request(ready["origin"].rstrip("/") + path,
                          data=data,
                          headers=headers | (extra or {}))
        try:
            response = urlopen(request, timeout=30)
        except HTTPError as error:
            response = error
        with response:
            value = json.loads(response.read(8 * 1024 * 1024 + 1))
            assert response.status == expected, f"unexpected catalog status {response.status}"
            return value, response.headers

    previous = registry()
    target = copy.deepcopy(previous)
    target["tables"][0]["revision"] = 2
    target["tables"][0]["columns"].append({"id": 2, "name": "note", "data_type": {"kind": "string"},
        "nullability": "optional", "semantic": "customer.note", "description": "Optional note", "classification": "internal"})
    extra = copy.deepcopy(previous["tables"][0])
    extra["name"] = "transactions"
    target["tables"].append(extra)
    current_path, target_path = directory / "current.json", directory / "target.json"
    write_json(current_path, previous)
    write_json(target_path, target)
    create = run([str(binary), "pinax", "catalog-plan", "--registry", str(current_path), "--warehouse", "warehouse"])[0]
    http(create["path"], create["body"])
    path = "/catalog/v1/warehouse/namespaces/acme/tables/customers"
    before, before_headers = http(path)
    token = before_headers["x-lakecat-table-state"]
    plan = run([str(binary), "registry-deployment", "plan", "--current", str(current_path), "--target", str(target_path), "--warehouse", "warehouse"])
    config = directory / "config.json"
    write_json(config, {"registry": "current.json", "warehouse": "warehouse", "policy": "policy.yaml", "policy_id": "enterprise",
        "policy_revision": 1, "server_did": "did:example:registry", "lakecat_origin": ready["origin"],
        "lakecat_subject": ready["subject"], "lakecat_identity": "identity.json"})
    resources = [f"pinax/acme/{name}/revisions/{revision}{suffix}" for name in ("customers", "transactions")
                 for revision in (1, 2) for suffix in ("", "/columns/1", "/columns/2")]
    (directory / "policy.yaml").write_text("roles:\n  - name: analyst\n    permissions: [read]\n    resources: " + json.dumps(resources)
        + "\nassignments:\n  - subject: " + json.dumps(ready["subject"]) + "\n    roles: [analyst]\n")
    base_config = json.loads(config.read_text())
    old_consumer, new_consumer = directory / "consumer-old.json", directory / "consumer-target.json"
    write_json(old_consumer, base_config | {"activation":{"expected_registry_digest":plan["previous_digest"]}})
    write_json(new_consumer, base_config | {"registry":"target.json", "activation":{"expected_registry_digest":plan["target_digest"]}})
    wrong_consumer = directory / "consumer-unreviewed.json"
    write_json(wrong_consumer, base_config | {"activation":{"expected_registry_digest":"unreviewed"}})
    rejects_consumer_start(binary, wrong_consumer)
    rejects_consumer_start(binary, new_consumer)
    with consumer_registry(binary, old_consumer, ready["subject"]) as page:
        assert [(item["name"], item["revision"]) for item in page["tables"]] == [("customers", 1)]
    # The old consumer has exited before any catalog mutation below.
    apply = [str(binary), "registry-deployment", "apply", "--registry-config", str(config), "--target", str(target_path), "--expected-target-digest"]
    run(apply + ["unreviewed"], expected=1)
    unchanged, _ = http(path)
    assert unchanged["metadata"] == before["metadata"]
    applied = run(apply + [plan["target_digest"]])
    assert applied["status"] == "ready" and len(applied["tables"]) == 2
    rejects_consumer_start(binary, old_consumer)
    with consumer_registry(binary, new_consumer, ready["subject"]) as page:
        assert [(item["name"], item["revision"]) for item in page["tables"]] == [("customers", 2), ("transactions", 1)]
        assert page["registry_digest"] == plan["target_digest"]
    after, _ = http(path)
    assert after["metadata"]["table-uuid"] == before["metadata"]["table-uuid"]
    assert after["metadata"]["current-schema-id"] != before["metadata"]["current-schema-id"]
    current_schema = next(schema for schema in after["metadata"]["schemas"] if schema["schema-id"] == after["metadata"]["current-schema-id"])
    assert [(field["id"], field["name"], field["required"]) for field in current_schema["fields"]] == [(1, "customer_id", True), (2, "note", False)]
    transactions, _ = http(path.replace("customers", "transactions"))
    for loaded in [after, transactions]:
        assert loaded["metadata"]["properties"]["pinax.registry-digest"] == plan["target_digest"]
        metadata_file = Path(unquote(urlparse(loaded["metadata-location"]).path))
        assert metadata_file.is_relative_to(directory)
        physical = json.loads(metadata_file.read_text())
        # CatalogStore adds these two bookkeeping fields after Sail persists
        # the Iceberg file. Compare every actual Iceberg metadata field/pin.
        annotations = {"lakecat:version", "lakecat:last-request-hash"}
        physical_iceberg = {key: value for key, value in physical.items() if key not in annotations}
        catalog_iceberg = {key: value for key, value in loaded["metadata"].items() if key not in annotations}
        assert physical_iceberg == catalog_iceberg
    assert run(apply + [plan["target_digest"]])["status"] == "ready"
    repeated, _ = http(path)
    assert repeated["metadata-location"] == after["metadata-location"]
    http(path, {"requirements": [], "updates": []}, extra={"x-lakecat-expected-table-state": token}, expected=409)
    current, current_headers = http(path)
    assert current["metadata"] == after["metadata"]
    http(path, {"requirements": [], "updates": [{"action": "set-properties", "updates": {"pinax.registry-digest": "unreviewed-drift"}}]},
         extra={"x-lakecat-expected-table-state": current_headers["x-lakecat-table-state"]})
    reconcile = run([str(binary), "registry-deployment", "reconcile", "--registry-config", str(config), "--target", str(target_path)], expected=1)
    assert reconcile["status"] == "blocked"
    rejects_consumer_start(binary, new_consumer)
    run(apply + [plan["target_digest"]], expected=1)
    final_transactions, _ = http(path.replace("customers", "transactions"))
    assert final_transactions["metadata-location"] == transactions["metadata-location"]
    return {"review_digest_enforced": True, "real_sail_schema_update": True, "all_registry_pins_updated": True,
            "physical_metadata_readback": True, "completed_updates_not_repeated": True,
            "stale_owner_token_rejected": True, "registry_drift_blocks_deployment": True,
            "reviewed_consumer_stop_apply_restart": True, "unready_or_stale_consumer_rejected": True,
            "target_digest": plan["target_digest"], "governed_row_execution": "not_tested"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lakecat-root", type=Path, required=True)
    parser.add_argument("--querygraph-bin", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--sail-root", type=Path)
    args = parser.parse_args()
    lakecat, binary = args.lakecat_root.resolve(strict=True), args.querygraph_bin.resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="querygraph-registry-live-") as name:
        directory = Path(name).resolve()
        fixture = build_fixture(directory, lakecat, args.sail_root.resolve(strict=True) if args.sail_root else None)
        process = subprocess.Popen([str(fixture), str(directory)], stdout=subprocess.PIPE, text=True)
        try:
            if not select.select([process.stdout], [], [], 60)[0]:
                raise TimeoutError("LakeCat fixture did not become ready")
            ready = json.loads(process.stdout.readline(65536))
            report = exercise(directory, binary, ready, fixture)
        finally:
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=10)
    report["fixture_process_stopped"] = True
    report["fixture_directory_removed"] = not directory.exists()
    report["lakecat_commit"] = subprocess.check_output(["git", "-C", str(lakecat), "rev-parse", "HEAD"], text=True).strip()
    with binary.open("rb") as source:
        report["querygraph_binary_sha256"] = hashlib.file_digest(source, "sha256").hexdigest()
    args.report.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
