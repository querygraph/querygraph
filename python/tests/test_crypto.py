from __future__ import annotations

import pytest

from querygraph import crypto
from querygraph.lineage import LineageAttestation, OpenLineageRunEvent
from querygraph.typedid import TypeDidAgent, TypeDidEnvelope

pytestmark = pytest.mark.skipif(
    not crypto.CRYPTO_AVAILABLE, reason="querygraph[crypto] extra not installed"
)


def test_seed_derived_keys_are_deterministic():
    a = crypto.Ed25519Signer.from_seed("querygraph-agent:FinanceAgent")
    b = crypto.Ed25519Signer.from_seed("querygraph-agent:FinanceAgent")
    other = crypto.Ed25519Signer.from_seed("querygraph-agent:EnergyAgent")

    assert a.did_key() == b.did_key()
    assert a.did_key().startswith("did:key:z6Mk")  # ed25519 multicodec prefix
    assert a.did_key() != other.did_key()


def test_sign_verify_roundtrip_and_tamper_detection():
    signer = crypto.Ed25519Signer.from_seed("test-seed")
    signature = signer.sign("hello governed world")

    assert signature.startswith("ed25519:")
    assert crypto.verify(signer.public_key_bytes, "hello governed world", signature)
    assert crypto.verify(signer.did_key(), "hello governed world", signature)
    assert crypto.verify(signer.public_key_multibase, "hello governed world", signature)
    assert not crypto.verify(signer.public_key_bytes, "hello tampered world", signature)
    other = crypto.Ed25519Signer.from_seed("other-seed")
    assert not crypto.verify(other.public_key_bytes, "hello governed world", signature)


def test_agent_envelopes_carry_real_signatures():
    supervisor = TypeDidAgent.new("SupervisorAgent")
    finance = TypeDidAgent.new("FinanceAgent")

    request = supervisor.request(
        finance,
        action="summarize",
        resource="compartment:finance",
        payload={"question": "Where is fiscal stress highest?"},
    )

    assert request.is_signed()
    assert request.verification_method == supervisor.signer.verification_method()
    assert request.verify_signature()
    # Verifying against the wrong agent's key must fail.
    assert not request.verify_signature(finance.signer.public_key_bytes)

    tampered = request.model_copy(update={"payload": {"question": "different"}})
    assert not tampered.verify_signature()


def test_response_envelope_signed_by_responder():
    supervisor = TypeDidAgent.new("SupervisorAgent")
    finance = TypeDidAgent.new("FinanceAgent")
    request = supervisor.request(
        finance, action="summarize", resource="compartment:finance", payload={"q": "?"}
    )

    response = finance.answer(request, status="allowed", summary="ok")

    assert response.envelope.is_signed()
    assert response.envelope.verify_signature(finance.signer.public_key_bytes)


def test_unsigned_envelopes_are_clearly_marked():
    envelope = TypeDidEnvelope.create(
        sender="did:example:sender",
        recipient="did:example:recipient",
        action="summarize",
        resource="compartment:finance",
        payload={"q": "?"},
    )

    assert envelope.signature.startswith("unsigned:sha256:")
    assert not envelope.is_signed()
    assert not envelope.verify_signature()


def test_lineage_attestation_signs_and_verifies():
    supervisor = TypeDidAgent.new("SupervisorAgent")
    request = supervisor.request(
        TypeDidAgent.new("SynthesisAgent"),
        action="aggregate",
        resource="querygraph:briefing",
        payload={"summaryCount": 3},
    )
    event = OpenLineageRunEvent.for_agent_run(
        request=request, job_name="crypto-test", inputs=["a"], outputs=["b"]
    )

    attestation = LineageAttestation.from_event(
        issuer=supervisor.did.id,
        subject="querygraph:briefing",
        event_hash=event.event_hash(),
        signer=supervisor.signer,
    )

    assert attestation.signature_type == "QueryGraphEd25519Signature"
    assert attestation.verify()
    assert attestation.verify(supervisor.signer.public_key_bytes)
    forged = attestation.model_copy(update={"event_hash": "0" * 64})
    assert not forged.verify()


def test_multibase_and_did_key_decode_roundtrip():
    signer = crypto.Ed25519Signer.from_seed("roundtrip")

    assert crypto.public_key_from_multibase(signer.public_key_multibase) == signer.public_key_bytes
    assert crypto.public_key_from_did_key(signer.did_key()) == signer.public_key_bytes
    assert (
        crypto.public_key_from_did_key(signer.verification_method())
        == signer.public_key_bytes
    )
