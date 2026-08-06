use querygraph::agent::PyTypeDidEnvelope;
use querygraph::stack::security::verify_http_envelope;
use serde_json::json;
use sha2::{Digest, Sha256};

fn body_hash(body: &[u8]) -> String {
    format!("{:x}", Sha256::digest(body))
}

#[test]
fn verified_agent_is_derived_from_the_signed_sender() {
    let body = br#"{"question":"coffee"}"#;
    let envelope = PyTypeDidEnvelope::signed(
        "stack-security-test-sender",
        "did:web:qg-server",
        "invoke",
        "/v1/answer",
        json!({"bodySha256": body_hash(body)}),
    );

    let agent = verify_http_envelope("/v1/answer", body, &envelope, "did:web:qg-server")
        .expect("signed envelope should verify");

    assert_eq!(agent.subject, envelope.sender);
}

#[test]
fn path_binding_failure_is_rejected() {
    let body = b"{}";
    let envelope = PyTypeDidEnvelope::signed(
        "stack-security-test-sender",
        "did:web:qg-server",
        "invoke",
        "/v1/answer",
        json!({"bodySha256": body_hash(body)}),
    );

    let failure = verify_http_envelope("/v1/models", body, &envelope, "did:web:qg-server")
        .expect_err("resource must be bound to the route");

    assert_eq!(failure.reason, "envelope auth failed");
    assert_eq!(failure.checks["resourceBoundToPath"], json!(false));
}
