from copy import deepcopy

import pytest

from querygraph.catalog_migration import (
    CatalogTableState,
    MigrationDimension,
    verify_migration,
)


def table_response() -> dict:
    return {
        "metadata-location": "s3://warehouse/events/metadata/00001.json",
        "metadata": {
            "table-uuid": "00000000-0000-0000-0000-000000000001",
            "format-version": 2,
            "schemas": [{"schema-id": 0, "type": "struct", "fields": []}],
            "current-schema-id": 0,
            "partition-specs": [{"spec-id": 0, "fields": []}],
            "default-spec-id": 0,
            "sort-orders": [{"order-id": 0, "fields": []}],
            "default-sort-order-id": 0,
            "snapshots": [{"snapshot-id": 17, "sequence-number": 1}],
            "current-snapshot-id": 17,
            "refs": {"main": {"snapshot-id": 17, "type": "branch"}},
        },
    }


def test_equal_nonempty_table_state_proves_semantic_preservation() -> None:
    source = CatalogTableState.from_rest_response(table_response())
    destination = CatalogTableState.from_rest_response(deepcopy(table_response()))

    result = verify_migration(source, destination)

    assert result.preserved
    assert result.proves_nonempty_history
    assert result.source_digest == result.destination_digest
    assert result.snapshot_count == 1
    assert result.ref_count == 1


def test_snapshot_and_ref_loss_is_reported_explicitly() -> None:
    destination_response = table_response()
    destination_response["metadata"]["snapshots"] = []
    destination_response["metadata"]["current-snapshot-id"] = None
    destination_response["metadata"]["refs"] = {}

    result = verify_migration(
        CatalogTableState.from_rest_response(table_response()),
        CatalogTableState.from_rest_response(destination_response),
    )

    assert not result.preserved
    assert result.mismatches == (
        MigrationDimension.SNAPSHOTS,
        MigrationDimension.CURRENT_SNAPSHOT,
        MigrationDimension.REFS,
    )


def test_known_iceberg_fields_are_required_at_the_boundary() -> None:
    response = table_response()
    del response["metadata"]["partition-specs"]

    with pytest.raises(ValueError, match="partition-specs must be an array"):
        CatalogTableState.from_rest_response(response)
