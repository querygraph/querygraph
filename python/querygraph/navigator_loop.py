"""The governed navigator loop (FABLE-REVIEW-1 P1-8).

question → semantic-model search → RBAC+ODRL gate (receipts; denials are
first-class) → SQL plan over governed Sail sources → synthesis (any LLM via a
pluggable callable, or the deterministic baseline) → a signed TypeDID envelope
carrying the answer, plus the OpenLineage event and Ed25519 attestation.

The LLM is deliberately just `Callable[[str], str]`: bind LangChain,
PydanticAI, or any OpenAI-compatible server (`openai_compatible_llm`) without
QueryGraph growing a provider layer. With `llm=None` the loop synthesizes
deterministically — the golden baseline the qglake story established.
"""

from __future__ import annotations

import re
from typing import Any, Callable

from pydantic import BaseModel, Field

from querygraph.lineage import LineageAttestation, OpenLineageRunEvent
from querygraph.odrl import Action
from querygraph.odrl_rights import OdrlRightsLayer
from querygraph.osi import OsiDocument, OsiSemanticModel
from querygraph.typedid import AccessReceipt, TypeDidAgent, TypeDidEnvelope

_STOPWORDS = {
    "a", "an", "and", "are", "by", "do", "does", "for", "how", "in", "is",
    "of", "on", "or", "the", "to", "what", "where", "which", "who", "why",
}


class PlannedQuery(BaseModel):
    dataset: str
    source: str
    sql: str
    metric: str | None = None


class NavigatorAnswer(BaseModel):
    question: str
    answer: str
    synthesized_by: str
    matches: list[dict[str, str]] = Field(default_factory=list)
    plans: list[PlannedQuery] = Field(default_factory=list)
    receipts: list[AccessReceipt] = Field(default_factory=list)
    denied_sources: list[str] = Field(default_factory=list)
    envelope: TypeDidEnvelope
    openlineage: dict[str, Any]
    attestation: dict[str, Any]


def openai_compatible_llm(
    base_url: str,
    model: str,
    *,
    api_key: str | None = None,
    timeout: float = 120.0,
) -> Callable[[str], str]:
    """Bind any OpenAI-compatible chat endpoint (Ollama, vLLM, llama.cpp,
    LM Studio, OpenRouter, …) as the loop's synthesizer. Stdlib-only."""
    import json
    import urllib.request

    endpoint = f"{base_url.rstrip('/')}/v1/chat/completions"

    def call(prompt: str) -> str:
        request = urllib.request.Request(
            endpoint,
            data=json.dumps(
                {"model": model, "messages": [{"role": "user", "content": prompt}]}
            ).encode(),
            headers={
                "Content-Type": "application/json",
                **({"Authorization": f"Bearer {api_key}"} if api_key else {}),
            },
            method="POST",
        )
        with urllib.request.urlopen(request, timeout=timeout) as response:
            body = json.loads(response.read())
        return body["choices"][0]["message"]["content"]

    return call


class GovernedNavigatorLoop:
    def __init__(
        self,
        document: OsiDocument,
        rights: OdrlRightsLayer,
        *,
        llm: Callable[[str], str] | None = None,
        llm_name: str = "deterministic",
        agent_name: str = "NavigatorAgent",
        principal: str | None = None,
    ) -> None:
        self.document = document
        self.rights = rights
        self.llm = llm
        self.llm_name = llm_name if llm is not None else "deterministic"
        self.agent = TypeDidAgent.new(agent_name)
        self.principal = principal or self.agent.did.id

    def answer(self, question: str) -> NavigatorAnswer:
        model = self.document.semantic_model
        matches = self._search(model, question)
        plans, receipts, denied = self._plan_governed(model, matches)
        prompt = self._prompt(question, matches, plans, denied)
        if self.llm is not None:
            answer_text = self.llm(prompt)
        else:
            answer_text = self._deterministic_synthesis(question, plans, denied)

        supervisor = TypeDidAgent.new("SupervisorAgent")
        envelope = self.agent.request(
            supervisor,
            action="answer",
            resource=f"model:{model.name}",
            payload={
                "question": question,
                "answer": answer_text,
                "synthesizedBy": self.llm_name,
                "plans": [plan.model_dump() for plan in plans],
                "receipts": [
                    receipt.model_dump(mode="json") for receipt in receipts
                ],
                "deniedSources": denied,
            },
        )
        event = OpenLineageRunEvent.for_agent_run(
            request=envelope,
            job_name="qg-python-navigator-loop",
            inputs=[plan.source for plan in plans] + denied,
            outputs=[f"querygraph:answer:{envelope.payload_sha256[:16]}"],
        )
        attestation = LineageAttestation.from_event(
            issuer=self.agent.did.id,
            subject=f"querygraph:answer:{envelope.payload_sha256[:16]}",
            event_hash=event.event_hash(),
            signer=self.agent.signer,
        )
        return NavigatorAnswer(
            question=question,
            answer=answer_text,
            synthesized_by=self.llm_name,
            matches=matches,
            plans=plans,
            receipts=receipts,
            denied_sources=denied,
            envelope=envelope,
            openlineage=event.model_dump(mode="json"),
            attestation=attestation.model_dump(mode="json"),
        )

    def _search(
        self, model: OsiSemanticModel, question: str
    ) -> list[dict[str, str]]:
        tokens = [
            term
            for term in re.findall(r"[a-z0-9_]+", question.lower())
            if term not in _STOPWORDS and len(term) > 2
        ]
        # Multi-word synonyms ("budget headroom") match via adjacent bigrams.
        terms = tokens + [
            f"{first} {second}" for first, second in zip(tokens, tokens[1:])
        ]
        seen: set[tuple[str, ...]] = set()
        matches: list[dict[str, str]] = []
        for term in terms:
            for match in model.find_by_synonym(term) + self._contains(model, term):
                key = tuple(sorted(match.items()))
                if key not in seen:
                    seen.add(key)
                    matches.append(match)
        return matches

    @staticmethod
    def _contains(model: OsiSemanticModel, term: str) -> list[dict[str, str]]:
        matches = []
        for dataset in model.datasets:
            haystack = " ".join(
                filter(None, [dataset.name, dataset.description])
            ).lower()
            if term in haystack:
                matches.append({"kind": "dataset", "name": dataset.name})
            for field in dataset.fields:
                if term in field.name.lower() or (
                    field.description and term in field.description.lower()
                ):
                    matches.append(
                        {"kind": "field", "name": field.name, "dataset": dataset.name}
                    )
        for metric in model.metrics:
            haystack = " ".join(
                filter(None, [metric.name, metric.description])
            ).lower()
            if term in haystack:
                matches.append({"kind": "metric", "name": metric.name})
        return matches

    def _plan_governed(
        self, model: OsiSemanticModel, matches: list[dict[str, str]]
    ) -> tuple[list[PlannedQuery], list[AccessReceipt], list[str]]:
        matched_datasets = {m["name"] for m in matches if m["kind"] == "dataset"}
        matched_datasets |= {m["dataset"] for m in matches if m["kind"] == "field"}
        matched_fields = {
            (m["dataset"], m["name"]) for m in matches if m["kind"] == "field"
        }
        matched_metrics = [m["name"] for m in matches if m["kind"] == "metric"]

        plans: list[PlannedQuery] = []
        receipts: list[AccessReceipt] = []
        denied: list[str] = []
        allowed_sources: list[str] = []
        for dataset in model.datasets:
            if dataset.name not in matched_datasets:
                continue
            decision = self.rights.decide(self.principal, dataset.source, Action.READ)
            receipts.append(decision.receipt)
            if not decision.allowed:
                denied.append(dataset.source)
                continue
            allowed_sources.append(dataset.source)
            columns = [
                (field.expression.for_dialect("SAIL_SQL") if field.expression else None)
                or f"`{field.name}`"
                for field in dataset.fields
                if not matched_fields or (dataset.name, field.name) in matched_fields
            ] or ["*"]
            plans.append(
                PlannedQuery(
                    dataset=dataset.name,
                    source=dataset.source,
                    sql=f"SELECT {', '.join(columns)} FROM {dataset.source}",
                )
            )
        # A metric-only match still needs a governed source: gate remaining
        # datasets until one allows, so the receipt trail stays complete.
        if matched_metrics and not allowed_sources:
            for dataset in model.datasets:
                if dataset.name in matched_datasets:
                    continue
                decision = self.rights.decide(
                    self.principal, dataset.source, Action.READ
                )
                receipts.append(decision.receipt)
                if decision.allowed:
                    allowed_sources.append(dataset.source)
                    break
                denied.append(dataset.source)
        for metric_name in matched_metrics:
            try:
                expression = model.resolve_metric(metric_name, "SAIL_SQL")
            except KeyError:
                continue
            source = allowed_sources[0] if allowed_sources else None
            if source is None:
                continue
            plans.append(
                PlannedQuery(
                    dataset=metric_name,
                    source=source,
                    sql=f"SELECT {expression} FROM {source}",
                    metric=metric_name,
                )
            )
        return plans, receipts, denied

    def _prompt(
        self,
        question: str,
        matches: list[dict[str, str]],
        plans: list[PlannedQuery],
        denied: list[str],
    ) -> str:
        model = self.document.semantic_model
        lines = [
            "Answer using only the governed semantic context below.",
            f"Question: {question}",
            "",
            f"Semantic model: {model.name}",
        ]
        if model.ai_context and model.ai_context.instructions:
            lines.append(f"Instructions: {model.ai_context.instructions}")
        if matches:
            lines.append("Matched semantics:")
            lines.extend(f"- {m['kind']}: {m['name']}" for m in matches)
        if plans:
            lines.append("Governed SQL plans (allowed under RBAC+ODRL):")
            lines.extend(f"- {plan.sql}" for plan in plans)
        if denied:
            lines.append(
                "Denied sources (policy receipts issued; do NOT use): "
                + ", ".join(denied)
            )
        return "\n".join(lines)

    @staticmethod
    def demo(
        *,
        llm: Callable[[str], str] | None = None,
        llm_name: str = "deterministic",
    ) -> "GovernedNavigatorLoop":
        """The Resilience Desk demo loop: finance and energy readable, the
        restricted health compartment denied with a receipt."""
        from querygraph.odrl import Policy, Rule
        from querygraph.rbac import RbacPolicy, RoleGrant, RolePermission

        document = OsiDocument.from_mapping(
            {
                "semantic_model": {
                    "name": "resilience_desk",
                    "description": "Resilience Desk demo semantics",
                    "ai_context": {
                        "instructions": "Prefer governed Sail columns.",
                        "synonyms": ["resilience"],
                    },
                    "datasets": [
                        {
                            "name": "county_finance",
                            "source": "sail.qg_lakehouse.government_finance__countydata",
                            "description": "County fiscal capacity",
                            "ai_context": {"synonyms": ["fiscal", "budgets"]},
                            "fields": [
                                {
                                    "name": "total_revenue",
                                    "description": "Total county revenue",
                                    "expression": {
                                        "dialects": [
                                            {
                                                "dialect": "SAIL_SQL",
                                                "expression": "`total_revenue`",
                                            }
                                        ]
                                    },
                                }
                            ],
                        },
                        {
                            "name": "energy_burden",
                            "source": "sail.qg_lakehouse.access_2018__access_data",
                            "description": "Household energy burden",
                            "ai_context": {"synonyms": ["energy"]},
                            "fields": [
                                {
                                    "name": "monthly_cost",
                                    "description": "Monthly energy cost",
                                }
                            ],
                        },
                        {
                            "name": "restricted_health",
                            "source": "sail.qg_lakehouse.haalsi_baseline__restricted_raw",
                            "description": "Restricted health rows",
                            "ai_context": {"synonyms": ["health"]},
                        },
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
                                    }
                                ]
                            },
                        }
                    ],
                }
            }
        )
        principal = "did:example:qg-navigator"
        readable = [
            "sail.qg_lakehouse.government_finance__countydata",
            "sail.qg_lakehouse.access_2018__access_data",
        ]
        rights = OdrlRightsLayer(
            rbac=RbacPolicy(
                grants=[RoleGrant(principal=principal, role="navigator")],
                permissions=[
                    RolePermission(
                        role="navigator", resource=source, action=Action.READ.value
                    )
                    for source in readable
                ],
            ),
            odrl=Policy(
                id="urn:querygraph:policy:navigator-demo",
                target="sail.qg_lakehouse",
                assigner="did:example:qg-issuer",
                permissions=[Rule(action=Action.READ, assignee=principal)],
                prohibitions=[Rule(action=Action.DERIVE, assignee=principal)],
            ),
        )
        return GovernedNavigatorLoop(
            document, rights, llm=llm, llm_name=llm_name, principal=principal
        )

    @staticmethod
    def _deterministic_synthesis(
        question: str, plans: list[PlannedQuery], denied: list[str]
    ) -> str:
        if not plans:
            return (
                f"No governed sources matched {question!r}; "
                "no data may be consulted."
            )
        sources = ", ".join(sorted({plan.source for plan in plans}))
        denial_note = (
            f" Restricted sources were denied with receipts: {', '.join(denied)}."
            if denied
            else ""
        )
        return (
            f"Answerable from governed sources {sources} via "
            f"{len(plans)} planned quer{'y' if len(plans) == 1 else 'ies'}."
            + denial_note
        )
