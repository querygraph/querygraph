from __future__ import annotations

import pytest

from querygraph import crypto
from querygraph.navigator_loop import GovernedNavigatorLoop


def test_deterministic_loop_plans_only_allowed_sources():
    loop = GovernedNavigatorLoop.demo()

    result = loop.answer(
        "Where do fiscal capacity and energy burden overlap with health risk?"
    )

    planned_sources = {plan.source for plan in result.plans}
    assert "sail.qg_lakehouse.government_finance__countydata" in planned_sources
    assert "sail.qg_lakehouse.access_2018__access_data" in planned_sources
    # The restricted compartment is denied with a receipt, never planned.
    assert "sail.qg_lakehouse.haalsi_baseline__restricted_raw" not in planned_sources
    assert result.denied_sources == ["sail.qg_lakehouse.haalsi_baseline__restricted_raw"]
    denied_receipts = [r for r in result.receipts if not r.allowed]
    assert len(denied_receipts) == 1
    assert result.synthesized_by == "deterministic"
    assert "denied with receipts" in result.answer


def test_metric_plan_resolves_with_dialect_fallback():
    loop = GovernedNavigatorLoop.demo()

    result = loop.answer("What is our budget headroom?")

    metric_plans = [plan for plan in result.plans if plan.metric == "fiscal_capacity"]
    assert metric_plans, "metric synonym must resolve to a plan"
    assert "SUM(total_revenue - mandated_spend)" in metric_plans[0].sql


def test_llm_synthesis_receives_governed_prompt():
    captured: dict[str, str] = {}

    def fake_llm(prompt: str) -> str:
        captured["prompt"] = prompt
        return "Overlap is highest in counties with weak budgets and high bills."

    loop = GovernedNavigatorLoop.demo(llm=fake_llm, llm_name="fake:test")
    result = loop.answer("Where do fiscal and energy stress overlap with health?")

    assert result.answer.startswith("Overlap is highest")
    assert result.synthesized_by == "fake:test"
    prompt = captured["prompt"]
    assert "Governed SQL plans" in prompt
    assert "haalsi_baseline__restricted_raw" in prompt  # named as denied
    assert "do NOT use" in prompt


def test_answer_carries_verifiable_evidence_chain():
    loop = GovernedNavigatorLoop.demo()
    result = loop.answer("How is fiscal capacity trending?")

    assert result.envelope.verify_payload()
    if crypto.CRYPTO_AVAILABLE:
        assert result.envelope.verify_signature()
    assert result.envelope.payload["deniedSources"] == result.denied_sources
    assert result.openlineage["eventType"] == "COMPLETE"

    jsonschema = pytest.importorskip("jsonschema")  # noqa: F841
    from querygraph.validation import validate_openlineage_schema

    assert validate_openlineage_schema(result.openlineage) == []


def test_unmatched_question_yields_no_plans():
    loop = GovernedNavigatorLoop.demo()
    result = loop.answer("Tell me about quasar luminosity.")

    assert result.plans == []
    assert "No governed sources matched" in result.answer
