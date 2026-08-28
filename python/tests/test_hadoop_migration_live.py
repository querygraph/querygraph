from querygraph.hadoop_migration_live import (
    LegacyMigrationState,
    canonical_rows,
    verify_legacy_migration,
)


def state() -> LegacyMigrationState:
    return LegacyMigrationState(
        schema="struct<1: id: required long>",
        specs=((0, "[]"), (1, "[bucket[4](1)]")),
        current_spec_id=1,
        snapshots=(10, 20),
        current_snapshot_id=20,
        refs=(("audit", "20"), ("main", "20")),
        metadata_location="s3://warehouse/hadoop/ns/events/metadata/v3.json",
    )


def test_equal_evolved_legacy_state_is_preserved():
    result = verify_legacy_migration(state(), state())

    assert result.preserved
    assert result.snapshot_count == 2
    assert result.spec_count == 2
    assert result.ref_count == 2


def test_legacy_loss_report_names_each_changed_dimension():
    destination = LegacyMigrationState(
        **{**state().__dict__, "snapshots": (20,), "refs": (("main", "20"),)}
    )

    result = verify_legacy_migration(state(), destination)

    assert result.mismatches == ("snapshots", "refs")


def test_legacy_row_digest_is_order_independent_and_null_aware():
    left = canonical_rows([(2, "b", None), (1, "a", "x")])
    right = canonical_rows([(1, "a", "x"), (2, "b", None)])

    assert left == right
    assert left != canonical_rows([(1, "changed", "x")])
