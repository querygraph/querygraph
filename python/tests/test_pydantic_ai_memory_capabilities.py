from __future__ import annotations

import asyncio
from typing import Any

from querygraph.api_auth import mint_envelope_header
from querygraph.pydantic_ai_capabilities import (
    HttpResult,
    QueryGraphAgentDeps,
    build_access_credential_capability,
    build_memory_capability,
    run_recaller,
    run_specialist,
)
from querygraph.typedid import TypeDidAgent, TypeDidEnvelope


class FakeQueryGraph:
    def __init__(self, allowed: set[str]) -> None:
        self.allowed = allowed
        self.memories: list[dict[str, Any]] = []

    def __call__(
        self,
        _base_url: str,
        path: str,
        payload: dict[str, Any],
        credential: TypeDidAgent,
    ) -> HttpResult:
        did = credential.did_key()
        assert did is not None
        if path == "/v1/models/import/croissant":
            return HttpResult(200, {"imported": "resilience_signals_semantic_model"})
        if path == "/v1/answer":
            return HttpResult(
                200,
                {
                    "answer": "Answerable from governed sources sail.resilience.",
                    "plans": [{"source": "sail.resilience"}],
                },
            )
        if did not in self.allowed:
            return HttpResult(
                403,
                {
                    "error": "policy denied",
                    "receipt": {"allowed": False, "subject": did},
                },
            )
        if path == "/v1/memory/remember":
            memory = {"id": f"mem-{len(self.memories) + 1}", "text": payload["text"]}
            self.memories.append(memory)
            return HttpResult(200, {"allowed": True, "subject": did, "result": memory})
        if path == "/v1/memory/recall":
            return HttpResult(
                200,
                {
                    "allowed": True,
                    "subject": did,
                    "result": {"hits": list(self.memories), "redacted": []},
                },
            )
        raise AssertionError(f"unexpected path {path}")


def test_v2_agents_carry_both_capabilities_and_policy_isolates_memory() -> None:
    specialist = TypeDidAgent.new("Specialist", seed="test:specialist")
    supervisor = TypeDidAgent.new("Supervisor", seed="test:supervisor")
    outsider = TypeDidAgent.new("Outsider", seed="test:outsider")
    transport = FakeQueryGraph({specialist.did_key(), supervisor.did_key()})

    specialist_report = asyncio.run(
        run_specialist(
            QueryGraphAgentDeps("http://fake", specialist, transport=transport)
        )
    )
    supervisor_report = asyncio.run(
        run_recaller(
            "supervisor",
            QueryGraphAgentDeps("http://fake", supervisor, transport=transport),
        )
    )
    outsider_report = asyncio.run(
        run_recaller(
            "outsider",
            QueryGraphAgentDeps("http://fake", outsider, transport=transport),
        )
    )

    assert specialist_report["capabilities"] == [
        "querygraph.typedid-credential",
        "querygraph.marciana-memory",
    ]
    assert specialist_report["memoryId"] == "mem-1"
    assert supervisor_report["hits"][0]["id"] == "mem-1"
    assert outsider_report["hits"] == []
    assert outsider_report["denial"]["receipt"]["allowed"] is False


def test_capabilities_expose_toolsets_without_exposing_private_seed() -> None:
    assert build_access_credential_capability().get_toolset() is not None
    assert build_memory_capability().get_toolset() is not None
    credential = TypeDidAgent.new("Private", seed="do-not-serialize")
    assert "seed" not in credential.model_dump()


def test_http_auth_uses_the_signing_did_as_the_policy_subject() -> None:
    credential = TypeDidAgent.new("ApiClient", seed="test:api-client")
    envelope = TypeDidEnvelope.model_validate_json(
        mint_envelope_header(credential, path="/v1/answer", body=b"{}")
    )
    assert envelope.sender == credential.did_key()
    assert envelope.verification_method is not None
    assert envelope.verification_method.split("#", 1)[0] == envelope.sender
    assert envelope.verify_signature()
