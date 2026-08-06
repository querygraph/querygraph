from __future__ import annotations

from dataclasses import dataclass
from hashlib import sha256

from querygraph.base58 import b58encode


@dataclass(frozen=True)
class DidDocument:
    id: str
    controller: str
    public_key_multibase: str
    context: list[str] | None = None
    service_endpoint: str | None = None

    @classmethod
    def new_oyd(cls, seed: bytes | str, controller: str) -> "DidDocument":
        seed_bytes = seed.encode() if isinstance(seed, str) else seed
        digest = sha256(seed_bytes).digest()
        multihash = bytes([0x12, 0x20]) + digest
        fingerprint = b58encode(multihash)
        return cls(
            context=[
                "https://www.w3.org/ns/did/v1",
                "https://w3id.org/security/suites/ed25519-2020/v1",
            ],
            id=f"did:oyd:z{fingerprint}",
            controller=controller,
            public_key_multibase=f"z{b58encode(digest)}",
        )

    def with_service_endpoint(self, endpoint: str) -> "DidDocument":
        return DidDocument(
            context=self.context,
            id=self.id,
            controller=self.controller,
            public_key_multibase=self.public_key_multibase,
            service_endpoint=endpoint,
        )

    def to_json(self) -> dict:
        doc = {
            "id": self.id,
            "controller": self.controller,
            "public_key_multibase": self.public_key_multibase,
            "service_endpoint": self.service_endpoint,
        }
        if self.context is not None:
            doc["@context"] = self.context
        return doc
