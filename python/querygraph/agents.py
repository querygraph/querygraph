from __future__ import annotations

import asyncio
from typing import Any, Callable

from pydantic import BaseModel, Field

from querygraph.typedid import AgentResponse, GovernedPrompt, TypeDidAgent

# JSON Schema for the standard governed-tool input shape shared by every
# adapter and by `to_tool_schema()` exports.
TOOL_PARAMETERS_SCHEMA: dict[str, Any] = {
    "type": "object",
    "properties": {
        "question": {
            "type": "string",
            "description": "Natural-language question for the governed specialist.",
        },
        "resource": {
            "type": "string",
            "description": "Governed resource or compartment to consult.",
            "default": "qg_lakehouse",
        },
    },
    "required": ["question"],
}


def tool_description(agent: TypeDidAgent) -> str:
    return (
        f"Governed TypeDID tool for {agent.name}; returns a signed summary or a "
        "denial receipt, with the payload hash and envelope for audit."
    )


def to_tool_schema(agent: TypeDidAgent, *, flavor: str = "openai") -> dict[str, Any]:
    """Export a TypeDID agent as a standard JSON-Schema tool definition.

    `flavor="openai"` emits the OpenAI function-calling shape (also accepted by
    Mistral, vLLM, Ollama, and most local runtimes); `flavor="anthropic"` emits
    the Anthropic tool-use shape.
    """
    name = agent.name
    if flavor == "openai":
        return {
            "type": "function",
            "function": {
                "name": name,
                "description": tool_description(agent),
                "parameters": TOOL_PARAMETERS_SCHEMA,
            },
        }
    if flavor == "anthropic":
        return {
            "name": name,
            "description": tool_description(agent),
            "input_schema": TOOL_PARAMETERS_SCHEMA,
        }
    raise ValueError(f"Unknown tool-schema flavor {flavor!r}")


class TypeDidAgentRun(BaseModel):
    supervisor: TypeDidAgent
    specialists: list[TypeDidAgent]
    prompt: GovernedPrompt
    responses: list[AgentResponse] = Field(default_factory=list)

    def aggregate(self) -> dict[str, Any]:
        allowed = [response for response in self.responses if response.status == "allowed"]
        denied = [response for response in self.responses if response.status == "denied"]
        return {
            "supervisor": self.supervisor.name,
            "question": self.prompt.question,
            "allowedSummaries": [response.summary for response in allowed],
            "denials": [response.summary for response in denied],
            "evidenceHashes": [
                response.envelope.payload_sha256 for response in self.responses
            ],
        }


def deterministic_specialist(
    agent: TypeDidAgent,
    *,
    summary: str,
    status: str = "allowed",
    evidence: list[str] | None = None,
    redactions: list[str] | None = None,
) -> Callable[[dict[str, Any]], AgentResponse]:
    def invoke(payload: dict[str, Any]) -> AgentResponse:
        supervisor = TypeDidAgent.new("SupervisorAgent")
        request = supervisor.request(
            agent,
            action=payload.get("action", "summarize"),
            resource=payload.get("resource", "qg_lakehouse"),
            payload=payload,
        )
        return agent.answer(
            request,
            status="allowed" if status == "allowed" else "denied",
            summary=summary,
            evidence=evidence or [payload.get("resource", "qg_lakehouse")],
            redactions=redactions or [],
        )

    return invoke


class TypeDidLangChainToolAdapter:
    """Small adapter that exposes a TypeDID agent as a LangChain StructuredTool."""

    def __init__(
        self,
        agent: TypeDidAgent,
        handler: Callable[[dict[str, Any]], AgentResponse],
    ) -> None:
        self.agent = agent
        self.handler = handler

    def invoke(self, question: str, resource: str = "qg_lakehouse") -> dict[str, Any]:
        """Run the governed handler; the result always carries the envelope."""
        response = self.handler(
            {"question": question, "resource": resource, "action": "summarize"}
        )
        return response.model_dump(mode="json")

    async def ainvoke(
        self, question: str, resource: str = "qg_lakehouse"
    ) -> dict[str, Any]:
        """Async variant; runs sync handlers on a worker thread."""
        return await asyncio.to_thread(self.invoke, question, resource)

    def to_tool_schema(self, *, flavor: str = "openai") -> dict[str, Any]:
        return to_tool_schema(self.agent, flavor=flavor)

    def _structured_tool_cls(self):
        try:
            from langchain_core.tools import StructuredTool
        except ImportError as exc:  # pragma: no cover - depends on optional extra.
            raise RuntimeError(
                "Install querygraph[agents] to use LangChain tool adapters."
            ) from exc
        return StructuredTool

    def as_tool(self):
        return self._structured_tool_cls().from_function(
            func=self.invoke,
            name=self.agent.name,
            description=tool_description(self.agent),
        )

    def as_async_tool(self):
        """A StructuredTool with both sync and async execution paths."""
        return self._structured_tool_cls().from_function(
            func=self.invoke,
            coroutine=self.ainvoke,
            name=self.agent.name,
            description=tool_description(self.agent),
        )
