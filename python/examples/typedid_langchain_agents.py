#!/usr/bin/env python3
"""TypeDID agents with Pydantic models and an optional LangChain adapter."""
from __future__ import annotations

import json

from querygraph.agents import TypeDidLangChainToolAdapter, deterministic_specialist
from querygraph.qglake import build_python_qglake_story
from querygraph.typedid import TypeDidAgent


def main() -> None:
    story = build_python_qglake_story()
    print(json.dumps(story["synthesis"], indent=2))

    finance = TypeDidAgent.new("FinanceAgent")
    handler = deterministic_specialist(
        finance,
        summary="Fiscal capacity summary from governed Sail finance tables.",
        evidence=["global_temp.government_finance__countydata"],
    )

    try:
        tool = TypeDidLangChainToolAdapter(finance, handler).as_tool()
    except RuntimeError as exc:
        print(f"LangChain optional extra not installed: {exc}")
        return

    print(tool.invoke({"question": "Summarize fiscal capacity", "resource": "compartment:finance"}))


if __name__ == "__main__":
    main()
