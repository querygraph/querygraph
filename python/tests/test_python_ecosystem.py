from __future__ import annotations

import json
from pathlib import Path

from querygraph.croissant import CroissantDataset, Field, FileObject, RecordSet
from querygraph.cdif import CdifResource
from querygraph.dataverse import DataverseDataset
from querygraph.lakehouse import example_queries, load_table_specs
from querygraph.lineage import LineageAttestation, OpenLineageRunEvent
from querygraph.odrl import Action, Policy, Rule
from querygraph.odrl_rights import OdrlRightsLayer
from querygraph.osi import OsiDocument
from querygraph.qglake import build_python_qglake_story
from querygraph.rbac import RbacPolicy, RoleGrant, RolePermission
from querygraph.typedid import TypeDidAgent
from querygraph.validation import validate_cdif, validate_croissant, validate_openlineage


def test_osi_projects_semantic_croissant_fields_to_sail_expressions():
    dataset = CroissantDataset(
        id="https://querygraph.ai/test/#dataset",
        name="Energy Burden",
        description="Demo energy fields",
        license="https://creativecommons.org/licenses/by/4.0/",
        creators=["QueryGraph"],
        files=[
            FileObject(
                id="https://querygraph.ai/test/#file",
                name="energy.parquet",
                content_url="sail://qg_lakehouse/energy",
                encoding_format="application/vnd.apache.parquet",
            )
        ],
        record_sets=[
            RecordSet(
                id="https://querygraph.ai/test/#recordset",
                name="observations",
                fields=[
                    Field("monthly_cost", "sc:Float", "Monthly cost").semantic_type(
                        "https://querygraph.ai/ontology/monthlyEnergyCost"
                    )
                ],
            )
        ],
        keywords=["energy"],
    )

    osi = OsiDocument.from_croissant(dataset)

    assert osi.semantic_model.datasets[0].fields[0].name == "monthly_cost"
    assert osi.semantic_model.datasets[0].fields[0].expression is not None
    assert osi.semantic_model.ontology_terms[0].id.endswith("monthlyEnergyCost")


def test_typedid_agents_create_signed_request_and_response():
    supervisor = TypeDidAgent.new("SupervisorAgent")
    finance = TypeDidAgent.new("FinanceAgent")

    request = supervisor.request(
        finance,
        action="summarize",
        resource="compartment:finance",
        payload={"question": "Where is fiscal stress highest?"},
    )
    response = finance.answer(
        request,
        status="allowed",
        summary="Fiscal stress summary over governed finance tables.",
    )

    assert request.verify_payload()
    assert response.envelope.conversation_id == request.conversation_id
    assert response.envelope.payload["requestSha256"] == request.payload_sha256


def test_lineage_event_and_attestation_from_typedid_request():
    supervisor = TypeDidAgent.new("SupervisorAgent")
    synthesis = TypeDidAgent.new("SynthesisAgent")
    request = supervisor.request(
        synthesis,
        action="aggregate",
        resource="querygraph:briefing",
        payload={"summaryCount": 3},
    )

    event = OpenLineageRunEvent.for_agent_run(
        request=request,
        job_name="python-test",
        inputs=["qg_lakehouse.finance"],
        outputs=["querygraph:briefing"],
    )
    attestation = LineageAttestation.from_event(
        issuer=supervisor.did.id,
        subject="querygraph:briefing",
        event_hash=event.event_hash(),
    )

    assert event.run["facets"]["queryGraph_typeDid"]["payloadSha256"]
    assert attestation.event_hash == event.event_hash()
    # Signed when the crypto extra is present; clearly marked unsigned otherwise.
    assert attestation.signature.startswith(("ed25519:", "unsigned:sha256:"))


def test_lakehouse_manifest_specs_find_parquet_locations(tmp_path: Path):
    warehouse = tmp_path / "spark-warehouse"
    table_dir = warehouse / "government_finance__countydata-123"
    table_dir.mkdir(parents=True)
    manifest = tmp_path / "load-report.json"
    manifest.write_text(
        json.dumps(
            {
                "datasets": [
                    {
                        "files": [
                            {
                                "table": "qg_lakehouse.government_finance__countydata",
                                "rows": 42,
                            }
                        ]
                    }
                ]
            }
        )
    )

    specs = load_table_specs(manifest, warehouse)

    assert specs[0].bare_name == "government_finance__countydata"
    assert specs[0].rows == 42
    assert specs[0].location == table_dir


def test_python_qglake_story_exercises_denial_lineage_and_attestation():
    story = build_python_qglake_story()

    assert story["synthesis"]["denials"]
    assert story["openlineage"]["eventType"] == "COMPLETE"
    assert story["attestation"]["event_hash"]
    assert any("codata_constants" in query for query in example_queries())


def test_odrl_rights_layer_requires_both_rbac_and_odrl():
    principal = "did:example:finance"
    resource = "compartment:finance"
    rights = OdrlRightsLayer(
        rbac=RbacPolicy(
            grants=[RoleGrant(principal=principal, role="finance_reader")],
            permissions=[
                RolePermission(
                    role="finance_reader",
                    resource=resource,
                    action=Action.READ.value,
                )
            ],
        ),
        odrl=Policy(
            id="policy",
            target=resource,
            assigner="did:example:issuer",
            permissions=[Rule(action=Action.READ, assignee=principal)],
            prohibitions=[Rule(action=Action.DERIVE, assignee=principal)],
        ),
    )

    assert rights.decide(principal, resource, Action.READ).allowed
    assert not rights.decide(principal, resource, Action.DERIVE).allowed


def test_dataverse_native_payload_projects_to_croissant_and_validation():
    dataset = DataverseDataset.from_native_api(
        {
            "data": {
                "id": 123,
                "persistentId": "doi:10.7910/DVN/TEST",
                "persistentUrl": "https://doi.org/10.7910/DVN/TEST",
                "latestVersion": {
                    "metadataBlocks": {
                        "citation": {
                            "fields": [
                                {"typeName": "title", "value": "Demo Dataverse Dataset"},
                                {"typeName": "subject", "value": ["Social Sciences"]},
                                {
                                    "typeName": "keyword",
                                    "value": [{"keywordValue": "QueryGraph"}],
                                },
                            ]
                        }
                    },
                    "files": [
                        {
                            "dataFile": {
                                "id": 999,
                                "filename": "demo.csv",
                                "contentType": "text/csv",
                            }
                        }
                    ],
                },
            }
        }
    )
    croissant = dataset.to_croissant().to_json_ld()
    osi = OsiDocument.from_croissant(dataset.to_croissant())

    assert dataset.title == "Demo Dataverse Dataset"
    assert validate_croissant(croissant) == []
    assert osi.semantic_model.datasets[0].fields[0].name == "dataset_persistent_id"


def test_openlineage_event_conforms_to_official_schema():
    jsonschema = __import__("pytest").importorskip("jsonschema")  # noqa: F841
    from querygraph.validation import validate_openlineage_schema

    story = build_python_qglake_story()
    assert validate_openlineage_schema(story["openlineage"]) == []

    # runId is a spec-required UUID, derived deterministically (UUIDv5).
    import uuid

    uuid.UUID(story["openlineage"]["run"]["runId"])

    # A broken event is reported, not silently accepted.
    broken = dict(story["openlineage"], run={"facets": {}})
    assert validate_openlineage_schema(broken)


def test_validators_accept_generated_cdif_and_openlineage():
    output = build_python_qglake_story()
    event_errors = validate_openlineage(output["openlineage"])
    dataset = CroissantDataset(
        id="https://querygraph.ai/test/#dataset",
        name="Validation Dataset",
        description="Validation demo",
        license="https://creativecommons.org/licenses/by/4.0/",
        creators=["QueryGraph"],
        files=[
            FileObject(
                id="https://querygraph.ai/test/#file",
                name="demo.csv",
                content_url="https://querygraph.ai/demo.csv",
                encoding_format="text/csv",
            )
        ],
        record_sets=[
            RecordSet(
                id="https://querygraph.ai/test/#recordset",
                name="demo",
                fields=[Field("value", "sc:Text", "Value")],
            )
        ],
        keywords=[],
    )
    cdif = CdifResource.from_croissant(
        dataset,
        "https://querygraph.ai/test",
        "https://querygraph.ai/demo.csv",
    ).to_json_ld()
    cdif_errors = validate_cdif(cdif)

    assert event_errors == []
    assert cdif_errors == []
