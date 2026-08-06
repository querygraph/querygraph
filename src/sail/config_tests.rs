use grust::SailWarehouse;

use super::querygraph_sail_config;

#[test]
fn querygraph_config_preserves_inputs_and_uses_safe_defaults() {
    let config = querygraph_sail_config(
        "http://sail.example.test:50051",
        "querygraph-lakehouse",
        256,
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
