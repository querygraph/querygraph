from __future__ import annotations

import pytest

from querygraph.osi import OsiDocument


OSI_UPSTREAM_STYLE = {
    "version": "0.2.0.dev0",
    "semantic_model": [
        {
            "name": "resilience_model",
            "description": "Resilience desk semantics",
            "ai_context": {
                "instructions": "Prefer governed Sail columns.",
                "synonyms": ["resilience desk"],
                "examples": ["Where do fiscal stress and energy burden overlap?"],
            },
            "datasets": [
                {
                    "name": "county_finance",
                    "source": "sail.qg_lakehouse.government_finance__countydata",
                    "primary_key": ["county_fips"],
                    "unique_keys": [["county_fips", "fiscal_year"]],
                    "ai_context": {"synonyms": ["county budgets", "fiscal data"]},
                    "fields": [
                        {
                            "name": "total_revenue",
                            "description": "Total county revenue",
                            "ai_context": {"synonyms": ["revenue", "income"]},
                            "expression": {
                                "dialects": [
                                    {"dialect": "ANSI_SQL", "expression": "total_revenue"},
                                    {"dialect": "SAIL_SQL", "expression": "`total_revenue`"},
                                ]
                            },
                        }
                    ],
                }
            ],
            "relationships": [
                {
                    "name": "finance_to_energy",
                    "from": "county_finance",
                    "to": "energy_burden",
                    "from_columns": ["county_fips"],
                    "to_columns": ["fips"],
                }
            ],
            "metrics": [
                {
                    "name": "fiscal_capacity",
                    "description": "Revenue minus mandated spending",
                    "ai_context": {"synonyms": ["budget headroom"]},
                    "expression": {
                        "dialects": [
                            {
                                "dialect": "ANSI_SQL",
                                "expression": "SUM(total_revenue - mandated_spend)",
                            },
                            {
                                "dialect": "SNOWFLAKE",
                                "expression": "SUM(total_revenue - mandated_spend) /* sf */",
                            },
                        ]
                    },
                }
            ],
        }
    ],
}


def test_loads_upstream_style_document_with_model_list():
    document = OsiDocument.from_mapping(OSI_UPSTREAM_STYLE)
    model = document.semantic_model

    assert model.name == "resilience_model"
    assert model.ai_context.instructions == "Prefer governed Sail columns."
    assert model.ai_context.synonyms == ["resilience desk"]
    assert model.datasets[0].primary_key == ["county_fips"]
    assert model.datasets[0].unique_keys == [["county_fips", "fiscal_year"]]
    assert model.relationships[0].from_dataset == "county_finance"
    assert model.relationships[0].to_columns == ["fips"]


def test_string_ai_context_normalizes_to_instructions():
    document = OsiDocument.from_mapping(
        {
            "semantic_model": {
                "name": "m",
                "ai_context": "Use ontology terms first.",
                "datasets": [],
            }
        }
    )

    assert document.semantic_model.ai_context.instructions == "Use ontology terms first."


def test_resolve_metric_with_dialect_fallback():
    model = OsiDocument.from_mapping(OSI_UPSTREAM_STYLE).semantic_model

    assert model.resolve_metric("fiscal_capacity", "SNOWFLAKE").endswith("/* sf */")
    # SAIL_SQL is absent for this metric; falls back to ANSI_SQL.
    assert (
        model.resolve_metric("fiscal_capacity", "SAIL_SQL")
        == "SUM(total_revenue - mandated_spend)"
    )
    with pytest.raises(KeyError):
        model.resolve_metric("missing_metric", "ANSI_SQL")


def test_find_by_synonym_matches_datasets_fields_and_metrics():
    model = OsiDocument.from_mapping(OSI_UPSTREAM_STYLE).semantic_model

    assert {"kind": "dataset", "name": "county_finance"} in model.find_by_synonym(
        "county budgets"
    )
    assert any(m["kind"] == "field" for m in model.find_by_synonym("revenue"))
    assert any(m["kind"] == "metric" for m in model.find_by_synonym("budget headroom"))
    assert model.find_by_synonym("nonexistent") == []
