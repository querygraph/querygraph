import copy

import pytest

from querygraph.semantic_answer_proof import (
    build_answer_proof,
    canonical_hash,
    drift_rejection_report,
    verify_answer_proof,
)


def proof():
    drain = {"events": [{"event-id": "event-1", "graph-events": 2, "replay-event-hashes": [canonical_hash("event")], "replay-open-lineage-hashes": [canonical_hash("openlineage")]}]}
    return build_answer_proof(model_hash=canonical_hash("model"), artifact_hash=canonical_hash("artifact"), policy={"id": "read"}, snapshots={"store_sales": 42}, plans={"total-sales": "SELECT SUM(sales)"}, answers={"total-sales": 105}, drain=drain)


def test_answer_proof_verifies_and_rejects_all_required_drift_dimensions():
    candidate = proof()
    verify_answer_proof(candidate)
    assert drift_rejection_report(candidate) == {name: "rejected" for name in ("physical", "model", "policy", "graph", "lineage", "artifact")}


@pytest.mark.parametrize("field", ["answers", "basis", "proof-hash"])
def test_answer_proof_rejects_tampering(field):
    candidate = copy.deepcopy(proof())
    if field == "answers":
        candidate[field]["total-sales"] = 106
    elif field == "basis":
        candidate[field]["plan"] = canonical_hash("other-plan")
    else:
        candidate[field] = canonical_hash("forged-proof")
    with pytest.raises(ValueError):
        verify_answer_proof(candidate)
