"""Prepare a NEW retained customer-discovery fixture; never overwrite the old one.

Run through uv with the same pyiceberg extras as execute_rows.py.
The existing rows fixture retains its original snapshot and policy.
"""
import argparse
import json
from pathlib import Path
import shutil

import pyarrow as pa
from pyiceberg.catalog.sql import SqlCatalog
from run import run, write_json


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--existing", type=Path, required=True)
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--querygraph-bin", type=Path, required=True)
    args = parser.parse_args()
    old = args.existing.resolve(strict=True)
    dest = args.destination.resolve()
    fixture = json.loads((Path(__file__).resolve().parents[2] / "demo/pinax/customer-meaning.json").read_text())
    document = json.loads((old / "registry.json").read_text())
    if [t["name"] for t in document["tables"]] != ["rows"]:
        raise ValueError("Expected the original single-table demo")
    dest.mkdir(parents=True, exist_ok=False)
    for name in ["identity.json", "recipient.json", "agent.seed"]:
        shutil.copy2(old / name, dest / name)
    document["tables"].extend(fixture["tables"])
    write_json(dest / "registry.json", document)
    plans = run([str(args.querygraph_bin), "pinax", "catalog-plan", "--registry", str(dest / "registry.json"), "--warehouse", "warehouse"])
    properties = {p["body"]["name"]: p["body"]["properties"] for p in plans}
    seed = json.loads((old / "seed.json").read_text())
    seed["metadata"]["properties"].update(properties["rows"])
    write_json(dest / "seed.json", seed)
    catalog = SqlCatalog("customer-demo", uri=f"sqlite:///{dest / 'catalog.db'}", warehouse=(dest / "warehouse").as_uri())
    catalog.create_namespace("acme")
    tables = []
    for contract in fixture["tables"]:
        schema = pa.schema([pa.field(c["name"], pa.int64(), nullable=c["nullability"] != "required") for c in contract["columns"]])
        table = catalog.create_table(f"acme.{contract['name']}", schema=schema, properties=properties[contract["name"]])
        table.append(pa.Table.from_pylist(fixture["records"][contract["name"]], schema=schema))
        tables.append({"name":contract["name"], "location":table.location(), "metadata_location":table.metadata_location,
                       "metadata":table.metadata.model_dump(mode="json", by_alias=True)})
    write_json(dest / "customer-tables.json", tables)
    config = json.loads((old / "registry-service.json").read_text())
    config.pop("ontology", None)
    config["activation"]["expected_registry_digest"] = properties["rows"]["pinax.registry-digest"]
    write_json(dest / "registry-service.json", config)
    # Preserve the narrow original role; append only these reviewed resources.
    import yaml
    policy = yaml.safe_load((old / "policy.yaml").read_text())
    for table in fixture["tables"]:
        prefix = f"pinax/acme/{table['name']}/revisions/1"
        policy["roles"][0]["resources"].extend([prefix] + [f"{prefix}/columns/{c['id']}" for c in table["columns"]])
    (dest / "policy.yaml").write_text(yaml.safe_dump(policy))
    print(json.dumps({"prepared":str(dest), "business_tables":len(tables), "original_fixture":str(old)}))


if __name__ == "__main__":
    main()
