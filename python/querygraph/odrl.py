from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class Action(Enum):
    USE = "odrl:use"
    READ = "odrl:read"
    DERIVE = "odrl:derive"
    TRANSLATE = "querygraph:translate"
    INDEX = "querygraph:index"

    def iri(self) -> str:
        return self.value


@dataclass(frozen=True)
class Rule:
    action: Action
    assignee: str
    constraint: str | None = None


@dataclass(frozen=True)
class Policy:
    id: str
    target: str
    assigner: str
    permissions: list[Rule]
    prohibitions: list[Rule]

    def allows(self, assignee: str, action: Action) -> bool:
        prohibited = any(
            rule.assignee == assignee and rule.action == action
            for rule in self.prohibitions
        )
        permitted = any(
            rule.assignee == assignee and rule.action == action
            for rule in self.permissions
        )
        return permitted and not prohibited

    def to_json_ld(self) -> dict:
        return {
            "@type": "odrl:Policy",
            "@id": self.id,
            "odrl:target": self.target,
            "odrl:assigner": self.assigner,
            "odrl:permission": [_rule_json(rule) for rule in self.permissions],
            "odrl:prohibition": [_rule_json(rule) for rule in self.prohibitions],
        }


def _rule_json(rule: Rule) -> dict:
    return {
        "odrl:action": rule.action.iri(),
        "odrl:assignee": rule.assignee,
        "odrl:constraint": rule.constraint,
    }
