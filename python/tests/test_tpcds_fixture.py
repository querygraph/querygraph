from __future__ import annotations

from querygraph.tpcds_fixture import fixture_plan


def test_pinned_ossie_model_generates_five_connected_physical_fixtures():
    datasets = []
    for name, fields in {
        "store_sales": ["ss_sold_date_sk", "ss_item_sk", "ss_customer_sk", "ss_store_sk"],
        "date_dim": ["d_date_sk"], "customer": ["c_customer_sk", "customer_full_name"],
        "item": ["i_item_sk"], "store": ["s_store_sk"],
    }.items():
        datasets.append({"name": name, "fields": [{"name": field, "datatype": "Integer", "expression": {"dialects": [{"dialect": "ANSI_SQL", "expression": field if field != "customer_full_name" else "c_first_name || c_last_name"}]}} for field in fields]})
    model = {"semantic_model": [{"name": "tpcds_retail_model", "datasets": datasets}]}
    plan = fixture_plan(model)
    assert [item.name for item in plan] == ["store_sales", "date_dim", "customer", "item", "store"]
    assert all(len(item.rows) == 3 for item in plan)
    sales = next(item for item in plan if item.name == "store_sales")
    assert sales.rows[0][0:4] == (1, 1, 1, 1)
    customer = next(item for item in plan if item.name == "customer")
    assert "customer_full_name" not in dict(customer.columns)
