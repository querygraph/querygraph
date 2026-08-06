from __future__ import annotations

from querygraph.a2a import A2A_PROTOCOL_VERSION, SKILLS, build_agent_card


def test_agent_card_shape():
    card = build_agent_card("http://example.com/")

    assert card["protocolVersion"] == A2A_PROTOCOL_VERSION
    assert card["url"] == "http://example.com/v1"
    assert card["preferredTransport"] == "HTTP+JSON"
    assert [skill["id"] for skill in card["skills"]] == [
        "navigator-bundle",
        "qglake-story",
        "verify-envelope",
        "import-semantic-model",
        "semantic-search",
    ]
    scheme = card["securitySchemes"]["typedid"]
    assert "querygraph-typedid-signing-v1" in scheme["description"]


def test_every_skill_documents_itself():
    for skill in SKILLS:
        assert skill["id"] and skill["name"] and skill["description"]
        assert skill["tags"], f"skill {skill['id']} has no tags"
        assert skill["examples"], f"skill {skill['id']} has no examples"
