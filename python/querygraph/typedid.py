from __future__ import annotations

from datetime import UTC, datetime
from functools import lru_cache
from hashlib import sha256
from typing import Any, Literal

from pydantic import BaseModel, Field

from querygraph import crypto
from querygraph.did import DidDocument
from querygraph.odrl import Action, Policy


def sha256_hex(value: bytes | str) -> str:
    data = value.encode() if isinstance(value, str) else value
    return sha256(data).hexdigest()


@lru_cache(maxsize=256)
def _signer_from_seed(seed: str) -> "crypto.Ed25519Signer":
    return crypto.Ed25519Signer.from_seed(seed)


class AccessReceipt(BaseModel):
    principal: str
    resource: str
    action: str
    allowed: bool
    reason: str
    policy_id: str | None = None
    issued_at: datetime = Field(default_factory=lambda: datetime.now(UTC))


# Default TypeDID profile id, mirroring TypeSec 0.11 "Burano"'s
# `TypeDidProfile::ed25519_x25519_chacha20()`.
TYPEDID_PROFILE = "ed25519-x25519-chacha20"


class TypeDidEnvelope(BaseModel):
    protocol: str = "querygraph.typedid.v1"
    conversation_id: str
    sender: str
    recipient: str
    action: str
    resource: str
    # Audit-safe attestation fields, mirroring the Rust port's adoption of
    # TypeSec 0.11 "Burano" `VerifiedTypeDidMessage::attestation()`: privacy
    # level, negotiated profile, and a digest binding the attestation to this
    # exact envelope — surfaced without revealing the payload.
    privacy: str = "secret"
    profile: str = TYPEDID_PROFILE
    content_type: str = "application/json"
    payload: dict[str, Any]
    payload_sha256: str
    signature: str
    # did:key verification method for `ed25519:` signatures; None when the
    # envelope carries only an `unsigned:sha256:` digest (crypto extra absent).
    verification_method: str | None = None
    envelope_digest: str = ""
    created_at: datetime = Field(default_factory=lambda: datetime.now(UTC))

    @classmethod
    def create(
        cls,
        *,
        sender: DidDocument | str,
        recipient: DidDocument | str,
        action: str,
        resource: str,
        payload: dict[str, Any],
        conversation_id: str | None = None,
        content_type: str = "application/json",
        privacy: str = "secret",
        profile: str = TYPEDID_PROFILE,
        signer: "crypto.Ed25519Signer | None" = None,
    ) -> "TypeDidEnvelope":
        sender_id = sender.id if isinstance(sender, DidDocument) else sender
        recipient_id = recipient.id if isinstance(recipient, DidDocument) else recipient
        payload_hash = sha256_hex(_canonical(payload))
        conversation = conversation_id or f"qg:{payload_hash[:16]}"
        signing_payload = signing_payload_v1(
            sender=sender_id,
            recipient=recipient_id,
            action=action,
            resource=resource,
            payload_sha256=payload_hash,
        )
        verification_method: str | None = None
        if signer is not None:
            signature = signer.sign(signing_payload)
            verification_method = signer.verification_method()
        else:
            signature = crypto.unsigned_digest(signing_payload)
        envelope_digest = sha256_hex(
            "\n".join(
                [
                    "querygraph-typedid-envelope-digest-v1",
                    conversation,
                    privacy,
                    profile,
                    signature,
                ]
            )
        )
        return cls(
            conversation_id=conversation,
            sender=sender_id,
            recipient=recipient_id,
            action=action,
            resource=resource,
            privacy=privacy,
            profile=profile,
            content_type=content_type,
            payload=payload,
            payload_sha256=payload_hash,
            signature=signature,
            verification_method=verification_method,
            envelope_digest=f"sha256:{envelope_digest}",
        )

    def signing_payload(self) -> str:
        return signing_payload_v1(
            sender=self.sender,
            recipient=self.recipient,
            action=self.action,
            resource=self.resource,
            payload_sha256=self.payload_sha256,
        )

    def verify_payload(self) -> bool:
        return self.payload_sha256 == sha256_hex(_canonical(self.payload))

    def verify_signature(self, public_key: bytes | str | None = None) -> bool:
        """Verify the envelope signature.

        `ed25519:` signatures verify against `public_key` (raw bytes, multibase,
        or did:key) or, when omitted, the envelope's own `verification_method`.
        `unsigned:` digests are re-derivable but are never valid signatures.
        """
        if not self.verify_payload():
            return False
        if self.signature.startswith(crypto.SIGNATURE_PREFIX):
            key = public_key or self.verification_method
            if key is None:
                return False
            return crypto.verify(key, self.signing_payload(), self.signature)
        return False

    def is_signed(self) -> bool:
        return self.signature.startswith(crypto.SIGNATURE_PREFIX)


def signing_payload_v1(
    *,
    sender: str,
    recipient: str,
    action: str,
    resource: str,
    payload_sha256: str,
) -> str:
    """Canonical byte string that TypeDID envelope signatures cover."""
    return "\n".join(
        [
            "querygraph-typedid-signing-v1",
            sender,
            recipient,
            action,
            resource,
            payload_sha256,
        ]
    )


class GovernedPrompt(BaseModel):
    question: str
    semantic_context: dict[str, Any]
    allowed_sources: list[str] = Field(default_factory=list)
    denied_sources: list[str] = Field(default_factory=list)
    receipts: list[AccessReceipt] = Field(default_factory=list)


class AgentResponse(BaseModel):
    agent: str
    status: Literal["allowed", "denied"]
    summary: str
    evidence: list[str] = Field(default_factory=list)
    redactions: list[str] = Field(default_factory=list)
    envelope: TypeDidEnvelope


class TypeDidAgent(BaseModel):
    name: str
    did: DidDocument
    capabilities: list[str] = Field(default_factory=list)
    # Deterministic key seed, mirroring Rust TypeSec `Ed25519DidKey::from_seed`.
    # Excluded from serialization: the seed is the private key material.
    seed: str | None = Field(default=None, exclude=True, repr=False)

    @classmethod
    def new(cls, name: str, *, seed: str | None = None) -> "TypeDidAgent":
        agent_seed = seed or f"querygraph-agent:{name}"
        did = DidDocument.new_oyd(agent_seed, name)
        return cls(name=name, did=did, capabilities=[], seed=agent_seed)

    @property
    def signer(self) -> "crypto.Ed25519Signer | None":
        """Real Ed25519 signer when the crypto extra is installed and a seed is known."""
        if self.seed is None or not crypto.CRYPTO_AVAILABLE:
            return None
        return _signer_from_seed(self.seed)

    def did_key(self) -> str | None:
        signer = self.signer
        return signer.did_key() if signer is not None else None

    def to_tool_schema(self, *, flavor: str = "openai") -> dict[str, Any]:
        """Standard JSON-Schema tool definition (OpenAI or Anthropic flavor)."""
        from querygraph.agents import to_tool_schema

        return to_tool_schema(self, flavor=flavor)

    def request(
        self,
        recipient: "TypeDidAgent",
        *,
        action: str,
        resource: str,
        payload: dict[str, Any],
    ) -> TypeDidEnvelope:
        return TypeDidEnvelope.create(
            sender=self.did,
            recipient=recipient.did,
            action=action,
            resource=resource,
            payload=payload,
            signer=self.signer,
        )

    def answer(
        self,
        request: TypeDidEnvelope,
        *,
        status: Literal["allowed", "denied"],
        summary: str,
        evidence: list[str] | None = None,
        redactions: list[str] | None = None,
    ) -> AgentResponse:
        payload = {
            "status": status,
            "summary": summary,
            "evidence": evidence or [],
            "redactions": redactions or [],
            "requestSha256": request.payload_sha256,
        }
        envelope = TypeDidEnvelope.create(
            sender=self.did,
            recipient=request.sender,
            action="respond",
            resource=request.resource,
            payload=payload,
            conversation_id=request.conversation_id,
            signer=self.signer,
        )
        return AgentResponse(
            agent=self.name,
            status=status,
            summary=summary,
            evidence=evidence or [],
            redactions=redactions or [],
            envelope=envelope,
        )


def evaluate_policy(
    *,
    principal: str,
    resource: str,
    action: Action,
    policy: Policy,
) -> AccessReceipt:
    allowed = policy.allows(principal, action)
    return AccessReceipt(
        principal=principal,
        resource=resource,
        action=action.iri(),
        allowed=allowed,
        reason="policy permitted action" if allowed else "policy denied action",
        policy_id=policy.id,
    )


def _canonical(payload: dict[str, Any]) -> str:
    import json

    return json.dumps(payload, sort_keys=True, separators=(",", ":"))
