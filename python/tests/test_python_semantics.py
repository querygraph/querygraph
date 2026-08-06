from querygraph import AiNavigator, NavigatorInput
from querygraph.did import DidDocument
from querygraph.odrl import Action, Policy, Rule


def test_builds_all_four_semantic_layers():
    output = AiNavigator().build(
        NavigatorInput(
            dataset_name="Hazard vocabulary",
            description="Controlled vocabulary with multilingual technical terms",
            landing_page="https://querygraph.ai/datasets/hazards",
            data_url="https://querygraph.ai/datasets/hazards.csv",
            creator="QueryGraph",
            agent_name="AI Navigator",
        )
    )

    assert output.croissant["@type"] == "cr:Dataset"
    assert output.cdif["@type"] == "dcat:Dataset"
    assert output.did.id.startswith("did:oyd:zQm")
    assert output.odrl["@type"] == "odrl:Policy"


def test_did_generation_matches_known_rust_output():
    did = DidDocument.new_oyd(
        "AI Navigator:QueryGraph:Hazard vocabulary", "AI Navigator"
    )

    assert did.id == "did:oyd:zQmciWcCbpqbsYcNPVzdQ4YqznbAK9kRsnNRDdXg5Z73qCe"
    assert did.public_key_multibase == "zFNru6TqKpt4ymk5pbCM5Bq4beqJ9nEDgLucfyv9nr98e"


def test_policy_allows_permissions_without_matching_prohibition():
    policy = Policy(
        id="policy",
        target="dataset",
        assigner="did:example:agent",
        permissions=[
            Rule(action=Action.READ, assignee="public"),
            Rule(action=Action.INDEX, assignee="did:example:agent"),
        ],
        prohibitions=[Rule(action=Action.DERIVE, assignee="public")],
    )

    assert policy.allows("public", Action.READ)
    assert not policy.allows("public", Action.DERIVE)
    assert not policy.allows("public", Action.INDEX)
