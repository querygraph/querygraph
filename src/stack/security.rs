//! QueryGraph-owned request-authentication boundary.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::agent::PyTypeDidEnvelope;

/// Identity established by a verified TypeDID request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedAgent {
    /// The signing DID. Callers must not replace this with a body field.
    pub subject: String,
}

/// Structured authentication failure consumed by HTTP and MCP adapters.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthFailure {
    pub reason: String,
    pub checks: Value,
}

/// Verify the QueryGraph HTTP envelope contract without exposing verification
/// details to route handlers.
pub fn verify_http_envelope(
    path: &str,
    body: &[u8],
    envelope: &PyTypeDidEnvelope,
    server_did: &str,
) -> Result<VerifiedAgent, AuthFailure> {
    let verification = envelope.verify();
    let body_sha256 = format!("{:x}", Sha256::digest(body));
    let signer = envelope
        .verification_method
        .as_deref()
        .and_then(|method| method.split('#').next());
    let checks = json!({
        "signatureValid": verification.signature_valid,
        "senderMatchesVerificationMethod": signer == Some(envelope.sender.as_str()),
        "recipientIsServer": envelope.recipient == server_did,
        "actionIsInvoke": envelope.action == "invoke",
        "resourceBoundToPath": envelope.resource == path,
        "bodyBound": envelope.payload["bodySha256"] == json!(body_sha256),
    });
    let allowed = checks
        .as_object()
        .expect("authentication checks are an object")
        .values()
        .all(|value| value == &json!(true));
    if !allowed {
        return Err(AuthFailure {
            reason: "envelope auth failed".to_string(),
            checks,
        });
    }
    Ok(VerifiedAgent {
        subject: envelope.sender.clone(),
    })
}
