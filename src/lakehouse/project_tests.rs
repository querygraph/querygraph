use super::*;
use crate::lakehouse::types::{LakehouseDataType, TypedColumn};

#[test]
fn physical_column_inventory_does_not_promote_name_guesses_to_meaning() {
    let dataset = crate::dataverse::sample_datasets().remove(0);
    let report = LakehouseFileReport {
        file_id: "1".into(),
        filename: "observations.csv".into(),
        content_type: Some("text/csv".into()),
        local_path: "/tmp/observations.csv".into(),
        size_bytes: 0,
        sha256: "fixture".into(),
        table: Some("observations".into()),
        rows: Some(0),
        columns: vec![TypedColumn {
            source_name: "Platform cost".into(),
            name: "platform_cost".into(),
            data_type: LakehouseDataType::Float64,
            nullable: false,
        }],
        parse_status: "loaded".into(),
    };
    let metadata = croissant_for_lakehouse(&dataset, &[report]);
    let field = &metadata.record_sets[0].fields[0];
    assert_eq!(field.name, "platform_cost");
    assert_eq!(field.data_type, "sc:Float");
    assert_eq!(field.semantic_type, None);
}
