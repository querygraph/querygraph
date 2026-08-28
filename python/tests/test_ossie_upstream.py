from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path


ROOT = Path(__file__).parents[2]


def _module():
    path = ROOT / "scripts/fetch-ossie.py"
    spec = importlib.util.spec_from_file_location("fetch_ossie", path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_manifest_is_commit_and_sha256_bound():
    manifest = json.loads((ROOT / "ossie/upstream.json").read_text())
    assert len(manifest["revision"]) == 40
    assert set(manifest["artifacts"]) == {
        "core-spec/osi-schema.json",
        "validation/validate.py",
        "examples/tpcds_semantic_model.yaml",
    }
    assert all(len(value) == 64 for value in manifest["artifacts"].values())


def test_verify_rejects_artifact_drift(tmp_path):
    artifact = tmp_path / "schema.json"
    artifact.write_text("accepted")
    manifest = tmp_path / "manifest.json"
    manifest.write_text(json.dumps({"artifacts": {"schema.json": hashlib.sha256(b"accepted").hexdigest()}}))
    module = _module()
    module.verify(manifest, tmp_path)
    artifact.write_text("drift")
    try:
        module.verify(manifest, tmp_path)
    except RuntimeError as error:
        assert "artifact drift" in str(error)
    else:
        raise AssertionError("drift was accepted")
