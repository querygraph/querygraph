#!/usr/bin/env python3
"""Build OSI business semantics from a Semantic Croissant dataset."""
from __future__ import annotations

import json

from querygraph.croissant import CroissantDataset, Field, FileObject, RecordSet
from querygraph.osi import OsiDocument


dataset = CroissantDataset(
    id="https://querygraph.ai/datasets/energy-burden/#dataset",
    name="Energy Burden Demonstration",
    description="Governed household energy survey fields in Sail.",
    license="https://creativecommons.org/licenses/by/4.0/",
    creators=["QueryGraph"],
    files=[
        FileObject(
            id="https://querygraph.ai/datasets/energy-burden/#file",
            name="energy_burden.parquet",
            content_url="sail://qg_lakehouse/access_2018__access_data",
            encoding_format="application/vnd.apache.parquet",
        )
    ],
    record_sets=[
        RecordSet(
            id="https://querygraph.ai/datasets/energy-burden/#recordset",
            name="energy burden observations",
            fields=[
                Field(
                    "household_id",
                    "sc:Text",
                    "Stable household identifier.",
                ).semantic_type("https://schema.org/identifier"),
                Field(
                    "energy_source",
                    "sc:Text",
                    "Primary household energy source.",
                ).semantic_type("https://querygraph.ai/ontology/energySource"),
                Field(
                    "monthly_energy_cost",
                    "sc:Float",
                    "Monthly energy cost in local currency.",
                ).semantic_type("https://querygraph.ai/ontology/monthlyEnergyCost"),
            ],
        )
    ],
    keywords=["Semantic Croissant", "OSI", "energy burden", "Sail"],
)


def main() -> None:
    osi = OsiDocument.from_croissant(dataset, sail_schema="qg_lakehouse")
    print(json.dumps({"croissant": dataset.to_json_ld(), "osi": osi.to_json()}, indent=2))


if __name__ == "__main__":
    main()
