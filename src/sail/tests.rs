use super::*;
use crate::dataverse::sample_datasets;
use serde_json::json;

#[test]
fn stages_dataverse_jsonl_for_sail() {
    let root = std::env::temp_dir().join(format!("querygraph-sail-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);

    let report = LocalSailLakehouse::new(&root)
        .stage_dataverse_datasets(&sample_datasets())
        .expect("datasets should stage");

    assert_eq!(report.loads.len(), 2);
    assert!(report.loads[0].metadata_path.exists());
    assert!(report.bootstrap_sql[0].contains("CREATE OR REPLACE TEMP VIEW"));
    assert!(report.graph.is_none());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn builds_dataverse_semantic_graph() {
    let bundle = json!({
        "identity": {"id": "did:example:bundle"},
        "generatedAt": "2026-06-14T00:00:00Z",
        "layers": {"cdif": {"cdif:dataElement": [{
            "@id": "field:one",
            "dct:title": "field one",
            "cdif:semanticType": "https://schema.org/name"
        }]}}
    });
    let osi = OsiDocument::for_dataverse(&sample_datasets());
    let graph = dataverse_semantic_graph(&sample_datasets(), &bundle, Some(&osi));

    assert!(
        graph
            .nodes
            .iter()
            .any(|node| node.label.as_str() == "DataverseDataset")
    );
    assert!(
        graph
            .nodes
            .iter()
            .any(|node| node.label.as_str() == "OsiMetric")
    );
    assert!(
        graph
            .edges
            .iter()
            .any(|edge| edge.label.as_str() == "described_by")
    );
}

#[test]
fn summarizes_semantic_graph_with_cypher() {
    let bundle = json!({
        "identity": {"id": "did:example:bundle"},
        "generatedAt": "2026-06-14T00:00:00Z",
        "layers": {"cdif": {"cdif:dataElement": []}}
    });
    let datasets = sample_datasets();
    let graph = dataverse_semantic_graph(&datasets, &bundle, None);
    let (labels, dataset_count) = summarize_semantic_graph(&graph).expect("cypher summary");

    assert!(labels.contains(&"DataverseDataset".to_string()));
    assert!(labels.windows(2).all(|pair| pair[0] <= pair[1]), "sorted");
    assert_eq!(dataset_count, datasets.len() as i64);
}
