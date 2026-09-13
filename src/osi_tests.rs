use super::*;
use crate::dataverse::sample_datasets;

#[test]
fn synthesizes_osi_from_dataverse() {
    let osi = OsiDocument::for_dataverse(&sample_datasets());
    assert_eq!(osi.semantic_model.name, "querygraph_dataverse_navigator");
    assert_eq!(osi.semantic_model.datasets.len(), 2);
    assert_eq!(osi.semantic_model.metrics[0].name, "governed_dataset_count");
    assert!(!osi.semantic_model.ontology_terms.is_empty());
}

#[test]
fn parses_osi_yaml() {
    let yaml = r#"
version: 0.2.0.dev0
semantic_model:
  name: revenue
  datasets:
    - name: orders
      source: sail.orders
      fields:
        - name: amount
          semantic_type: https://schema.org/price
  metrics:
    - name: total_revenue
      expression:
        dialects:
          - dialect: SAIL_SQL
            expression: SUM(amount)
"#;
    let parsed: OsiDocument = serde_yaml::from_str(yaml).expect("valid OSI yaml");
    assert_eq!(parsed.semantic_model.metrics[0].name, "total_revenue");
}
