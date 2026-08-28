from __future__ import annotations

import hashlib
import json

import pytest

from querygraph.semantic_publication import SemanticPublicationError, SemanticPublicationRequest, publish_semantic_model


SCHEMA = {"type": "object", "required": ["version", "semantic_model"], "properties": {"version": {"const": "0.2.0.dev0"}, "semantic_model": {"type": "array", "minItems": 1}}}
VALID = json.dumps({"version": "0.2.0.dev0", "semantic_model": [{"name": "m", "datasets": []}]}).encode()


def request(content=VALID, *, expected_hash=None):
    return SemanticPublicationRequest(content, "application/json", expected_hash or f"sha256:{hashlib.sha256(content).hexdigest()}", None, 1, {"m": "table"})


def run(value, failure=None, *, catalog_drift=False, effects=None):
    effects = effects if effects is not None else []
    def authorize(*_):
        effects.append("authorize")
        if failure == "unauthorized": raise PermissionError()
        return {"allowed": True, "receipt-hash": "sha256:receipt"}
    def physical(*_):
        effects.append("physical")
        if failure in {"missing-physical", "schema-drift"}: raise ValueError()
    def catalog(_artifact, req, digest, _receipt):
        effects.append("catalog")
        return {"version": req.publication_version + int(catalog_drift), "artifact-hash": digest}
    def graph(_): effects.append("graph")
    def lineage(_): effects.append("lineage")
    result = publish_semantic_model(value, schema=SCHEMA, authorize=authorize, validate_physical=physical, publish_catalog=catalog, promote_graph=graph, promote_lineage=lineage)
    return result, effects


@pytest.mark.parametrize("case,value", [
    ("malformed", request(b"{")),
    ("model-drift", request(expected_hash="sha256:" + "0" * 64)),
    ("unknown-version", request(json.dumps({"version": "9", "semantic_model": [{"name": "m"}]}).encode())),
    ("structural", request(json.dumps({"version": "0.2.0.dev0", "semantic_model": []}).encode())),
])
def test_pre_admission_artifact_failures_have_no_effects(case, value):
    effects = []
    with pytest.raises(SemanticPublicationError):
        run(value, effects=effects)
    assert effects == []


@pytest.mark.parametrize("failure", ["unauthorized", "missing-physical", "schema-drift"])
def test_authorization_and_physical_failures_precede_catalog_graph_and_lineage(failure):
    effects = []
    with pytest.raises(SemanticPublicationError):
        run(request(), failure, effects=effects)
    assert "catalog" not in effects and "graph" not in effects and "lineage" not in effects


def test_catalog_version_drift_precedes_graph_and_lineage():
    effects = []
    with pytest.raises(SemanticPublicationError):
        run(request(), catalog_drift=True, effects=effects)
    assert "graph" not in effects and "lineage" not in effects


def test_success_promotes_only_after_catalog_publication():
    result, effects = run(request())
    assert result.publication["version"] == 1
    assert effects == ["authorize", "physical", "catalog", "graph", "lineage"]
