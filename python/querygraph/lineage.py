from __future__ import annotations

import json
import uuid
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from pydantic import BaseModel, Field

from querygraph import crypto
from querygraph.typedid import TypeDidEnvelope, sha256_hex

# Deterministic, OpenLineage-conformant run ids: the spec requires
# `run.runId` to be a UUID, so seeds (envelope signatures, bundle hashes)
# are mapped through UUIDv5 under a QueryGraph namespace. qg-rust derives
# the identical namespace and ids.
QG_RUN_NAMESPACE = uuid.uuid5(uuid.NAMESPACE_URL, "https://querygraph.ai/openlineage")


def run_id_for(seed: str) -> str:
    return str(uuid.uuid5(QG_RUN_NAMESPACE, seed))


class OpenLineageRunEvent(BaseModel):
    eventType: str = "COMPLETE"
    eventTime: datetime = Field(default_factory=lambda: datetime.now(UTC))
    run: dict[str, Any]
    job: dict[str, Any]
    inputs: list[dict[str, Any]] = Field(default_factory=list)
    outputs: list[dict[str, Any]] = Field(default_factory=list)
    producer: str = "https://querygraph.ai/qg-python"
    schemaURL: str = "https://openlineage.io/spec/2-0-2/OpenLineage.json"

    @classmethod
    def for_agent_run(
        cls,
        *,
        request: TypeDidEnvelope,
        job_name: str,
        inputs: list[str],
        outputs: list[str],
        namespace: str = "querygraph.python",
    ) -> "OpenLineageRunEvent":
        return cls(
            run={
                "runId": run_id_for(request.signature),
                "facets": {
                    "queryGraph_typeDid": {
                        "_producer": "https://querygraph.ai/qg-python",
                        "_schemaURL": "https://querygraph.ai/schemas/openlineage/querygraph-typedid-facet/0.1.0.json",
                        "protocol": request.protocol,
                        "conversationId": request.conversation_id,
                        "payloadSha256": request.payload_sha256,
                        "signature": request.signature,
                    }
                },
            },
            job={"namespace": namespace, "name": job_name},
            inputs=[{"namespace": "sail", "name": item} for item in inputs],
            outputs=[{"namespace": "querygraph", "name": item} for item in outputs],
        )

    def event_hash(self) -> str:
        return sha256_hex(self.model_dump_json(exclude_none=True))


class LineageAttestation(BaseModel):
    issuer: str
    subject: str
    event_hash: str
    merkle_root: str
    signature_type: str = "QueryGraphUnsignedDigest"
    verification_method: str
    signature: str
    signed_payload_sha256: str
    created_at: datetime = Field(default_factory=lambda: datetime.now(UTC))

    @classmethod
    def from_event(
        cls,
        *,
        issuer: str,
        subject: str,
        event_hash: str,
        signer: "crypto.Ed25519Signer | None" = None,
    ) -> "LineageAttestation":
        created_at = datetime.now(UTC)
        merkle_root = sha256_hex(f"querygraph-lineage\n{event_hash}")
        payload = attestation_payload_v1(
            issuer=issuer,
            subject=subject,
            event_hash=event_hash,
            merkle_root=merkle_root,
            created_at=created_at,
        )
        if signer is not None:
            signature = signer.sign(payload)
            signature_type = "QueryGraphEd25519Signature"
            verification_method = signer.verification_method()
        else:
            signature = crypto.unsigned_digest(payload)
            signature_type = "QueryGraphUnsignedDigest"
            verification_method = f"{issuer}#unsigned-digest"
        return cls(
            issuer=issuer,
            subject=subject,
            event_hash=event_hash,
            merkle_root=merkle_root,
            signature_type=signature_type,
            verification_method=verification_method,
            signature=signature,
            signed_payload_sha256=sha256_hex(payload),
            created_at=created_at,
        )

    def signing_payload(self) -> str:
        return attestation_payload_v1(
            issuer=self.issuer,
            subject=self.subject,
            event_hash=self.event_hash,
            merkle_root=self.merkle_root,
            created_at=self.created_at,
        )

    def verify(self, public_key: bytes | str | None = None) -> bool:
        """Verify an Ed25519-signed attestation; unsigned digests never verify."""
        if not self.signature.startswith(crypto.SIGNATURE_PREFIX):
            return False
        key = public_key or self.verification_method
        return crypto.verify(key, self.signing_payload(), self.signature)


def attestation_payload_v1(
    *,
    issuer: str,
    subject: str,
    event_hash: str,
    merkle_root: str,
    created_at: datetime,
) -> str:
    """Canonical byte string that lineage attestation signatures cover."""
    return "\n".join(
        [
            "querygraph-lineage-attestation-v1",
            f"issuer:{issuer}",
            f"subject:{subject}",
            f"event_hash:{event_hash}",
            f"merkle_root:{merkle_root}",
            f"created_at:{created_at.isoformat()}",
        ]
    )


def append_jsonl(path: str | Path, value: BaseModel | dict[str, Any]) -> Path:
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    data = value.model_dump(mode="json") if isinstance(value, BaseModel) else value
    with target.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(data, sort_keys=True))
        handle.write("\n")
    return target
