"""Model Context Protocol server for the QueryGraph governed semantic layer.

Exposes the semantic layer to any MCP client — Claude Code/Desktop, OpenAI
Agents SDK, LangChain (`langchain-mcp-adapters`), PydanticAI, LlamaIndex,
CrewAI — without per-framework adapters. Every tool result carries the
governance evidence (access receipts, signed envelopes, payload hashes), and a
policy denial is a first-class result, never an error.

Install with `pip install querygraph[mcp]` and run `querygraph mcp-serve`.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from querygraph import crypto
from querygraph.navigator import AiNavigator, NavigatorInput
from querygraph.odrl import Action, Policy, Rule
from querygraph.odrl_rights import OdrlRightsLayer
from querygraph.osi import OsiDocument
from querygraph.qglake import build_python_qglake_story
from querygraph.rbac import RbacPolicy, RoleGrant, RolePermission
from querygraph.typedid import TypeDidEnvelope

SERVER_NAME = "querygraph"

_ACTION_ALIASES = {action.value.split(":", 1)[1]: action for action in Action}


def parse_action(value: str) -> Action:
    """Accept 'read', 'odrl:read', or 'READ' spellings of an ODRL action."""
    normalized = value.strip()
    for action in Action:
        if normalized in (action.value, action.name):
            return action
    try:
        return _ACTION_ALIASES[normalized.lower()]
    except KeyError as exc:
        known = sorted(_ACTION_ALIASES)
        raise ValueError(f"Unknown ODRL action {value!r}; known: {known}") from exc


def demo_rights_layer() -> OdrlRightsLayer:
    """The Resilience Desk demo policy: read allowed, derive prohibited."""
    principal = "did:example:qg-agent"
    resource = "qg_lakehouse"
    return OdrlRightsLayer(
        rbac=RbacPolicy(
            grants=[RoleGrant(principal=principal, role="navigator")],
            permissions=[
                RolePermission(
                    role="navigator", resource=resource, action=Action.READ.value
                )
            ],
        ),
        odrl=Policy(
            id="urn:querygraph:policy:demo",
            target=resource,
            assigner="did:example:qg-issuer",
            permissions=[Rule(action=Action.READ, assignee=principal)],
            prohibitions=[Rule(action=Action.DERIVE, assignee=principal)],
        ),
    )


def load_rights_layer(path: str | Path) -> OdrlRightsLayer:
    """Load `{"rbac": {...}, "odrl": {...}}` governance config from JSON."""
    return OdrlRightsLayer.model_validate(
        json.loads(Path(path).read_text(encoding="utf-8"))
    )


def create_server(
    *,
    osi_path: str | Path | None = None,
    rights_path: str | Path | None = None,
):
    """Build the FastMCP server; import deferred so the extra stays optional."""
    try:
        from mcp.server.fastmcp import FastMCP
    except ImportError as exc:  # pragma: no cover - depends on optional extra.
        raise RuntimeError(
            "Install querygraph[mcp] to run the MCP server."
        ) from exc

    osi_document = OsiDocument.from_yaml_file(osi_path) if osi_path else None
    rights = load_rights_layer(rights_path) if rights_path else demo_rights_layer()

    server = FastMCP(
        SERVER_NAME,
        instructions=(
            "QueryGraph governed semantic layer. Search the semantic model, "
            "resolve metrics to governed SQL, check RBAC+ODRL access (denials "
            "return receipts, not errors), build four-layer semantic bundles "
            "(Croissant/CDIF/DID/ODRL), and verify TypeDID envelopes."
        ),
    )

    def _require_model() -> OsiDocument:
        if osi_document is None:
            raise ValueError(
                "No OSI semantic model loaded; start the server with --osi."
            )
        return osi_document

    @server.tool()
    def search_semantic_model(term: str) -> dict[str, Any]:
        """Find datasets, fields, and metrics matching a business term or synonym."""
        model = _require_model().semantic_model
        return {"term": term, "matches": model.find_by_synonym(term)}

    @server.tool()
    def resolve_metric(name: str, dialect: str = "SAIL_SQL") -> dict[str, Any]:
        """Resolve an OSI metric to a SQL expression, with ANSI_SQL fallback."""
        model = _require_model().semantic_model
        return {
            "metric": name,
            "dialect": dialect,
            "expression": model.resolve_metric(name, dialect),
        }

    @server.tool()
    def check_access(principal: str, resource: str, action: str) -> dict[str, Any]:
        """RBAC+ODRL dual-gate decision. Denials are receipts, not errors."""
        decision = rights.decide(principal, resource, parse_action(action))
        return decision.model_dump(mode="json")

    @server.tool()
    def build_navigator_bundle(
        dataset_name: str,
        description: str,
        landing_page: str,
        data_url: str,
        creator: str = "QueryGraph",
        agent_name: str = "AI Navigator",
    ) -> dict[str, Any]:
        """Build the four-layer semantic bundle: Croissant, CDIF, DID, ODRL."""
        output = AiNavigator().build(
            NavigatorInput(
                dataset_name=dataset_name,
                description=description,
                landing_page=landing_page,
                data_url=data_url,
                creator=creator,
                agent_name=agent_name,
            )
        )
        return output.bundle

    @server.tool()
    def run_qglake_story() -> dict[str, Any]:
        """Run the governed multi-agent Resilience Desk story with full evidence chain."""
        return build_python_qglake_story()

    @server.tool()
    def answer_question(question: str) -> dict[str, Any]:
        """Run the governed navigator loop: semantic search, RBAC+ODRL gate
        with receipts, SQL plans over allowed sources, synthesized answer in
        a signed envelope with the OpenLineage evidence chain."""
        from querygraph.navigator_loop import GovernedNavigatorLoop

        if osi_document is not None:
            loop = GovernedNavigatorLoop(osi_document, rights)
        else:
            loop = GovernedNavigatorLoop.demo()
        return loop.answer(question).model_dump(mode="json")

    @server.tool()
    def verify_envelope(envelope: dict[str, Any]) -> dict[str, Any]:
        """Verify a TypeDID envelope's payload hash and Ed25519 signature."""
        parsed = TypeDidEnvelope.model_validate(envelope)
        return {
            "payload_hash_valid": parsed.verify_payload(),
            "signed": parsed.is_signed(),
            "signature_valid": (
                parsed.verify_signature()
                if parsed.is_signed() and crypto.CRYPTO_AVAILABLE
                else False
            ),
            "verification_method": parsed.verification_method,
            "crypto_available": crypto.CRYPTO_AVAILABLE,
        }

    @server.resource("qg://story/resilience-desk")
    def resilience_desk_story() -> str:
        """The full governed Resilience Desk story as JSON."""
        return json.dumps(build_python_qglake_story(), sort_keys=True)

    if osi_document is not None:

        @server.resource("qg://models/current")
        def current_semantic_model() -> str:
            """The loaded OSI semantic model as JSON."""
            return json.dumps(osi_document.to_json(), sort_keys=True)

    return server


def serve(
    *,
    osi_path: str | Path | None = None,
    rights_path: str | Path | None = None,
    transport: str = "stdio",
) -> None:
    create_server(osi_path=osi_path, rights_path=rights_path).run(transport=transport)
