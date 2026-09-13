use grust::SailWarehouse;

use super::configured_sail_client;

#[test]
fn querygraph_config_preserves_inputs_and_uses_safe_defaults() {
    let config = configured_sail_client(
        "http://sail.example.test:50051",
        "querygraph-lakehouse",
        256,
        None,
    );

    assert_eq!(config.endpoint, "http://sail.example.test:50051");
    assert_eq!(config.user_id, "querygraph-lakehouse");
    assert_eq!(config.batch_size, 256);
    assert!(
        uuid::Uuid::parse_str(&config.session_id).is_ok(),
        "default-derived session id must be a UUID"
    );
    assert_eq!(config.warehouse, SailWarehouse::ServerManaged);
}

#[tokio::test]
async fn explicit_warehouse_is_validated_by_the_sail_client() {
    let relative = configured_sail_client(
        "http://localhost:15051",
        "demo",
        10,
        Some("relative".into()),
    );
    match grust::SailGraphStore::connect(relative).await {
        Ok(_) => panic!("relative warehouse was accepted"),
        Err(error) => assert!(error.to_string().contains("absolute path")),
    }
    let directory = std::env::temp_dir().join("querygraph-demo-warehouse");
    let absolute = configured_sail_client(
        "http://localhost:15051",
        "demo",
        10,
        Some(directory.clone()),
    );
    assert_eq!(absolute.warehouse, SailWarehouse::ExplicitPath(directory));
}
