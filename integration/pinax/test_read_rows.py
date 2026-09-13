"""Opt-in integration test; uses the isolated Sail checkout selected by the host."""
import json
import os
from pathlib import Path
import subprocess
import sys


def test_real_sail_buffered_rows(tmp_path):
    report = tmp_path / "report.json"
    subprocess.run([
        sys.executable, str(Path(__file__).with_name("read_rows.py")),
        "--sail-root", os.environ["QG_SAIL_SOURCE"], "--report", str(report),
    ], check=True, timeout=1500)
    evidence = json.loads(report.read_text())
    assert evidence["real_iceberg_rows"]
    assert evidence["temporary_data_removed"]
    assert evidence["governed_mcp_execution"] == "not_tested"


def test_real_authenticated_owner_rows(tmp_path):
    report = tmp_path / "owner-report.json"
    binary = os.environ.get("QG_RUST_MCP_BIN")
    mcp_args = ["--querygraph-bin", binary] if binary else []
    subprocess.run([
        sys.executable, str(Path(__file__).with_name("execute_rows.py")),
        "--sail-root", os.environ["QG_SAIL_SOURCE"],
        "--lakecat-root", os.environ["QG_LAKECAT_SOURCE"], "--report", str(report),
        *mcp_args,
    ], check=True, timeout=1500)
    evidence = json.loads(report.read_text())
    assert evidence["real_authenticated_owner_rows"]
    assert evidence["row_and_evidence_digests_verified"]
    assert evidence["real_buffer_discarded_after_mutations"] == ["purpose_revoked", "registry_drift", "snapshot_drift"]
    assert evidence["temporary_data_removed"]
    assert evidence["mcp_execution"] == ("direct_and_bridged_passed" if binary else "not_tested")
    assert evidence["real_rows_after_reviewed_consumer_cutover"] == bool(binary)
