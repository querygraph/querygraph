use super::*;
use serde_json::json;

#[test]
fn activation_rejects_unreviewed_registry_before_catalog_io() {
    let directory = tempfile::tempdir().unwrap();
    let registry = super::super::tests::registry();
    std::fs::write(
        directory.path().join("registry.json"),
        serde_json::to_vec(&registry).unwrap(),
    )
    .unwrap();
    std::fs::write(
        directory.path().join("policy.yaml"),
        "roles: []\nassignments: []\n",
    )
    .unwrap();
    std::fs::write(directory.path().join("envelope.json"), "{}").unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut config = json!({
        "registry":"registry.json", "warehouse":"warehouse", "policy":"policy.yaml",
        "policy_id":"enterprise", "policy_revision":1, "server_did":"did:example:registry",
        "lakecat_origin":format!("http://{}/", listener.local_addr().unwrap()),
        "lakecat_subject":"did:example:agent", "lakecat_envelope":"envelope.json"
    });
    let path = directory.path().join("config.json");
    std::fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
    // Existing discovery-only startup remains valid without catalog availability.
    assert!(load_registry_service(&path).is_ok());
    config["activation"] = json!({"expected_registry_digest":"unreviewed"});
    std::fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
    let error = load_registry_service(&path).err().expect("review mismatch");
    assert!(error.to_string().contains("reviewed activation digest"));
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}
