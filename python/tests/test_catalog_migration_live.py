from types import SimpleNamespace

from querygraph.catalog_migration_live import CatalogEndpoint, canonical_rows, table_state


def test_endpoint_properties_include_only_configured_auth(monkeypatch):
    monkeypatch.setenv("CATALOG_BENCH_S3_ENDPOINT", "http://objects")
    monkeypatch.setenv("CATALOG_BENCH_S3_ACCESS_KEY_ID", "access")
    monkeypatch.setenv("CATALOG_BENCH_S3_SECRET_ACCESS_KEY", "secret")
    monkeypatch.setenv("POLARIS_CREDENTIAL", "client:secret")
    endpoint = CatalogEndpoint(
        "polaris",
        "http://catalog",
        "bench",
        "POLARIS_CREDENTIAL",
        "http://catalog/v1/oauth/tokens",
        "PRINCIPAL_ROLE:ALL",
    )

    properties = endpoint.properties()

    assert properties["credential"] == "client:secret"
    assert properties["warehouse"] == "bench"
    assert properties["scope"] == "PRINCIPAL_ROLE:ALL"


def test_rows_are_order_independent_and_content_sensitive():
    left = canonical_rows([{"id": 2, "event": "b"}, {"id": 1, "event": "a"}])
    right = canonical_rows([{"id": 1, "event": "a"}, {"id": 2, "event": "b"}])

    assert left == right
    assert left != canonical_rows([{"id": 1, "event": "changed"}])


def test_table_state_uses_public_pyiceberg_shape():
    metadata = {
        "table-uuid": "table-uuid",
        "format-version": 2,
        "schemas": [{"schema-id": 0, "type": "struct", "fields": []}],
        "current-schema-id": 0,
        "partition-specs": [{"spec-id": 0, "fields": []}],
        "default-spec-id": 0,
        "sort-orders": [{"order-id": 0, "fields": []}],
        "default-sort-order-id": 0,
        "snapshots": [{"snapshot-id": 22, "sequence-number": 1}],
        "current-snapshot-id": 22,
        "refs": {"main": {"snapshot-id": 22, "type": "branch"}},
    }
    model = SimpleNamespace(model_dump=lambda **_: metadata)
    table = SimpleNamespace(
        metadata=model, metadata_location="s3://warehouse/table/metadata/v1.json"
    )

    state = table_state(table)

    assert state.table_uuid == "table-uuid"
    assert state.current_snapshot_id == 22
