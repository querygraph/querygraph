from __future__ import annotations

import asyncio
import json

import pytest

from querygraph.mcp_server import demo_rights_layer, parse_action
from querygraph.odrl import Action

mcp = pytest.importorskip("mcp", reason="querygraph[mcp] extra not installed")

from querygraph.mcp_server import create_server  # noqa: E402


OSI_YAML = """
version: 0.2.0.dev0
semantic_model:
  name: resilience_model
  ai_context:
    instructions: Prefer governed Sail columns.
    synonyms: [resilience desk]
  datasets:
    - name: county_finance
      source: sail.qg_lakehouse.government_finance__countydata
      ai_context:
        synonyms: [county budgets]
      fields:
        - name: total_revenue
          expression:
            dialects:
              - dialect: SAIL_SQL
                expression: "`total_revenue`"
  metrics:
    - name: fiscal_capacity
      expression:
        dialects:
          - dialect: ANSI_SQL
            expression: SUM(total_revenue - mandated_spend)
"""


@pytest.fixture()
def server(tmp_path):
    pytest.importorskip("yaml", reason="pyyaml required for OSI loading")
    osi_file = tmp_path / "model.yaml"
    osi_file.write_text(OSI_YAML)
    return create_server(osi_path=osi_file)


def call_tool(server, name: str, arguments: dict) -> dict:
    result = asyncio.run(server.call_tool(name, arguments))
    # FastMCP returns (content_blocks, structured_result) in recent SDKs.
    if isinstance(result, tuple):
        return result[1]
    return json.loads(result[0].text)


def test_parse_action_accepts_all_spellings():
    assert parse_action("read") is Action.READ
    assert parse_action("odrl:read") is Action.READ
    assert parse_action("READ") is Action.READ
    assert parse_action("index") is Action.INDEX
    with pytest.raises(ValueError):
        parse_action("frobnicate")


def test_demo_rights_layer_dual_gates():
    rights = demo_rights_layer()
    assert rights.decide("did:example:qg-agent", "qg_lakehouse", Action.READ).allowed
    assert not rights.decide(
        "did:example:qg-agent", "qg_lakehouse", Action.DERIVE
    ).allowed


def test_server_lists_governed_tools(server):
    tools = {tool.name for tool in asyncio.run(server.list_tools())}
    assert {
        "search_semantic_model",
        "resolve_metric",
        "check_access",
        "build_navigator_bundle",
        "run_qglake_story",
        "verify_envelope",
    } <= tools


def test_resolve_metric_falls_back_to_ansi(server):
    result = call_tool(
        server, "resolve_metric", {"name": "fiscal_capacity", "dialect": "SAIL_SQL"}
    )
    assert result["expression"] == "SUM(total_revenue - mandated_spend)"


def test_search_semantic_model_by_synonym(server):
    result = call_tool(server, "search_semantic_model", {"term": "county budgets"})
    assert {"kind": "dataset", "name": "county_finance"} in result["matches"]


def test_check_access_denial_is_a_receipt_not_an_error(server):
    result = call_tool(
        server,
        "check_access",
        {
            "principal": "did:example:qg-agent",
            "resource": "qg_lakehouse",
            "action": "derive",
        },
    )
    assert result["allowed"] is False
    assert result["receipt"]["reason"]


def test_verify_envelope_roundtrip(server):
    from querygraph.typedid import TypeDidAgent

    supervisor = TypeDidAgent.new("SupervisorAgent")
    envelope = supervisor.request(
        TypeDidAgent.new("FinanceAgent"),
        action="summarize",
        resource="compartment:finance",
        payload={"question": "?"},
    )

    result = call_tool(
        server, "verify_envelope", {"envelope": envelope.model_dump(mode="json")}
    )
    assert result["payload_hash_valid"] is True
    if result["crypto_available"]:
        assert result["signed"] is True
        assert result["signature_valid"] is True
