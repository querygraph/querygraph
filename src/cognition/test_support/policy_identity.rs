use std::collections::BTreeMap;
use std::sync::Arc;

use lakecat_core::governed_scan::GovernedScanProof;
use querygraph_memory::cognition::{
    CognitionEngineProfile, CognitionFieldMapping, CognitionOperation,
};
use typesec_core::ResourceId;
use typesec_core::policy::{PolicyEngine, PolicyResult, SubjectId};
use typesec_integrations::{
    A2aTypeDidAdapter, Did, DidMessageBody, Ed25519DidKey, Ed25519DidKeyStore,
    SecureEnvelopeAdapter, StaticDidResolver, TypeDidGateway, TypeDidMode, TypeDidProfile,
    TypeDidWrapRequest, VerifiedTypeDidMessage,
};
use typesec_memory::{MemoryId, MemorySpace, Resource};

use crate::cognition::{
    CLAIM_ALGORITHM, CLAIM_ALGORITHM_VERSION, CLAIM_CATALOG_IDENTITY, CLAIM_FIELD_MAPPING_DIGEST,
    CLAIM_FORMATION_PROFILE, CLAIM_GRANT_ID, CLAIM_INTENT_VERSION, CLAIM_JOB_ID, CLAIM_OPERATION,
    CLAIM_SOURCE_SELECTION_DIGEST, COGNITION_ACTION, COGNITION_INTENT_VERSION,
    cognition_field_mapping_digest, cognition_source_selection_digest,
};

#[derive(Default)]
pub(crate) struct AllowPolicy;

impl PolicyEngine for AllowPolicy {
    fn check(&self, _subject: &SubjectId, _action: &str, _resource: &ResourceId) -> PolicyResult {
        PolicyResult::Allow
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SignedCognitionRequest {
    pub(crate) action: String,
    pub(crate) resource: String,
    pub(crate) privacy: String,
    pub(crate) purpose: Option<String>,
    pub(crate) intent_version: String,
    pub(crate) job_id: String,
    pub(crate) operation: String,
    pub(crate) formation_profile: String,
    pub(crate) algorithm: String,
    pub(crate) algorithm_version: String,
    pub(crate) source_selection_digest: String,
    pub(crate) catalog_identity: String,
    pub(crate) grant_id: String,
    pub(crate) field_mapping_digest: String,
    pub(crate) extra_claims: BTreeMap<String, String>,
}

impl SignedCognitionRequest {
    pub(crate) fn valid(
        space: &MemorySpace,
        privacy: &str,
        source_ids: &[MemoryId],
        catalog_identity: &str,
        proof: &GovernedScanProof,
        mapping: &CognitionFieldMapping,
    ) -> Self {
        let operation = CognitionOperation::Deduplicate;
        let profile = CognitionEngineProfile::reference(operation);
        let formation_profile = marciana_cognition::FormationProfile::BackgroundDeduplicationV1;
        Self {
            action: COGNITION_ACTION.to_owned(),
            resource: space.resource_id().to_owned(),
            privacy: privacy.to_owned(),
            purpose: Some("research".to_owned()),
            intent_version: COGNITION_INTENT_VERSION.to_owned(),
            job_id: "job-42".to_owned(),
            operation: operation.as_str().to_owned(),
            formation_profile: formation_profile.as_str().to_owned(),
            algorithm: profile.algorithm().to_owned(),
            algorithm_version: profile.algorithm_version().to_owned(),
            source_selection_digest: cognition_source_selection_digest(source_ids)
                .expect("source selection digest"),
            catalog_identity: catalog_identity.to_owned(),
            grant_id: proof.grant_id().to_owned(),
            field_mapping_digest: cognition_field_mapping_digest(mapping)
                .expect("field mapping digest"),
            extra_claims: BTreeMap::new(),
        }
    }

    pub(crate) fn open(&self) -> VerifiedTypeDidMessage {
        let sender_key = Ed25519DidKey::from_seed(b"cognition-sender");
        let recipient_key = Ed25519DidKey::from_seed(b"cognition-recipient");
        let sender = Did::key(sender_key.signing_public());
        let recipient = Did::key(recipient_key.signing_public());
        let resolver = StaticDidResolver::new()
            .with_document(sender_key.document(sender.clone()))
            .with_document(recipient_key.document(recipient.clone()));
        let keys = Ed25519DidKeyStore::new()
            .with_key(sender.clone(), sender_key)
            .with_key(recipient.clone(), recipient_key);
        let body = DidMessageBody {
            action: self.action.clone(),
            resource: self.resource.clone(),
            privacy: self.privacy.clone(),
            claims: self.claims(&sender),
            reply_to: None,
        };
        let mut profile = TypeDidProfile::ed25519_x25519_chacha20();
        profile.policy_actions = vec![self.action.clone()];
        profile.required_claims = body.claims.keys().cloned().collect();
        let envelope = A2aTypeDidAdapter
            .wrap(
                TypeDidWrapRequest {
                    id: "cognition-request".into(),
                    from: sender,
                    to: recipient.clone(),
                    conversation_id: "research".into(),
                    mode: TypeDidMode::RequestReply,
                    body,
                    payload: b"governed request",
                    local_profiles: std::slice::from_ref(&profile),
                    remote_profiles: std::slice::from_ref(&profile),
                },
                &resolver,
                &keys,
            )
            .expect("wrap verified cognition request");
        TypeDidGateway::new(Arc::new(resolver), Arc::new(keys), recipient)
            .open_message(&envelope)
            .expect("verify cognition request")
    }

    fn claims(&self, sender: &Did) -> BTreeMap<String, String> {
        let mut claims = BTreeMap::from([
            ("org".to_owned(), "querygraph".to_owned()),
            ("agent_id".to_owned(), sender.to_string()),
            (CLAIM_INTENT_VERSION.to_owned(), self.intent_version.clone()),
            (CLAIM_JOB_ID.to_owned(), self.job_id.clone()),
            (CLAIM_OPERATION.to_owned(), self.operation.clone()),
            (
                CLAIM_FORMATION_PROFILE.to_owned(),
                self.formation_profile.clone(),
            ),
            (CLAIM_ALGORITHM.to_owned(), self.algorithm.clone()),
            (
                CLAIM_ALGORITHM_VERSION.to_owned(),
                self.algorithm_version.clone(),
            ),
            (
                CLAIM_SOURCE_SELECTION_DIGEST.to_owned(),
                self.source_selection_digest.clone(),
            ),
            (
                CLAIM_CATALOG_IDENTITY.to_owned(),
                self.catalog_identity.clone(),
            ),
            (CLAIM_GRANT_ID.to_owned(), self.grant_id.clone()),
            (
                CLAIM_FIELD_MAPPING_DIGEST.to_owned(),
                self.field_mapping_digest.clone(),
            ),
        ]);
        if let Some(purpose) = &self.purpose {
            claims.insert("purpose".to_owned(), purpose.clone());
        }
        claims.extend(self.extra_claims.clone());
        claims
    }
}

pub(crate) fn verified_subject() -> String {
    let key = Ed25519DidKey::from_seed(b"cognition-sender");
    Did::key(key.signing_public()).to_string()
}
