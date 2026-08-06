from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[1]
RUST_ROOT = ROOT.parent


def run_rust(*args: str) -> dict:
    return json.loads(
        subprocess.check_output(["cargo", "run", "--quiet", "--", *args], cwd=RUST_ROOT)
    )


def run_python(*args: str) -> dict:
    return json.loads(
        subprocess.check_output([sys.executable, "-m", "querygraph", *args], cwd=ROOT)
    )


def test_navigator_cli_matches_rust_bundle_shape_and_content():
    args = [
        "navigator",
        "--dataset-name",
        "Hazard vocabulary",
        "--description",
        "Controlled vocabulary with multilingual technical terms",
        "--landing-page",
        "https://querygraph.ai/datasets/hazards",
        "--data-url",
        "https://querygraph.ai/datasets/hazards.csv",
        "--creator",
        "QueryGraph",
        "--agent-name",
        "AI Navigator",
    ]

    rust = run_rust(*args)
    python = run_python(*args)

    rust["generatedAt"] = "<normalized>"
    python["generatedAt"] = "<normalized>"
    assert python == rust


def test_qglake_story_governance_semantics_match_rust():
    """The two qglake stories have different report shapes but must agree on
    the governance semantics: same question, same compartmentalized specialist
    roster, the restricted broker (and only it) denied, and a complete
    OpenLineage + Ed25519-attested evidence chain on both sides."""
    rust = run_rust("qglake-story", "--json")
    python = run_python("qglake-story")

    assert python["prompt"]["question"] == rust["question"]

    rust_specialists = [(s["agent"]["name"], s["access"]["allowed"]) for s in rust["specialists"]]
    python_specialists = [
        (r["agent"], r["status"] == "allowed") for r in python["responses"]
    ]
    assert python_specialists == rust_specialists

    denied_rust = [name for name, allowed in rust_specialists if not allowed]
    assert denied_rust == ["RestrictedDataBroker"]

    # Both sides emit a COMPLETE OpenLineage event whose hash is attested.
    assert rust["open_lineage"]["eventType"] == "COMPLETE"
    assert python["openlineage"]["eventType"] == "COMPLETE"

    # Attestation schemas are field-for-field identical across languages.
    assert sorted(rust["did_attestation"].keys()) == sorted(
        python["attestation"].keys()
    )
    assert rust["did_attestation"]["signature_type"] == "Ed25519Signature2020"
    # Python signs for real when the crypto extra is installed; otherwise the
    # digest is explicitly marked unsigned — it must never masquerade.
    python_signature = python["attestation"]["signature"]
    assert python_signature.startswith(("ed25519:", "unsigned:sha256:"))

    # Every envelope on both sides binds its payload by SHA-256.
    for specialist in rust["specialists"]:
        assert len(specialist["request"]["payload_sha256"]) == 64
    for response in python["responses"]:
        assert len(response["envelope"]["payload_sha256"]) == 64


def test_both_openlineage_events_conform_to_official_schema():
    """Interop is proven, not asserted: the events both CLIs emit must
    validate against the official OpenLineage 2-0-2 JSON Schema, including
    the UUID run.runId format."""
    pytest.importorskip("jsonschema")
    from querygraph.validation import validate_openlineage_schema

    python_event = run_python("qglake-story")["openlineage"]
    rust_event = run_rust("qglake-story", "--json")["open_lineage"]

    assert validate_openlineage_schema(python_event) == []
    assert validate_openlineage_schema(rust_event) == []


def test_a2a_agent_cards_agree_across_languages():
    """The Agent Card skill list and security contract are a cross-language
    contract: both CLIs must publish the same skills, protocol version, and
    TypeDID security scheme (implementation versions may differ)."""
    rust = run_rust("agent-card", "--base-url", "http://example.com")
    python = run_python("agent-card", "--base-url", "http://example.com")

    assert python["protocolVersion"] == rust["protocolVersion"]
    assert python["url"] == rust["url"]
    assert python["skills"] == rust["skills"]
    assert python["securitySchemes"] == rust["securitySchemes"]
    assert python["capabilities"] == rust["capabilities"]


def test_python_envelope_auth_accepted_by_live_rust_server():
    """Live cross-language auth: qg-python mints the x-qg-envelope header,
    qg-rust `serve --require-auth` accepts it, and rejects the same request
    without the header."""
    pytest.importorskip("cryptography", reason="querygraph[crypto] not installed")
    import os
    import time
    import urllib.error
    import urllib.request

    from querygraph.api_auth import governed_post
    from querygraph.typedid import TypeDidAgent

    subprocess.check_call(
        ["cargo", "build", "--quiet"], cwd=RUST_ROOT
    )
    port = 18000 + os.getpid() % 2000
    server = subprocess.Popen(
        [
            str(RUST_ROOT / "target" / "debug" / "querygraph"),
            "serve",
            "--port",
            str(port),
            "--require-auth",
        ],
        cwd=RUST_ROOT,
    )
    base = f"http://127.0.0.1:{port}"
    try:
        for _ in range(50):
            try:
                urllib.request.urlopen(f"{base}/v1/health", timeout=1)
                break
            except OSError:
                time.sleep(0.2)
        else:
            raise RuntimeError("qg-server did not come up")

        # Unauthenticated request to a governed route → 401 with a receipt.
        try:
            urllib.request.urlopen(
                urllib.request.Request(
                    f"{base}/v1/answer",
                    data=json.dumps({"question": "?"}).encode(),
                    headers={"Content-Type": "application/json"},
                ),
                timeout=5,
            )
            raise AssertionError("expected HTTP 401")
        except urllib.error.HTTPError as error:
            assert error.code == 401
            receipt = json.loads(error.read())["receipt"]
            assert receipt["allowed"] is False

        # Python-signed envelope → accepted, answer returned.
        agent = TypeDidAgent.new("ApiClient")
        result = governed_post(
            base, "/v1/answer", {"question": "what is fiscal capacity?"}, agent
        )
        assert result["synthesizedBy"] == "deterministic"
        assert "answer" in result
    finally:
        server.terminate()
        server.wait(timeout=10)


def test_rust_verifies_python_ed25519_envelope():
    """Cross-language crypto parity: an envelope signed by qg-python (real
    Ed25519 under a did:key verification method) must verify in qg-rust."""
    pytest.importorskip("cryptography", reason="querygraph[crypto] not installed")
    from querygraph.typedid import TypeDidAgent

    supervisor = TypeDidAgent.new("SupervisorAgent")
    envelope = supervisor.request(
        TypeDidAgent.new("FinanceAgent"),
        action="summarize",
        resource="compartment:finance",
        payload={"question": "Where is fiscal stress highest?", "unicode": "café ☕"},
    )
    assert envelope.is_signed()

    process = subprocess.run(
        ["cargo", "run", "--quiet", "--", "verify-envelope", "--file", "-"],
        cwd=RUST_ROOT,
        input=json.dumps(envelope.model_dump(mode="json")),
        capture_output=True,
        text=True,
    )
    report = json.loads(process.stdout)
    assert report["payload_hash_valid"] is True
    assert report["signature_valid"] is True
    assert process.returncode == 0

    # A tampered envelope must fail with a non-zero exit.
    tampered = envelope.model_dump(mode="json") | {"resource": "compartment:other"}
    process = subprocess.run(
        ["cargo", "run", "--quiet", "--", "verify-envelope", "--file", "-"],
        cwd=RUST_ROOT,
        input=json.dumps(tampered),
        capture_output=True,
        text=True,
    )
    assert process.returncode == 1
    assert json.loads(process.stdout)["signature_valid"] is False
