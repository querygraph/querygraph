"""Agent2Agent (A2A) protocol surface.

Publishes the QueryGraph Agent Card — the same card qg-rust serves at
`/.well-known/agent-card.json` — so A2A clients can discover the governed
semantic layer's skills and its TypeDID security contract. The skill list is a
cross-language contract asserted by the Rust equivalence suite.
"""

from __future__ import annotations

from importlib import metadata
from typing import Any

A2A_PROTOCOL_VERSION = "0.3.0"

SKILLS: list[dict[str, Any]] = [
    {
        "id": "navigator-bundle",
        "name": "Build semantic bundle",
        "description": (
            "Project a dataset into the four governed layers: Semantic "
            "Croissant, CDIF, DID, and ODRL."
        ),
        "tags": ["semantic-layer", "croissant", "cdif", "did", "odrl"],
        "examples": ["Build a semantic bundle for the hazard vocabulary dataset."],
    },
    {
        "id": "qglake-story",
        "name": "Governed multi-agent run",
        "description": (
            "Run the compartmentalized supervisor/specialist/broker/synthesis "
            "story with signed envelopes, policy receipts, and an OpenLineage "
            "evidence chain."
        ),
        "tags": ["governance", "rbac", "odrl", "openlineage", "typedid"],
        "examples": [
            "Where do fiscal capacity, energy burden, mobility disruption, "
            "and climate-health risk overlap?"
        ],
    },
    {
        "id": "verify-envelope",
        "name": "Verify TypeDID envelope",
        "description": (
            "Verify a TypeDID envelope's payload hash and Ed25519 signature "
            "against its did:key verification method."
        ),
        "tags": ["audit", "ed25519", "did", "typedid"],
        "examples": ["Verify this agent response envelope before trusting its summary."],
    },
    {
        "id": "import-semantic-model",
        "name": "Import semantic model",
        "description": (
            "Import an OSI semantic model or a Semantic Croissant document "
            "into the governed model registry."
        ),
        "tags": ["osi", "croissant", "semantic-layer"],
        "examples": ["Import this OSI YAML so agents can resolve its metrics."],
    },
    {
        "id": "semantic-search",
        "name": "Search semantic models",
        "description": (
            "Find datasets, fields, metrics, and ontology terms matching a "
            "business term across registered semantic models."
        ),
        "tags": ["search", "osi", "ontology"],
        "examples": ["Which fields describe monthly energy cost?"],
    },
]


def build_agent_card(base_url: str = "http://localhost:8080") -> dict[str, Any]:
    """The QueryGraph A2A Agent Card for a deployment at `base_url`."""
    try:
        version = metadata.version("querygraph")
    except metadata.PackageNotFoundError:  # pragma: no cover - editable installs
        version = "0.0.0"
    base = base_url.rstrip("/")
    return {
        "protocolVersion": A2A_PROTOCOL_VERSION,
        "name": "QueryGraph Navigator",
        "description": (
            "Governed semantic-layer agent: builds four-layer semantic "
            "bundles (Croissant, CDIF, DID, ODRL), answers over "
            "RBAC+ODRL-gated lakehouse data with signed TypeDID envelopes, "
            "and emits an OpenLineage evidence chain anchored by Ed25519 "
            "attestations."
        ),
        "url": f"{base}/v1",
        "preferredTransport": "HTTP+JSON",
        "provider": {"organization": "QueryGraph", "url": "https://querygraph.ai"},
        "version": version,
        "capabilities": {
            "streaming": False,
            "pushNotifications": False,
            "stateTransitionHistory": False,
        },
        "defaultInputModes": ["application/json"],
        "defaultOutputModes": ["application/json"],
        "skills": SKILLS,
        "securitySchemes": {
            "typedid": {
                "type": "http",
                "scheme": "bearer",
                "description": (
                    "TypeDID signed envelope: Ed25519 signature over the "
                    "querygraph-typedid-signing-v1 payload, verifiable "
                    "against the did:key verification method carried in the "
                    "envelope. The sender must equal that signing DID. "
                    "Denials are receipts, not errors."
                ),
            },
        },
    }
