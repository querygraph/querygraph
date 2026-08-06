from __future__ import annotations

from pathlib import Path
from typing import Any

from pydantic import BaseModel, Field, field_validator

from querygraph.croissant import CroissantDataset

# Dialects from open-semantic-interchange/OSI core-spec, plus QueryGraph's
# governed Sail SQL dialect.
SUPPORTED_DIALECTS = {
    "ANSI_SQL",
    "SAIL_SQL",
    "SNOWFLAKE",
    "MDX",
    "TABLEAU",
    "DATABRICKS",
    "MAQL",
}

# Dialects tried, in order, when a requested dialect has no expression.
FALLBACK_DIALECTS = ("ANSI_SQL", "SAIL_SQL")


class OsiAiContext(BaseModel):
    """OSI ai_context: LLM-facing instructions, synonyms, and examples.

    OSI documents may spell ai_context as a bare string; it is normalized to
    the structured form with the string as `instructions`.
    """

    instructions: str | None = None
    synonyms: list[str] = Field(default_factory=list)
    examples: list[str] = Field(default_factory=list)

    @classmethod
    def coerce(cls, value: Any) -> "OsiAiContext | None":
        if value is None or isinstance(value, OsiAiContext):
            return value
        if isinstance(value, str):
            return cls(instructions=value)
        return cls.model_validate(value)


def _ai_context_validator(value: Any) -> Any:
    return OsiAiContext.coerce(value)


class OsiDialectExpression(BaseModel):
    dialect: str
    expression: str


class OsiExpression(BaseModel):
    dialects: list[OsiDialectExpression] = Field(default_factory=list)

    def for_dialect(self, dialect: str) -> str | None:
        """Expression in `dialect`, falling back to ANSI_SQL then SAIL_SQL."""
        by_dialect = {entry.dialect: entry.expression for entry in self.dialects}
        for candidate in (dialect, *FALLBACK_DIALECTS):
            if candidate in by_dialect:
                return by_dialect[candidate]
        return None


class OsiField(BaseModel):
    name: str
    description: str | None = None
    semantic_type: str | None = None
    expression: OsiExpression | None = None
    ai_context: OsiAiContext | None = None
    is_time_dimension: bool = False

    _coerce_ai_context = field_validator("ai_context", mode="before")(
        _ai_context_validator
    )

    @property
    def synonyms(self) -> list[str]:
        return self.ai_context.synonyms if self.ai_context else []


class OsiRelationship(BaseModel):
    name: str
    from_dataset: str = Field(alias="from")
    to_dataset: str = Field(alias="to")
    from_columns: list[str] = Field(default_factory=list)
    to_columns: list[str] = Field(default_factory=list)

    model_config = {"populate_by_name": True}


class OsiDataset(BaseModel):
    name: str
    source: str
    description: str | None = None
    ai_context: OsiAiContext | None = None
    primary_key: list[str] = Field(default_factory=list)
    unique_keys: list[list[str]] = Field(default_factory=list)
    fields: list[OsiField] = Field(default_factory=list)

    _coerce_ai_context = field_validator("ai_context", mode="before")(
        _ai_context_validator
    )

    @property
    def synonyms(self) -> list[str]:
        return self.ai_context.synonyms if self.ai_context else []


class OsiMetric(BaseModel):
    name: str
    expression: OsiExpression
    description: str | None = None
    ai_context: OsiAiContext | None = None

    _coerce_ai_context = field_validator("ai_context", mode="before")(
        _ai_context_validator
    )

    @property
    def synonyms(self) -> list[str]:
        return self.ai_context.synonyms if self.ai_context else []


class OsiOntologyTerm(BaseModel):
    id: str
    label: str
    source: str | None = None


class OsiSemanticModel(BaseModel):
    name: str
    description: str | None = None
    ai_context: OsiAiContext | None = None
    datasets: list[OsiDataset] = Field(default_factory=list)
    relationships: list[OsiRelationship] = Field(default_factory=list)
    metrics: list[OsiMetric] = Field(default_factory=list)
    ontology_terms: list[OsiOntologyTerm] = Field(default_factory=list)
    custom_extensions: list[dict[str, Any]] = Field(default_factory=list)

    _coerce_ai_context = field_validator("ai_context", mode="before")(
        _ai_context_validator
    )

    def resolve_metric(self, name: str, dialect: str = "SAIL_SQL") -> str:
        """The expression for metric `name` in `dialect`, with standard fallback."""
        for metric in self.metrics:
            if metric.name != name:
                continue
            expression = metric.expression.for_dialect(dialect)
            if expression is not None:
                return expression
        raise KeyError(f"No metric {name!r} with an expression for dialect {dialect!r}")

    def find_by_synonym(self, term: str) -> list[dict[str, str]]:
        """Datasets, fields, and metrics whose name or synonyms match `term`."""
        needle = term.strip().lower()
        matches: list[dict[str, str]] = []
        for dataset in self.datasets:
            if needle == dataset.name.lower() or needle in (
                synonym.lower() for synonym in dataset.synonyms
            ):
                matches.append({"kind": "dataset", "name": dataset.name})
            for field in dataset.fields:
                if needle == field.name.lower() or needle in (
                    synonym.lower() for synonym in field.synonyms
                ):
                    matches.append(
                        {"kind": "field", "name": field.name, "dataset": dataset.name}
                    )
        for metric in self.metrics:
            if needle == metric.name.lower() or needle in (
                synonym.lower() for synonym in metric.synonyms
            ):
                matches.append({"kind": "metric", "name": metric.name})
        return matches


class OsiDocument(BaseModel):
    version: str = "0.2.0.dev0"
    semantic_model: OsiSemanticModel

    @classmethod
    def from_mapping(cls, value: dict[str, Any]) -> "OsiDocument":
        # Upstream OSI documents spell `semantic_model` as a list of models;
        # QueryGraph documents carry a single model. Accept both.
        semantic_model = value.get("semantic_model")
        if isinstance(semantic_model, list):
            if not semantic_model:
                raise ValueError("semantic_model[] is empty")
            if len(semantic_model) > 1:
                raise ValueError("Multi-model OSI documents are not supported")
            value = {**value, "semantic_model": semantic_model[0]}
        return cls.model_validate(value)

    @classmethod
    def from_yaml_file(cls, path: str | Path) -> "OsiDocument":
        try:
            import yaml
        except ImportError as exc:  # pragma: no cover - exercised by users.
            raise RuntimeError("Install PyYAML to load OSI YAML files.") from exc
        return cls.from_mapping(yaml.safe_load(Path(path).read_text()))

    @classmethod
    def from_croissant(
        cls,
        dataset: CroissantDataset,
        *,
        model_name: str | None = None,
        sail_schema: str = "qg_lakehouse",
    ) -> "OsiDocument":
        fields = [
            OsiField(
                name=field.name,
                description=field.description,
                semantic_type=field.semantic_type_value,
                expression=OsiExpression(
                    dialects=[
                        OsiDialectExpression(
                            dialect="SAIL_SQL",
                            expression=f"`{field.name}`",
                        )
                    ]
                ),
            )
            for record_set in dataset.record_sets
            for field in record_set.fields
        ]
        terms = [
            OsiOntologyTerm(
                id=field.semantic_type_value,
                label=field.name,
                source="semantic-croissant",
            )
            for record_set in dataset.record_sets
            for field in record_set.fields
            if field.semantic_type_value
        ]
        safe_name = _safe_sql_name(dataset.name)
        return cls(
            semantic_model=OsiSemanticModel(
                name=model_name or f"{safe_name}_semantic_model",
                description=f"OSI model derived from Semantic Croissant dataset {dataset.name}.",
                ai_context=(
                    "Resolve user intent to ontology terms, then map those terms "
                    "to Croissant fields and governed Sail columns."
                ),
                datasets=[
                    OsiDataset(
                        name=safe_name,
                        source=f"sail.{sail_schema}.{safe_name}",
                        description=dataset.description,
                        ai_context=(
                            f"Dataset {dataset.name} has {len(dataset.files)} file(s) "
                            f"and {len(fields)} semantic field(s)."
                        ),
                        fields=fields,
                    )
                ],
                metrics=[
                    OsiMetric(
                        name="row_count",
                        description="Count of governed rows available in Sail.",
                        expression=OsiExpression(
                            dialects=[
                                OsiDialectExpression(
                                    dialect="SAIL_SQL",
                                    expression="COUNT(*)",
                                )
                            ]
                        ),
                        ai_context="Use this metric to verify loaded table scale.",
                    )
                ],
                ontology_terms=terms,
            )
        )

    def to_json(self) -> dict[str, Any]:
        return self.model_dump(mode="json", exclude_none=True)


def _safe_sql_name(name: str) -> str:
    out = "".join(ch.lower() if ch.isalnum() else "_" for ch in name)
    out = "_".join(part for part in out.split("_") if part)
    return out or "dataset"
