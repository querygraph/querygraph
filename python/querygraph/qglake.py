from __future__ import annotations

from typing import Any

from querygraph.agents import TypeDidAgentRun
from querygraph.lineage import LineageAttestation, OpenLineageRunEvent
from querygraph.typedid import GovernedPrompt, TypeDidAgent


def build_python_qglake_story() -> dict[str, Any]:
    supervisor = TypeDidAgent.new("SupervisorAgent")
    synthesis = TypeDidAgent.new("SynthesisAgent")
    specialists = [
        TypeDidAgent.new("FinanceAgent"),
        TypeDidAgent.new("EnergyAgent"),
        TypeDidAgent.new("MobilityAgent"),
        TypeDidAgent.new("ClimateHealthAgent"),
        TypeDidAgent.new("ReferenceAgent"),
        TypeDidAgent.new("RestrictedDataBroker"),
    ]
    prompt = GovernedPrompt(
        question=(
            "Where do fiscal capacity, energy burden, mobility disruption, "
            "and climate-health risk overlap?"
        ),
        semantic_context={
            "croissant": "semantic/croissant.json sidecars",
            "cdif": "semantic/cdif.json profiles",
            "osi": "business terms mapped to governed Sail columns",
            "sail": "qg_lakehouse typed tables",
        },
        allowed_sources=[
            "qg_lakehouse.government_finance__countydata",
            "qg_lakehouse.access_2018__access_data",
            "qg_lakehouse.dockless_transportation__trips",
            "qg_lakehouse.climate_health_pathways__pathways",
            "qg_lakehouse.codata_constants_2022__codata_constants_2022",
        ],
        denied_sources=["qg_lakehouse.haalsi_baseline__restricted_raw"],
    )

    responses = []
    summaries = {
        "FinanceAgent": "Fiscal capacity summary over county and municipal finance tables.",
        "EnergyAgent": "Energy burden summary from governed ACCESS and COVID insecurity fields.",
        "MobilityAgent": "Mobility disruption summary from dockless trips and injury severity tables.",
        "ClimateHealthAgent": "Climate-health pathway summary with approved aggregate evidence.",
        "ReferenceAgent": "CODATA constants normalize units before synthesis.",
        "RestrictedDataBroker": "Raw restricted health rows denied; metadata-only receipt returned.",
    }
    for specialist in specialists:
        request = supervisor.request(
            specialist,
            action="summarize",
            resource=f"compartment:{specialist.name}",
            payload=prompt.model_dump(mode="json"),
        )
        status = "denied" if specialist.name == "RestrictedDataBroker" else "allowed"
        responses.append(
            specialist.answer(
                request,
                status=status,
                summary=summaries[specialist.name],
                evidence=[f"semantic projection for {specialist.name}"],
                redactions=["restricted raw rows"] if status == "denied" else [],
            )
        )

    run = TypeDidAgentRun(
        supervisor=supervisor,
        specialists=specialists,
        prompt=prompt,
        responses=responses,
    )
    synthesis_request = supervisor.request(
        synthesis,
        action="aggregate",
        resource="querygraph:resilience-briefing",
        payload=run.aggregate(),
    )
    event = OpenLineageRunEvent.for_agent_run(
        request=synthesis_request,
        job_name="qg-python-qglake-story",
        inputs=prompt.allowed_sources + prompt.denied_sources,
        outputs=["querygraph:resilience-briefing"],
    )
    attestation = LineageAttestation.from_event(
        issuer=supervisor.did.id,
        subject="querygraph:resilience-briefing",
        event_hash=event.event_hash(),
        signer=supervisor.signer,
    )
    return {
        "prompt": prompt.model_dump(mode="json"),
        "agents": [agent.model_dump(mode="json") for agent in [supervisor, *specialists, synthesis]],
        "responses": [response.model_dump(mode="json") for response in responses],
        "synthesis": run.aggregate(),
        "openlineage": event.model_dump(mode="json"),
        "attestation": attestation.model_dump(mode="json"),
    }
