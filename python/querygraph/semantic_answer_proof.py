from __future__ import annotations

import copy
import hashlib
import json
from typing import Any


def canonical_hash(value: Any) -> str:
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    return "sha256:" + hashlib.sha256(encoded).hexdigest()


def build_answer_proof(*, model_hash: str, artifact_hash: str, policy: dict[str, Any], snapshots: dict[str, int], plans: dict[str, str], answers: dict[str, Any], drain: dict[str, Any]) -> dict[str, Any]:
    graph_basis = [{"event-id": event["event-id"], "graph-events": event["graph-events"]} for event in drain["events"]]
    lineage_basis = [{"event-id": event["event-id"], "event-hashes": event["replay-event-hashes"], "openlineage-hashes": event["replay-open-lineage-hashes"]} for event in drain["events"]]
    basis = {"artifact": artifact_hash, "graph": canonical_hash(graph_basis), "lineage": canonical_hash(lineage_basis), "model": model_hash, "physical": canonical_hash(snapshots), "plan": canonical_hash(plans), "policy": canonical_hash(policy)}
    return {"answers": answers, "answer-hash": canonical_hash(answers), "basis": basis, "proof-hash": canonical_hash({"answers": answers, "basis": basis})}


def verify_answer_proof(proof: dict[str, Any]) -> None:
    if proof.get("answer-hash") != canonical_hash(proof["answers"]):
        raise ValueError("semantic answer drift")
    if proof.get("proof-hash") != canonical_hash({"answers": proof["answers"], "basis": proof["basis"]}):
        raise ValueError("semantic proof basis drift")
    required = {"artifact", "graph", "lineage", "model", "physical", "plan", "policy"}
    if set(proof["basis"]) != required:
        raise ValueError("semantic proof basis is incomplete")
    if not all(isinstance(value, str) and value.startswith("sha256:") and len(value) == 71 for value in proof["basis"].values()):
        raise ValueError("semantic proof basis contains a malformed hash")


def drift_rejection_report(proof: dict[str, Any]) -> dict[str, str]:
    report: dict[str, str] = {}
    for dimension in ("physical", "model", "policy", "graph", "lineage", "artifact"):
        candidate = copy.deepcopy(proof)
        candidate["basis"][dimension] = canonical_hash({"deliberate-drift": dimension})
        try:
            verify_answer_proof(candidate)
        except ValueError:
            report[dimension] = "rejected"
        else:
            raise AssertionError(f"{dimension} drift was accepted")
    return report
