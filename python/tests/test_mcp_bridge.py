"""Real SDK sessions against Rust directly and through the Python bridge."""

from __future__ import annotations

import asyncio
import hashlib
import json
import os
from pathlib import Path
import sys

import pytest

pytest.importorskip("mcp")
from mcp import ClientSession, StdioServerParameters  # noqa: E402
from mcp.client.stdio import stdio_client  # noqa: E402
from mcp.shared.exceptions import McpError  # noqa: E402


def test_rust_and_python_bridge_share_contract_and_session_state():
    configured = os.environ.get("QG_RUST_MCP_BIN")
    if not configured:
        pytest.skip("set QG_RUST_MCP_BIN to the built Rust querygraph executable")
    binary = str(Path(configured).resolve(strict=True))

    async def exercise(parameters):
        async with stdio_client(parameters) as (read, write):
            async with ClientSession(read, write) as session:
                await session.initialize()
                listed = await session.list_tools()
                tools = [tool.model_dump(exclude_none=True) for tool in listed.tools]
                draft = {"version": "pinax.v1", "enterprise": "acme", "tables": []}
                valid = await session.call_tool("validate_pinax", {"document": json.dumps(draft)})
                assert valid.isError is False
                assert valid.structuredContent["valid"] is True
                invalid = await session.call_tool("validate_pinax", {"document": "{}"})
                assert invalid.isError is True
                missing = await session.call_tool("search_semantic_models", {})
                assert missing.isError is True
                with pytest.raises(McpError) as unknown:
                    await session.call_tool("missing_tool", {})
                assert unknown.value.error.code == -32602
                imported = await session.call_tool("import_semantic_model", {
                    "osi": {"semantic_model": {"name": "fixture", "datasets": [
                        {"name": "energy", "source": "sail.energy", "fields": []}
                    ]}}
                })
                assert imported.isError is False
                plural = await session.call_tool("search_semantic_models", {"term": "energy"})
                singular = await session.call_tool("search_semantic_model", {"term": "energy"})
                assert plural.structuredContent == singular.structuredContent
                assert plural.structuredContent["matches"]
                return tools, valid.structuredContent, plural.structuredContent

    direct = asyncio.run(exercise(StdioServerParameters(command=binary, args=["mcp-serve"])))
    bridge = asyncio.run(exercise(StdioServerParameters(
        command=sys.executable,
        args=["-m", "querygraph", "mcp-serve", "--rust-backend", binary],
    )))
    assert direct == bridge


def test_signed_discovery_through_both_transports_filters_columns_without_catalog(tmp_path):
    configured = os.environ.get("QG_RUST_MCP_BIN")
    if not configured:
        pytest.skip("set QG_RUST_MCP_BIN to the built Rust querygraph executable")
    from querygraph.crypto import Ed25519Signer
    from querygraph.typedid import TypeDidEnvelope

    binary = str(Path(configured).resolve(strict=True))
    signer = Ed25519Signer.from_seed("mcp-discovery-conformance")
    subject = signer.did_key()
    registry = {"version": "pinax.v1", "enterprise": "acme", "tables": [{
        "name": "customers", "revision": 1, "profile": "custom",
        "metadata": {"owner": "platform", "steward": "data",
                     "description": "Customers", "retention_days": 30},
        "security": {"policy": "enterprise", "revision": 1,
                     "classification": "internal", "purposes": ["analytics"]},
        "columns": [{"id": number, "name": name, "data_type": {"kind": "string"},
                     "nullability": "required", "semantic": f"customer.{name}",
                     "description": name, "classification": "internal"}
                    for number, name in [(1, "customer_id"), (2, "private_email")]],
        "primary_key": [1],
    }]}
    (tmp_path / "registry.json").write_text(json.dumps(registry))
    (tmp_path / "policy.yaml").write_text(f"""roles:
  - name: analyst
    permissions: [read]
    resources:
      - pinax/acme/customers/revisions/1
      - pinax/acme/customers/revisions/1/columns/1
assignments:
  - subject: "{subject}"
    roles: [analyst]
""")
    (tmp_path / "credential.json").write_text("{}")
    config = tmp_path / "service.json"
    config.write_text(json.dumps({
        "registry": "registry.json", "warehouse": "warehouse", "policy": "policy.yaml",
        "policy_id": "enterprise", "policy_revision": 1,
        "server_did": "did:example:registry", "lakecat_origin": "http://127.0.0.1:1/",
        "lakecat_subject": subject, "lakecat_envelope": "credential.json",
    }))

    def arguments(intent, resource="/mcp/discover_pinax_tables"):
        text = json.dumps(intent)
        envelope = TypeDidEnvelope.create(
            sender=subject, recipient="did:example:registry", action="invoke",
            resource=resource, payload={"bodySha256": hashlib.sha256(text.encode()).hexdigest()},
            signer=signer,
        )
        return {"intent": text, "envelope": envelope.model_dump(mode="json")}

    async def exercise(parameters):
        async with stdio_client(parameters) as (read, write):
            async with ClientSession(read, write) as session:
                await session.initialize()
                request = arguments({"purpose": "analytics", "limit": 10})
                result = await session.call_tool("discover_pinax_tables", request)
                assert result.isError is False
                page = result.structuredContent
                assert [table["name"] for table in page["tables"]] == ["customers"]
                assert [column["name"] for column in page["tables"][0]["columns"]] == ["customer_id"]
                assert "private_email" not in json.dumps(page)
                replay = await session.call_tool("plan_pinax_scan", request)
                assert replay.isError is True
                request["intent"] = json.dumps({"purpose": "marketing", "limit": 10})
                tampered = await session.call_tool("discover_pinax_tables", request)
                assert tampered.isError is True
                denied = await session.call_tool("discover_pinax_tables", arguments({
                    "purpose": "marketing", "limit": 10,
                }))
                assert denied.isError is False
                assert denied.structuredContent["tables"] == []
                return page

    direct = asyncio.run(exercise(StdioServerParameters(
        command=binary, args=["mcp-serve", "--registry-config", str(config)],
    )))
    bridge = asyncio.run(exercise(StdioServerParameters(
        command=sys.executable,
        args=["-m", "querygraph", "mcp-serve", "--rust-backend", binary,
              "--registry-config", str(config)],
    )))
    assert direct == bridge


def test_stateless_rust_and_handoff_conform_to_official_2026_schema():
    """Independent wire validation; production dispatch remains entirely Rust."""
    from jsonschema import Draft202012Validator

    configured = os.environ.get("QG_RUST_MCP_BIN")
    if not configured:
        pytest.skip("set QG_RUST_MCP_BIN to the built Rust querygraph executable")
    binary = str(Path(configured).resolve(strict=True))
    schema = json.loads((Path(__file__).parent / "fixtures/mcp-2026-07-28/schema.json").read_text())

    def validate(value, definition):
        Draft202012Validator({"$ref": f"#/$defs/{definition}", "$defs": schema["$defs"]}).validate(value)

    async def exercise(command):
        process = await asyncio.create_subprocess_exec(
            *command, stdin=asyncio.subprocess.PIPE, stdout=asyncio.subprocess.PIPE,
        )
        counter = 0

        async def call(method, params, definition):
            nonlocal counter
            counter += 1
            params = dict(params, _meta={
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": {},
            })
            message = {"jsonrpc": "2.0", "id": counter, "method": method, "params": params}
            validate(message, "JSONRPCRequest")
            process.stdin.write((json.dumps(message) + "\n").encode())
            await process.stdin.drain()
            response = json.loads(await asyncio.wait_for(process.stdout.readline(), timeout=10))
            assert response["id"] == counter
            validate(response, "JSONRPCResultResponse")
            validate(response["result"], definition)
            return response["result"]

        try:
            discovery = await call("server/discover", {}, "DiscoverResult")
            assert discovery["supportedVersions"] == ["2026-07-28"]
            listed = await call("tools/list", {}, "ListToolsResult")
            assert any(tool["name"] == "discover_pinax_ontology" for tool in listed["tools"])
            for tool in listed["tools"]:
                Draft202012Validator.check_schema(tool["inputSchema"])
                if "outputSchema" in tool:
                    Draft202012Validator.check_schema(tool["outputSchema"])
            imported = await call("tools/call", {
                "name": "import_semantic_model",
                "arguments": {"croissant": {"name": "Isolated", "recordSet": [
                    {"field": [{"name": "private_marker"}]}]}}
            }, "CallToolResult")
            handle = imported["structuredContent"]["model_handle"]
            empty = await call("tools/call", {
                "name": "search_semantic_models", "arguments": {"term": "private_marker"}
            }, "CallToolResult")
            assert empty["structuredContent"]["matches"] == []
            selected = await call("tools/call", {
                "name": "search_semantic_models",
                "arguments": {"term": "private_marker", "model_handles": [handle]}
            }, "CallToolResult")
            assert selected["structuredContent"]["matches"]
            return discovery, listed
        finally:
            process.stdin.close()
            try:
                await asyncio.wait_for(process.wait(), timeout=10)
            except TimeoutError:
                process.kill()
                await process.wait()
                raise
            assert process.returncode == 0

    direct = asyncio.run(exercise([binary, "mcp-serve"]))
    handoff = asyncio.run(exercise([sys.executable, "-m", "querygraph", "mcp-serve", "--rust-backend", binary]))
    assert direct == handoff
