from __future__ import annotations

from dataclasses import dataclass
from datetime import UTC, datetime

from querygraph.cdif import CdifResource
from querygraph.croissant import CroissantDataset, Field, FileObject, RecordSet
from querygraph.did import DidDocument
from querygraph.odrl import Action, Policy, Rule


@dataclass(frozen=True)
class NavigatorInput:
    dataset_name: str
    description: str
    landing_page: str
    data_url: str
    creator: str
    agent_name: str


@dataclass(frozen=True)
class NavigatorOutput:
    generated_at: datetime
    croissant: dict
    cdif: dict
    did: DidDocument
    odrl: dict
    bundle: dict


class AiNavigator:
    def build(self, input: NavigatorInput) -> NavigatorOutput:
        did = DidDocument.new_oyd(
            f"{input.agent_name}:{input.creator}:{input.dataset_name}",
            input.agent_name,
        ).with_service_endpoint(input.landing_page)

        dataset_id = f"{input.landing_page.rstrip('/')}/#dataset"
        dataset = CroissantDataset(
            id=dataset_id,
            name=input.dataset_name,
            description=input.description,
            license="https://creativecommons.org/licenses/by/4.0/",
            creators=[input.creator],
            files=[
                FileObject(
                    id=f"{dataset_id}/file/source",
                    name="source-data",
                    content_url=input.data_url,
                    encoding_format="application/octet-stream",
                )
            ],
            record_sets=[
                RecordSet(
                    id=f"{dataset_id}/recordset/default",
                    name="default observations",
                    fields=[
                        Field(
                            "subject",
                            "sc:Text",
                            "Primary entity or observation subject",
                        ).semantic_type("https://schema.org/about"),
                        Field(
                            "value",
                            "sc:Text",
                            "Observed value, label, or narrative",
                        ).semantic_type("https://schema.org/value"),
                        Field(
                            "source",
                            "sc:URL",
                            "Evidence or provenance URL",
                        ).semantic_type("https://schema.org/citation"),
                    ],
                )
            ],
            keywords=["AI Navigator", "Croissant", "CDIF", "DID", "ODRL"],
        )

        policy = Policy(
            id=f"{dataset_id}/policy/default",
            target=dataset_id,
            assigner=did.id,
            permissions=[
                Rule(
                    action=Action.READ,
                    assignee="public",
                    constraint="attribution required",
                ),
                Rule(
                    action=Action.INDEX,
                    assignee=did.id,
                    constraint="local semantic indexing for AI Navigator",
                ),
            ],
            prohibitions=[
                Rule(
                    action=Action.DERIVE,
                    assignee="public",
                    constraint="no model training without separate agreement",
                )
            ],
        )
        odrl_json = policy.to_json_ld()
        cdif = CdifResource.from_croissant(
            dataset, input.landing_page, input.data_url
        ).with_odrl_policy(policy.id, odrl_json)

        croissant_json = dataset.to_json_ld()
        cdif_json = cdif.to_json_ld()
        generated_at = datetime.now(UTC)
        did_json = did.to_json()
        bundle = {
            "@context": {
                "schema": "https://schema.org/",
                "cr": "http://mlcommons.org/croissant/",
                "cdif": "https://cdif.codata.org/",
                "dcat": "http://www.w3.org/ns/dcat#",
                "dct": "http://purl.org/dc/terms/",
                "odrl": "http://www.w3.org/ns/odrl/2/",
                "querygraph": "https://querygraph.ai/ns#",
            },
            "@type": "querygraph:AiNavigatorSemanticBundle",
            "generatedAt": generated_at.isoformat().replace("+00:00", "Z"),
            "identity": did_json,
            "layers": {
                "semanticCroissant": croissant_json,
                "cdif": cdif_json,
                "did": did_json,
                "odrl": odrl_json,
            },
        }

        return NavigatorOutput(
            generated_at=generated_at,
            croissant=croissant_json,
            cdif=cdif_json,
            did=did,
            odrl=odrl_json,
            bundle=bundle,
        )
