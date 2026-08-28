from __future__ import annotations

import hashlib
from dataclasses import dataclass
from typing import Any, Callable

from querygraph.osi import OsiArtifact


class SemanticPublicationError(ValueError):
    pass


@dataclass(frozen=True)
class SemanticPublicationRequest:
    content: bytes
    media_type: str
    expected_artifact_hash: str
    expected_current_version: int | None
    publication_version: int
    physical_bindings: dict[str, Any]


@dataclass(frozen=True)
class SemanticPublicationResult:
    artifact_hash: str
    publication: dict[str, Any]
    authorization_receipt: dict[str, Any]


def publish_semantic_model(
    request: SemanticPublicationRequest,
    *,
    schema: dict[str, Any],
    authorize: Callable[[OsiArtifact, str], dict[str, Any]],
    validate_physical: Callable[[OsiArtifact, dict[str, Any]], None],
    publish_catalog: Callable[[OsiArtifact, SemanticPublicationRequest, str, dict[str, Any]], dict[str, Any]],
    promote_graph: Callable[[dict[str, Any]], None],
    promote_lineage: Callable[[dict[str, Any]], None],
) -> SemanticPublicationResult:
    artifact_hash = f"sha256:{hashlib.sha256(request.content).hexdigest()}"
    if artifact_hash != request.expected_artifact_hash:
        raise SemanticPublicationError("model artifact hash drift")
    try:
        text = request.content.decode("utf-8")
        artifact = OsiArtifact.from_json(text) if request.media_type == "application/json" else OsiArtifact.from_yaml(text)
    except Exception as error:
        raise SemanticPublicationError("malformed model artifact") from error
    mapping = artifact.to_mapping()
    if mapping.get("version") != "0.2.0.dev0":
        raise SemanticPublicationError("unknown Ossie model version")
    errors = artifact.validate(schema)
    if errors:
        raise SemanticPublicationError(f"structural model validation failed: {errors[0]}")
    try:
        receipt = authorize(artifact, artifact_hash)
    except Exception as error:
        raise SemanticPublicationError("model publication unauthorized") from error
    if not receipt.get("allowed"):
        raise SemanticPublicationError("model publication unauthorized")
    try:
        validate_physical(artifact, request.physical_bindings)
    except Exception as error:
        raise SemanticPublicationError("physical binding validation failed") from error
    publication = publish_catalog(artifact, request, artifact_hash, receipt)
    if publication.get("version") != request.publication_version or publication.get("artifact-hash") != artifact_hash:
        raise SemanticPublicationError("catalog publication model/version drift")
    promote_graph(publication)
    promote_lineage(publication)
    return SemanticPublicationResult(artifact_hash, publication, receipt)
