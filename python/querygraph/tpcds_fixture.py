from __future__ import annotations

from dataclasses import dataclass
from decimal import Decimal
from typing import Any


@dataclass(frozen=True)
class TpcdsTableFixture:
    name: str
    columns: tuple[tuple[str, str], ...]
    rows: tuple[tuple[Any, ...], ...]


TYPE_SQL = {"Integer": "BIGINT", "Decimal": "DECIMAL(18,2)", "Date": "DATE", "String": "STRING"}


def fixture_plan(ossie: dict[str, Any]) -> tuple[TpcdsTableFixture, ...]:
    models = ossie.get("semantic_model", [])
    if len(models) != 1 or models[0].get("name") != "tpcds_retail_model":
        raise ValueError("expected the pinned TPC-DS Ossie model")
    fixtures = []
    for dataset in models[0].get("datasets", []):
        columns = []
        for field in dataset.get("fields", []):
            expressions = field.get("expression", {}).get("dialects", [])
            ansi = next((item.get("expression") for item in expressions if item.get("dialect") == "ANSI_SQL"), None)
            if ansi != field.get("name"):
                continue
            columns.append((field["name"], TYPE_SQL.get(field.get("datatype"), "STRING")))
        rows = tuple(tuple(_value(dataset["name"], name, sql_type, index) for name, sql_type in columns) for index in range(1, 4))
        fixtures.append(TpcdsTableFixture(dataset["name"], tuple(columns), rows))
    if {item.name for item in fixtures} != {"store_sales", "date_dim", "customer", "item", "store"}:
        raise ValueError("pinned TPC-DS dataset set drifted")
    return tuple(fixtures)


def _value(dataset: str, field: str, sql_type: str, index: int) -> Any:
    if sql_type == "DATE":
        return f"2024-0{index}-01"
    if sql_type.startswith("DECIMAL"):
        if field == "ss_ext_sales_price": return Decimal((20, 35, 50)[index - 1])
        if field == "ss_net_profit": return Decimal((5, 9, 14)[index - 1])
        return Decimal(10 + index)
    if sql_type == "BIGINT":
        if field == "d_year": return 2024
        if field == "ss_quantity": return (2, 3, 4)[index - 1]
        if field == "s_number_employees": return (10, 20, 25)[index - 1]
        return index
    return f"{dataset}-{field}-{index}"


def sql_literal(value: Any, sql_type: str) -> str:
    if sql_type == "DATE": return f"DATE '{value}'"
    if isinstance(value, (int, Decimal)): return str(value)
    return "'" + str(value).replace("'", "''") + "'"
