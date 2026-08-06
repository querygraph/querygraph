"""Real Ed25519 signing for QueryGraph TypeDID envelopes and lineage attestations.

Mirrors the Rust port's TypeSec `Ed25519DidKey::from_seed` pattern: a 32-byte
private key is derived deterministically from a seed via SHA-256, so agents
recreated from the same seed sign identically across processes. Signatures are
prefixed `ed25519:`; when the optional `cryptography` dependency is missing,
callers fall back to digests prefixed `unsigned:sha256:` that can never be
mistaken for signatures.

Install with `pip install querygraph[crypto]`.
"""

from __future__ import annotations

from hashlib import sha256

from querygraph.base58 import b58encode

try:  # pragma: no cover - trivially exercised by import.
    from cryptography.hazmat.primitives.asymmetric.ed25519 import (
        Ed25519PrivateKey,
        Ed25519PublicKey,
    )
    from cryptography.exceptions import InvalidSignature

    CRYPTO_AVAILABLE = True
except ImportError:  # pragma: no cover - depends on optional extra.
    CRYPTO_AVAILABLE = False

# Multicodec prefix for ed25519-pub (0xed, varint-encoded), per the did:key spec.
_ED25519_MULTICODEC = bytes([0xED, 0x01])

SIGNATURE_PREFIX = "ed25519:"
UNSIGNED_PREFIX = "unsigned:sha256:"


class CryptoUnavailableError(RuntimeError):
    def __init__(self) -> None:
        super().__init__(
            "Real Ed25519 signing requires the 'cryptography' package. "
            "Install querygraph[crypto]."
        )


def _require_crypto() -> None:
    if not CRYPTO_AVAILABLE:
        raise CryptoUnavailableError()


def _as_bytes(value: bytes | str) -> bytes:
    return value.encode() if isinstance(value, str) else value


class Ed25519Signer:
    """A deterministic Ed25519 keypair derived from a seed, or a random one."""

    def __init__(self, private_key: "Ed25519PrivateKey") -> None:
        self._private_key = private_key
        self._public_key = private_key.public_key()

    @classmethod
    def from_seed(cls, seed: bytes | str) -> "Ed25519Signer":
        _require_crypto()
        return cls(Ed25519PrivateKey.from_private_bytes(sha256(_as_bytes(seed)).digest()))

    @classmethod
    def generate(cls) -> "Ed25519Signer":
        _require_crypto()
        return cls(Ed25519PrivateKey.generate())

    @property
    def public_key_bytes(self) -> bytes:
        from cryptography.hazmat.primitives.serialization import (
            Encoding,
            PublicFormat,
        )

        return self._public_key.public_bytes(Encoding.Raw, PublicFormat.Raw)

    @property
    def public_key_multibase(self) -> str:
        return f"z{b58encode(_ED25519_MULTICODEC + self.public_key_bytes)}"

    def did_key(self) -> str:
        """The W3C did:key identifier for this keypair."""
        return f"did:key:{self.public_key_multibase}"

    def verification_method(self) -> str:
        did = self.did_key()
        return f"{did}#{self.public_key_multibase}"

    def sign(self, message: bytes | str) -> str:
        """Sign and return an `ed25519:<hex>` signature string."""
        return SIGNATURE_PREFIX + self._private_key.sign(_as_bytes(message)).hex()


def public_key_from_multibase(multibase: str) -> bytes:
    """Decode a `z…` base58btc multibase key, stripping the ed25519 multicodec."""
    from querygraph.base58 import b58decode

    if not multibase.startswith("z"):
        raise ValueError(f"Unsupported multibase prefix in {multibase!r}")
    raw = b58decode(multibase[1:])
    if raw.startswith(_ED25519_MULTICODEC):
        raw = raw[len(_ED25519_MULTICODEC) :]
    if len(raw) != 32:
        raise ValueError("Expected a 32-byte Ed25519 public key")
    return raw


def public_key_from_did_key(did: str) -> bytes:
    """Resolve a did:key (optionally with fragment) to raw public-key bytes."""
    identifier = did.split("#", 1)[0]
    if not identifier.startswith("did:key:"):
        raise ValueError(f"Not a did:key identifier: {did!r}")
    return public_key_from_multibase(identifier.removeprefix("did:key:"))


def verify(public_key: bytes | str, message: bytes | str, signature: str) -> bool:
    """Verify an `ed25519:<hex>` signature against a message.

    `public_key` may be raw bytes, a multibase string, or a did:key identifier.
    Returns False (never raises) for wrong keys, tampered messages, or
    `unsigned:` digests — those are not signatures.
    """
    _require_crypto()
    if not signature.startswith(SIGNATURE_PREFIX):
        return False
    if isinstance(public_key, str):
        try:
            public_key = (
                public_key_from_did_key(public_key)
                if public_key.startswith("did:key:")
                else public_key_from_multibase(public_key)
            )
        except ValueError:
            return False
    try:
        Ed25519PublicKey.from_public_bytes(public_key).verify(
            bytes.fromhex(signature.removeprefix(SIGNATURE_PREFIX)),
            _as_bytes(message),
        )
        return True
    except (InvalidSignature, ValueError):
        return False


def unsigned_digest(message: bytes | str) -> str:
    """A clearly-labelled non-signature digest for crypto-less environments."""
    return UNSIGNED_PREFIX + sha256(_as_bytes(message)).hexdigest()
