from __future__ import annotations

import asyncio

import pytest

from querygraph.agents import (
    TypeDidLangChainToolAdapter,
    deterministic_specialist,
    to_tool_schema,
)
from querygraph.typedid import TypeDidAgent


def _adapter() -> TypeDidLangChainToolAdapter:
    finance = TypeDidAgent.new("FinanceAgent")
    handler = deterministic_specialist(
        finance, summary="Fiscal capacity summary over governed finance tables."
    )
    return TypeDidLangChainToolAdapter(finance, handler)


def test_openai_tool_schema_shape():
    agent = TypeDidAgent.new("FinanceAgent")
    schema = agent.to_tool_schema()

    assert schema["type"] == "function"
    assert schema["function"]["name"] == "FinanceAgent"
    assert schema["function"]["parameters"]["required"] == ["question"]
    assert "question" in schema["function"]["parameters"]["properties"]


def test_anthropic_tool_schema_shape():
    schema = to_tool_schema(TypeDidAgent.new("EnergyAgent"), flavor="anthropic")

    assert schema["name"] == "EnergyAgent"
    assert schema["input_schema"]["type"] == "object"
    assert "type" not in schema  # Anthropic flavor has no wrapper


def test_unknown_flavor_rejected():
    with pytest.raises(ValueError):
        to_tool_schema(TypeDidAgent.new("X"), flavor="unknown")


def test_adapter_invoke_returns_envelope_and_summary():
    result = _adapter().invoke("Where is fiscal stress highest?")

    assert result["status"] == "allowed"
    assert result["summary"].startswith("Fiscal capacity")
    assert result["envelope"]["payload_sha256"]
    assert result["envelope"]["signature"].startswith(("ed25519:", "unsigned:sha256:"))


def test_adapter_async_invoke_matches_sync():
    adapter = _adapter()
    sync_result = adapter.invoke("Where is fiscal stress highest?")
    async_result = asyncio.run(adapter.ainvoke("Where is fiscal stress highest?"))

    assert async_result["summary"] == sync_result["summary"]
    assert async_result["status"] == sync_result["status"]


def test_langchain_structured_tools_when_extra_installed():
    pytest.importorskip("langchain_core")
    adapter = _adapter()

    tool = adapter.as_tool()
    async_tool = adapter.as_async_tool()

    result = tool.invoke({"question": "Where is fiscal stress highest?"})
    assert result["status"] == "allowed"
    async_result = asyncio.run(
        async_tool.ainvoke({"question": "Where is fiscal stress highest?"})
    )
    assert async_result["summary"] == result["summary"]
