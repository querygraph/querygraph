"""Real Iceberg row acceptance for the prepared Sail buffered-read helper."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

import pyarrow as pa
from pyiceberg.catalog.sql import SqlCatalog

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sail-root", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    sail = args.sail_root.resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="pinax-sail-rows-") as name:
        directory = Path(name).resolve()
        catalog = SqlCatalog("fixture", uri=f"sqlite:///{directory / 'catalog.db'}", warehouse=(directory / "warehouse").as_uri())
        catalog.create_namespace("acme")
        schema = pa.schema([pa.field("id", pa.int64(), nullable=False), pa.field("tenant", pa.string()), pa.field("private_email", pa.string())])
        table = catalog.create_table("acme.customers", schema=schema)
        table.append(pa.Table.from_pylist([
            {"id": 1, "tenant": "other", "private_email": "hidden@example.invalid"},
            {"id": 2, "tenant": "acme", "private_email": "hidden@example.invalid"},
            {"id": 3, "tenant": "acme", "private_email": "hidden@example.invalid"},
        ], schema=schema))
        snapshot = table.current_snapshot().snapshot_id
        request = {"location": table.location(), "metadata": table.metadata_location, "snapshot": snapshot,
                   "projection": ["id"], "tenant": "acme", "rows": 10, "bytes": 65536}
        manifest = directory / "Cargo.toml"
        manifest.write_text(f'''[package]
name = "pinax-sail-row-fixture"
version = "0.0.0"
edition = "2024"
publish = false
[[bin]]
name = "pinax-sail-row-fixture"
path = {json.dumps(str(ROOT / "integration/pinax/rows.rs"))}
[dependencies]
sail-iceberg = {{ path = {json.dumps(str(sail / "crates/sail-iceberg"))} }}
datafusion = {{ version = "54", default-features = false }}
tokio = {{ version = "1", features = ["macros", "rt-multi-thread", "time"] }}
serde_json = "1"
''')
        shutil.copyfile(sail / "Cargo.lock", directory / "Cargo.lock")
        target = Path("/tmp/sail-pinax-target")
        if os.environ.get("QG_FIXTURE_FETCH") == "1":
            subprocess.run(["cargo", "fetch", "--manifest-path", str(manifest)], check=True, timeout=1200)
        subprocess.run(["cargo", "build", "--offline", "--manifest-path", str(manifest), "--target-dir", str(target)],
                       check=True, timeout=int(os.environ.get("QG_FIXTURE_BUILD_TIMEOUT", "1200")))
        binary = target / "debug/pinax-sail-row-fixture"

        def read(changes=None, *, rejected=False):
            path = directory / "request.json"
            path.write_text(json.dumps(request | (changes or {})))
            result = subprocess.run([str(binary), str(path)], capture_output=True, text=True, timeout=45)
            assert (result.returncode != 0) == rejected, result.stderr
            value = json.loads(result.stdout)
            if rejected:
                assert value == {"error": "execution rejected"}
            return value

        assert sorted(read(), key=lambda row: row["id"]) == [{"id": 2}, {"id": 3}]
        assert read({"tenant": "no-match"}) == []
        assert read({"filter": {"type": "and", "left": {"type": "eq", "term": "tenant", "value": "acme"},
                                "right": {"type": "gt", "term": "id", "value": 2}}}) == [{"id": 3}]
        read({"filter": {"type": "eq", "term": "tenant", "value": "acme", "ignored": True}}, rejected=True)
        read({"filter": {"type": "unknown", "term": "tenant"}}, rejected=True)
        limited = read({"rows": 1})
        assert len(limited) == 1 and limited[0]["id"] in (2, 3)
        read({"bytes": 1}, rejected=True)
        read({"snapshot": -1}, rejected=True)
        read({"projection": []}, rejected=True)
        read({"projection": ["id", "id"]}, rejected=True)
        read({"projection": ["missing"]}, rejected=True)
        table.append(pa.Table.from_pylist([{"id": 4, "tenant": "acme", "private_email": "new@example.invalid"}], schema=schema))
        assert sorted(read(), key=lambda row: row["id"]) == [{"id": 2}, {"id": 3}]
        new_rows = read({"metadata": table.metadata_location, "snapshot": table.current_snapshot().snapshot_id})
        assert sorted(new_rows, key=lambda row: row["id"]) == [{"id": 2}, {"id": 3}, {"id": 4}]
        read({"metadata": (directory / "missing.metadata.json").as_uri()}, rejected=True)
        data_files = list((directory / "warehouse").rglob("*.parquet"))
        assert len(data_files) == 2
        data_files[-1].unlink()
        read({"metadata": table.metadata_location, "snapshot": table.current_snapshot().snapshot_id}, rejected=True)
        with binary.open("rb") as source:
            binary_hash = hashlib.file_digest(source, "sha256").hexdigest()
    report = {"real_iceberg_rows": True, "row_filter_before_projection_and_limit": True,
              "output_bytes_bounded": True, "explicit_snapshot_preserved": True,
              "invalid_projection_and_missing_backend_rejected": True,
              "missing_data_file_releases_no_partial_rows": True,
              "temporary_data_removed": not directory.exists(), "binary_sha256": binary_hash,
              "governed_mcp_execution": "not_tested"}
    args.report.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
