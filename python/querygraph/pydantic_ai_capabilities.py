"""Pydantic AI v2 capabilities for TypeDID access and Marciana memory.

The private signing seed lives only inside :class:`QueryGraphAgentDeps`.
Models see tools, public identity, and governed results; they never receive
the credential object or seed in instructions or tool arguments.
"""

from __future__ import annotations

import asyncio
import json
import urllib.error
from collections.abc import Callable
from dataclasses import dataclass, field
from typing import Any

from pydantic_ai import Agent, RunContext
from pydantic_ai.capabilities import Capability
from pydantic_ai.models.test import TestModel

from querygraph.api_auth import governed_post
from querygraph.typedid import TypeDidAgent


@dataclass(frozen=True)
class HttpResult:
    """One signed QueryGraph HTTP result, including policy denials."""

    status: int
    body: dict[str, Any]

    @property
    def allowed(self) -> bool:
        return 200 <= self.status < 300


SignedTransport = Callable[[str, str, dict[str, Any], TypeDidAgent], HttpResult]


def signed_http_transport(
    base_url: str,
    path: str,
    payload: dict[str, Any],
    credential: TypeDidAgent,
) -> HttpResult:
    """Call qg-rust and retain structured 4xx receipts for the agent."""

    try:
        return HttpResult(200, governed_post(base_url, path, payload, credential))
    except urllib.error.HTTPError as error:
        raw = error.read()
        try:
            body = json.loads(raw) if raw else {"error": str(error)}
        except json.JSONDecodeError:
            body = {"error": raw.decode(errors="replace") or str(error)}
        return HttpResult(error.code, body)


def demo_semantic_model() -> dict[str, Any]:
    """Small Croissant model that gives the deterministic answer real evidence."""

    return {
        "name": "Resilience Signals",
        "description": "Governed household resilience indicators",
        "distribution": [{"name": "resilience.parquet"}],
        "recordSet": [
            {
                "name": "observations",
                "field": [
                    {
                        "name": "monthly_energy_burden",
                        "description": "Monthly household energy burden",
                        "sameAs": "https://querygraph.ai/ontology/energyBurden",
                    }
                ],
            }
        ],
    }


@dataclass
class QueryGraphAgentDeps:
    """Per-agent dependencies; credential material is deliberately opaque."""

    base_url: str
    credential: TypeDidAgent = field(repr=False)
    space: str = "memory/team:marciana/shared"
    question: str = "Which governed signal describes monthly energy burden?"
    purpose: str = "resilience-research"
    model: dict[str, Any] = field(default_factory=demo_semantic_model)
    transport: SignedTransport = field(default=signed_http_transport, repr=False)
    last_answer: str | None = None
    last_memory_id: str | None = None
    last_recall: list[dict[str, Any]] = field(default_factory=list)
    last_denial: dict[str, Any] | None = None

    @property
    def public_did(self) -> str:
        """The public signing identity; never the private deterministic seed."""

        did = self.credential.did_key()
        if did is None:
            raise RuntimeError(
                "QueryGraph credentials require the querygraph[crypto] extra and a seed"
            )
        return did

    async def post(self, path: str, payload: dict[str, Any]) -> HttpResult:
        return await asyncio.to_thread(
            self.transport, self.base_url, path, payload, self.credential
        )


def build_access_credential_capability() -> Capability[QueryGraphAgentDeps]:
    """Credential capability: public identity and authenticated data access."""

    capability = Capability[QueryGraphAgentDeps](
        id="querygraph.typedid-credential",
        description="Use the agent's private TypeDID credential for governed QueryGraph access.",
        instructions=(
            "Use these tools for QueryGraph data access. The signing seed is a runtime "
            "dependency and must never be requested, repeated, or placed in model context."
        ),
    )

    @capability.tool(
        name="credential_identity",
        description="Return the public did:key for the active credential (never private key material).",
        sequential=True,
    )
    async def credential_identity(ctx: RunContext[QueryGraphAgentDeps]) -> str:
        return json.dumps({"authenticatedAs": ctx.deps.public_did})

    @capability.tool(
        name="query_governed_answer",
        description="Import the demo semantic model and answer the assigned question with signed requests.",
        sequential=True,
    )
    async def query_governed_answer(ctx: RunContext[QueryGraphAgentDeps]) -> str:
        imported = await ctx.deps.post("/v1/models/import/croissant", ctx.deps.model)
        if not imported.allowed:
            ctx.deps.last_denial = imported.body
            return json.dumps(
                {"allowed": False, "status": imported.status, "receipt": imported.body},
                sort_keys=True,
            )
        answer = await ctx.deps.post(
            "/v1/answer", {"question": ctx.deps.question}
        )
        if not answer.allowed:
            ctx.deps.last_denial = answer.body
            return json.dumps(
                {"allowed": False, "status": answer.status, "receipt": answer.body},
                sort_keys=True,
            )
        ctx.deps.last_answer = str(answer.body.get("answer", answer.body))
        return json.dumps(
            {
                "allowed": True,
                "authenticatedAs": ctx.deps.public_did,
                "answer": ctx.deps.last_answer,
                "plans": answer.body.get("plans", []),
            },
            sort_keys=True,
        )

    return capability


def _memory_result(body: dict[str, Any]) -> dict[str, Any]:
    result = body.get("result", body)
    return result if isinstance(result, dict) else {"value": result}


def build_memory_capability() -> Capability[QueryGraphAgentDeps]:
    """Memory capability backed by qg-rust, TypeSec, Grust, and Turso."""

    capability = Capability[QueryGraphAgentDeps](
        id="querygraph.marciana-memory",
        description="Remember, recall, and forget policy-scoped Marciana memories.",
        instructions=(
            "Use memory only through these tools. Treat denials as authoritative. "
            "Never invent a subject: the server derives it from the signed credential."
        ),
    )

    @capability.tool(
        name="remember_governed_finding",
        description="Persist the last governed answer in the team's policy-scoped memory.",
        sequential=True,
    )
    async def remember_governed_finding(ctx: RunContext[QueryGraphAgentDeps]) -> str:
        if not ctx.deps.last_answer:
            return json.dumps({"allowed": False, "error": "no governed answer to remember"})
        response = await ctx.deps.post(
            "/v1/memory/remember",
            {
                "space": ctx.deps.space,
                "text": ctx.deps.last_answer,
                "kind": "semantic",
                "purpose": ctx.deps.purpose,
            },
        )
        if not response.allowed:
            ctx.deps.last_denial = response.body
            return json.dumps(
                {"allowed": False, "status": response.status, "receipt": response.body},
                sort_keys=True,
            )
        result = _memory_result(response.body)
        memory_id = result.get("id")
        ctx.deps.last_memory_id = str(memory_id) if memory_id is not None else None
        return json.dumps(
            {
                "allowed": True,
                "authenticatedAs": ctx.deps.public_did,
                "memory": result,
            },
            sort_keys=True,
        )

    @capability.tool(
        name="recall_team_memory",
        description="Recall the team's governed findings at Internal clearance.",
        sequential=True,
    )
    async def recall_team_memory(ctx: RunContext[QueryGraphAgentDeps]) -> str:
        response = await ctx.deps.post(
            "/v1/memory/recall",
            {
                "space": ctx.deps.space,
                "query": "governed sources",
                "clearance": "internal",
                "purpose": ctx.deps.purpose,
            },
        )
        if not response.allowed:
            ctx.deps.last_denial = response.body
            return json.dumps(
                {"allowed": False, "status": response.status, "receipt": response.body},
                sort_keys=True,
            )
        result = _memory_result(response.body)
        hits = result.get("hits", [])
        ctx.deps.last_recall = hits if isinstance(hits, list) else []
        return json.dumps(
            {
                "allowed": True,
                "authenticatedAs": ctx.deps.public_did,
                "hits": ctx.deps.last_recall,
                "redacted": result.get("redacted", []),
            },
            sort_keys=True,
        )

    @capability.tool(
        name="forget_last_memory",
        description="Delete the memory created by this agent, when policy permits.",
        sequential=True,
    )
    async def forget_last_memory(ctx: RunContext[QueryGraphAgentDeps]) -> str:
        ids = [ctx.deps.last_memory_id] if ctx.deps.last_memory_id else []
        response = await ctx.deps.post(
            "/v1/memory/forget", {"space": ctx.deps.space, "ids": ids}
        )
        if not response.allowed:
            ctx.deps.last_denial = response.body
            return json.dumps(
                {"allowed": False, "status": response.status, "receipt": response.body},
                sort_keys=True,
            )
        return json.dumps(
            {"allowed": True, "memory": _memory_result(response.body)}, sort_keys=True
        )

    return capability


def build_querygraph_agent(name: str) -> Agent[QueryGraphAgentDeps, str]:
    """Create a provider-free v2 agent carrying both production capabilities."""

    return Agent(
        TestModel(call_tools=[]),
        name=name,
        deps_type=QueryGraphAgentDeps,
        output_type=str,
        instructions=(
            "Operate as a governed QueryGraph teammate. Use the credential capability "
            "for data access and the memory capability for durable team context."
        ),
        capabilities=[
            build_access_credential_capability(),
            build_memory_capability(),
        ],
    )


async def run_tool_phase(
    agent: Agent[QueryGraphAgentDeps, str],
    deps: QueryGraphAgentDeps,
    tool_name: str,
    prompt: str,
) -> str:
    """Run one deterministic agent turn through a selected capability tool."""

    with agent.override(model=TestModel(call_tools=[tool_name])):
        result = await agent.run(prompt, deps=deps)
    return str(result.output)


async def run_specialist(
    deps: QueryGraphAgentDeps,
) -> dict[str, Any]:
    """Credentialed specialist: answer, then commit the finding to memory."""

    agent = build_querygraph_agent("resilience-specialist")
    identity = await run_tool_phase(
        agent, deps, "credential_identity", "Identify the active public credential."
    )
    answer = await run_tool_phase(
        agent, deps, "query_governed_answer", "Use governed evidence to answer the question."
    )
    remembered = await run_tool_phase(
        agent,
        deps,
        "remember_governed_finding",
        "Commit the governed answer to our shared memory.",
    )
    return {
        "agent": agent.name,
        "publicDid": deps.public_did,
        "capabilities": [
            "querygraph.typedid-credential",
            "querygraph.marciana-memory",
        ],
        "identityTurn": identity,
        "answerTurn": answer,
        "memoryTurn": remembered,
        "answer": deps.last_answer,
        "memoryId": deps.last_memory_id,
    }


async def run_recaller(
    name: str, deps: QueryGraphAgentDeps
) -> dict[str, Any]:
    """Supervisor or outsider: exercise policy-bound recall."""

    agent = build_querygraph_agent(name)
    transcript = await run_tool_phase(
        agent, deps, "recall_team_memory", "Recall the team's durable governed finding."
    )
    return {
        "agent": agent.name,
        "publicDid": deps.public_did,
        "capabilities": [
            "querygraph.typedid-credential",
            "querygraph.marciana-memory",
        ],
        "recallTurn": transcript,
        "hits": deps.last_recall,
        "denial": deps.last_denial,
    }
