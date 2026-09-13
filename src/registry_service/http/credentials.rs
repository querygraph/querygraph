//! Host-owned credentials mint fresh TypeSec envelopes without caching decisions.

use lakecat_core::{LakeCatError, LakeCatResult, content_hash_bytes};
use reqwest::Url;
use typesec_integrations::{
    Did, DidDocument, DidEnvelope, DidMessageBody, Ed25519DidKey, Ed25519DidKeyStore,
    StaticDidResolver, TypeDidConversation, TypeDidMode, TypeDidProfile,
};

/// Trusted credential provider. Request arguments cannot select or replace it.
pub trait LakeCatCredentials: Send + Sync {
    /// Mint a fresh envelope for one outgoing request. Never reuse a consumed
    /// TypeSec envelope: the owner gateway enforces replay protection.
    fn envelope(&self, method: &str, url: &Url, body: &[u8]) -> LakeCatResult<String>;
}

/// TypeSec's Ed25519/X25519 credential implementation for a configured agent.
/// Private key material remains in the TypeSec keystore; no Debug/serialization.
pub struct TypeSecLakeCatCredentials {
    sender: Did,
    recipient: Did,
    resolver: StaticDidResolver,
    keys: Ed25519DidKeyStore,
}

impl TypeSecLakeCatCredentials {
    /// Bind a high-entropy 32-byte host seed to its canonical MCP did:key subject
    /// and a host-pinned recipient DID document. The caller owns seed storage.
    ///
    /// # Errors
    /// Rejects malformed seeds, subjects not matching the actual signing key,
    /// or unusable recipient documents when an envelope is minted.
    pub fn from_seed(seed: &[u8], subject: &str, recipient: DidDocument) -> LakeCatResult<Self> {
        if seed.len() != 32 {
            return Err(LakeCatError::InvalidArgument(
                "LakeCat identity requires a 32-byte seed".into(),
            ));
        }
        let key = Ed25519DidKey::from_seed(seed);
        let verifying = ed25519_dalek::VerifyingKey::from_bytes(&key.signing_public())
            .map_err(|_| LakeCatError::InvalidArgument("invalid LakeCat signing key".into()))?;
        let expected = format!("did:key:{}", crate::agent::ed25519_multibase(&verifying));
        if subject != expected {
            return Err(LakeCatError::InvalidArgument(
                "LakeCat signing key does not match configured subject".into(),
            ));
        }
        let sender = Did::parse(subject)
            .map_err(|_| LakeCatError::InvalidArgument("invalid LakeCat subject".into()))?;
        let recipient_id = recipient.id.clone();
        let resolver = StaticDidResolver::new()
            .with_document(key.document(sender.clone()))
            .with_document(recipient);
        Ok(Self {
            sender: sender.clone(),
            recipient: recipient_id,
            resolver,
            keys: Ed25519DidKeyStore::new().with_key(sender, key),
        })
    }
}

impl LakeCatCredentials for TypeSecLakeCatCredentials {
    fn envelope(&self, method: &str, url: &Url, body: &[u8]) -> LakeCatResult<String> {
        let id = format!("querygraph:{}", uuid::Uuid::new_v4());
        let resource = format!("lakecat:http:{method}:{}", url.path());
        let payload = serde_json::to_vec(&serde_json::json!({"method":method,"path":url.path(),"body_sha256":content_hash_bytes(body)}))
            .map_err(|_| LakeCatError::Internal("cannot encode catalog request binding".into()))?;
        let envelope = DidEnvelope::typedid(
            &id,
            self.sender.clone(),
            self.recipient.clone(),
            DidMessageBody::agent_message(resource, "internal"),
            TypeDidConversation::new(
                id.clone(),
                TypeDidMode::RequestReply,
                TypeDidProfile::ed25519_x25519_chacha20().id,
                "https",
            ),
            &payload,
            &self.resolver,
            &self.keys,
        )
        .map_err(|_| LakeCatError::Forbidden("cannot mint LakeCat TypeSec credential".into()))?;
        serde_json::to_string(&envelope)
            .map_err(|_| LakeCatError::Internal("cannot encode LakeCat credential".into()))
    }
}

pub(super) struct OneShotEnvelope(pub(super) std::sync::Mutex<Option<String>>);
impl LakeCatCredentials for OneShotEnvelope {
    fn envelope(&self, _: &str, _: &Url, _: &[u8]) -> LakeCatResult<String> {
        self.0
            .lock()
            .map_err(|_| LakeCatError::Internal("catalog credential state unavailable".into()))?
            .take()
            .ok_or_else(|| {
                LakeCatError::NotSupported(
                    "single-use LakeCat envelope consumed; configure a signing identity".into(),
                )
            })
    }
}

#[cfg(test)]
mod tests;
