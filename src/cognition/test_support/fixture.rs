use std::sync::Arc;

use chrono::{TimeDelta, Utc};
use ed25519_dalek::{SigningKey, VerifyingKey};
use lakecat_core::governed_scan::{
    GovernedScanCatalogIdentity, GovernedScanProof, GovernedScanProofEvidence,
    governed_authorization_digest, governed_evidence_digest, governed_plan_digest,
    governed_policy_digest,
};
use lakecat_core::{Namespace, TableIdent, TableName, WarehouseName};
use querygraph_memory::cognition::{CognitionEngine, CognitionFieldMapping};
use serde_json::json;
use typesec_core::policy::{MintOptions, PolicyEngine, RequestContext, mint_capability_for_id};
use typesec_core::{CanRead, CanWrite, Capability, Permission, Resource};
use typesec_integrations::{ReceiptIssuer, VerifiedTypeDidContext};
use typesec_memory::{
    CognitionProposal, Label, MemoryContent, MemoryDraft, MemoryId, MemoryKind, MemorySpace,
    MemoryVault, Provenance,
};

use crate::cognition::{
    CognitionBindingError, CognitionEngineBinding, FormationProfile, GovernedCognitionApplication,
    GovernedCognitionConfig, LakeCatCognitionAuthority,
};

use super::{
    AllowPolicy, ExactGovernedSourceVerifier, FakeCommitStore, FakeLakeCatAuthority,
    SignedCognitionRequest, TestClock, verified_subject,
};

pub(crate) const CATALOG_IDENTITY: &str = "lakecat://qglake";

pub(crate) type TestApplication =
    GovernedCognitionApplication<FakeCommitStore, FakeLakeCatAuthority>;

pub(crate) struct Fixture {
    pub(crate) application: TestApplication,
    pub(crate) lakecat: FakeLakeCatAuthority,
    pub(crate) store: FakeCommitStore,
    pub(crate) clock: TestClock,
    pub(crate) read: Capability<CanRead, MemorySpace>,
    pub(crate) write: Capability<CanWrite, MemorySpace>,
    pub(crate) proposal: CognitionProposal,
    pub(crate) sources: Vec<MemoryId>,
    pub(crate) space: MemorySpace,
    pub(crate) proof: GovernedScanProof,
    pub(crate) mapping: CognitionFieldMapping,
    pub(crate) fresh_authorization_digest: String,
    pub(crate) verifying_key: VerifyingKey,
}

impl Fixture {
    pub(crate) async fn new() -> Self {
        Self::build(
            &["duplicate protected source", "duplicate protected source"],
            projection(),
        )
        .await
    }

    pub(crate) async fn with_projection(granted_projection: Vec<&str>) -> Self {
        Self::build(
            &["duplicate protected source", "duplicate protected source"],
            granted_projection,
        )
        .await
    }

    pub(crate) async fn no_change() -> Self {
        Self::build(
            &["first protected source", "second protected source"],
            projection(),
        )
        .await
    }

    async fn build(source_texts: &[&str], granted_projection: Vec<&str>) -> Self {
        let clock = TestClock::new(Utc::now());
        let store = FakeCommitStore::default();
        let policy = Arc::new(AllowPolicy);
        let space = MemorySpace::new("tenant:acme", "research");
        let subject = verified_subject();
        let read = mint::<CanRead>(&policy, &space, &subject);
        let write = mint::<CanWrite>(&policy, &space, &subject);
        let proof = proof(&subject, 42, granted_projection);
        let context = RequestContext::new().with_purpose("research");
        let drafts = source_texts
            .iter()
            .map(|text| {
                MemoryDraft::new(
                    MemoryKind::Semantic,
                    MemoryContent::text((*text).to_owned()),
                    Provenance::Operator,
                )
                .with_label(Label::Sensitive)
                .for_purposes(["research"])
            })
            .collect::<Vec<_>>();
        let source_verifier = Arc::new(ExactGovernedSourceVerifier::new(
            CATALOG_IDENTITY,
            &proof,
            &subject,
            &space,
            "research",
            &drafts,
        ));
        let vault = MemoryVault::new(store.clone())
            .with_policy(policy)
            .with_governed_source_verifier(source_verifier.clone());
        let sources = drafts
            .into_iter()
            .map(|draft| {
                vault
                    .remember_governed(
                        &space,
                        &write,
                        draft,
                        source_verifier.scope(),
                        source_verifier.evidence(),
                        &context,
                    )
                    .expect("store cognition source")
            })
            .collect::<Vec<_>>();
        let mapping = field_mapping();
        let request = SignedCognitionRequest::valid(
            &space,
            "sensitive",
            &sources,
            CATALOG_IDENTITY,
            &proof,
            &mapping,
        );
        let verified = request.open();
        let fresh_authorization_digest = fresh_authorization_digest("initial");
        let lakecat = FakeLakeCatAuthority::allowing(
            CATALOG_IDENTITY,
            proof.clone(),
            &fresh_authorization_digest,
            clock.now(),
        );
        let issuer = ReceiptIssuer::new(SigningKey::from_bytes(&[17; 32]));
        let verifying_key = issuer.verifying_key();
        let application = GovernedCognitionApplication::new_with_clock(
            vault,
            lakecat.clone(),
            CognitionEngineBinding::reference(),
            verified.verified_context(),
            application_config(
                space.clone(),
                proof.clone(),
                sources.clone(),
                mapping.clone(),
            ),
            issuer,
            Arc::new(clock.clone()),
        )
        .expect("governed cognition application");
        let proposal = application
            .plan_for_test(&read)
            .await
            .expect("plan proposal");
        Self {
            application,
            lakecat,
            store,
            clock,
            read,
            write,
            proposal,
            sources,
            space,
            proof,
            mapping,
            fresh_authorization_digest,
            verifying_key,
        }
    }

    pub(crate) fn signed_request(&self) -> SignedCognitionRequest {
        SignedCognitionRequest::valid(
            &self.space,
            "sensitive",
            &self.sources,
            self.lakecat.catalog_identity().as_str(),
            &self.proof,
            &self.mapping,
        )
    }
}

pub(crate) fn application_config(
    space: MemorySpace,
    proof: GovernedScanProof,
    source_ids: Vec<MemoryId>,
    field_mapping: CognitionFieldMapping,
) -> GovernedCognitionConfig {
    GovernedCognitionConfig {
        space,
        proof,
        source_ids,
        field_mapping,
        formation_profile: FormationProfile::BackgroundDeduplicationV1,
        authorized_clearance: Label::Sensitive,
        receipt_ttl: TimeDelta::minutes(5),
        authority_max_age: TimeDelta::seconds(30),
        authority_future_skew: TimeDelta::seconds(2),
    }
}

pub(crate) fn build_application(
    store: FakeCommitStore,
    clock: TestClock,
    request: &SignedCognitionRequest,
    lakecat: FakeLakeCatAuthority,
    config: GovernedCognitionConfig,
) -> Result<TestApplication, CognitionBindingError> {
    build_application_with_policy(
        store,
        clock,
        request,
        lakecat,
        config,
        Arc::new(AllowPolicy),
    )
}

pub(crate) fn build_application_from_verified(
    store: FakeCommitStore,
    clock: TestClock,
    verified: VerifiedTypeDidContext<'_>,
    lakecat: FakeLakeCatAuthority,
    config: GovernedCognitionConfig,
) -> Result<TestApplication, CognitionBindingError> {
    GovernedCognitionApplication::new_with_clock(
        MemoryVault::new(store).with_policy(Arc::new(AllowPolicy)),
        lakecat,
        CognitionEngineBinding::reference(),
        verified,
        config,
        ReceiptIssuer::new(SigningKey::from_bytes(&[29; 32])),
        Arc::new(clock),
    )
}

pub(crate) fn build_application_with_clocks(
    store: FakeCommitStore,
    clock: TestClock,
    receipt_clock: TestClock,
    request: &SignedCognitionRequest,
    lakecat: FakeLakeCatAuthority,
    config: GovernedCognitionConfig,
) -> Result<TestApplication, CognitionBindingError> {
    let verified = request.open();
    GovernedCognitionApplication::new_with_clocks(
        MemoryVault::new(store).with_policy(Arc::new(AllowPolicy)),
        lakecat,
        CognitionEngineBinding::reference(),
        verified.verified_context(),
        config,
        ReceiptIssuer::new(SigningKey::from_bytes(&[29; 32])),
        Arc::new(clock),
        Arc::new(receipt_clock),
    )
}

pub(crate) fn build_application_with_policy(
    store: FakeCommitStore,
    clock: TestClock,
    request: &SignedCognitionRequest,
    lakecat: FakeLakeCatAuthority,
    config: GovernedCognitionConfig,
    policy: Arc<dyn PolicyEngine>,
) -> Result<TestApplication, CognitionBindingError> {
    build_application_with_policy_and_engine(
        store,
        clock,
        request,
        lakecat,
        config,
        policy,
        CognitionEngineBinding::reference(),
    )
}

pub(crate) fn build_application_with_engine(
    store: FakeCommitStore,
    clock: TestClock,
    request: &SignedCognitionRequest,
    lakecat: FakeLakeCatAuthority,
    config: GovernedCognitionConfig,
    engine: Arc<dyn CognitionEngine>,
) -> Result<TestApplication, CognitionBindingError> {
    build_application_with_policy_and_engine(
        store,
        clock,
        request,
        lakecat,
        config,
        Arc::new(AllowPolicy),
        CognitionEngineBinding::test_reference(engine),
    )
}

pub(crate) fn build_application_with_sail_engine(
    store: FakeCommitStore,
    clock: TestClock,
    request: &SignedCognitionRequest,
    lakecat: FakeLakeCatAuthority,
    config: GovernedCognitionConfig,
    engine: Arc<dyn CognitionEngine>,
) -> Result<TestApplication, CognitionBindingError> {
    build_application_with_policy_and_engine(
        store,
        clock,
        request,
        lakecat,
        config,
        Arc::new(AllowPolicy),
        CognitionEngineBinding::test_sail(engine),
    )
}

pub(crate) fn build_application_with_policy_and_engine(
    store: FakeCommitStore,
    clock: TestClock,
    request: &SignedCognitionRequest,
    lakecat: FakeLakeCatAuthority,
    config: GovernedCognitionConfig,
    policy: Arc<dyn PolicyEngine>,
    engine: CognitionEngineBinding,
) -> Result<TestApplication, CognitionBindingError> {
    let verified = request.open();
    let vault = MemoryVault::new(store).with_policy(policy);
    GovernedCognitionApplication::new_with_clock(
        vault,
        lakecat,
        engine,
        verified.verified_context(),
        config,
        ReceiptIssuer::new(SigningKey::from_bytes(&[29; 32])),
        Arc::new(clock),
    )
}

pub(crate) fn field_mapping() -> CognitionFieldMapping {
    CognitionFieldMapping {
        id: "memory_id".into(),
        text: "memory_text".into(),
        valid_from: "valid_from".into(),
    }
}

pub(crate) fn projection() -> Vec<&'static str> {
    vec!["memory_id", "memory_text", "valid_from"]
}

pub(crate) fn proof(subject: &str, snapshot_id: i64, projection: Vec<&str>) -> GovernedScanProof {
    proof_for(subject, "research", snapshot_id, projection)
}

pub(crate) fn proof_for(
    subject: &str,
    purpose: &str,
    snapshot_id: i64,
    projection: Vec<&str>,
) -> GovernedScanProof {
    GovernedScanProof::issue(GovernedScanProofEvidence {
        catalog_identity: GovernedScanCatalogIdentity::new(CATALOG_IDENTITY)
            .expect("catalog identity"),
        table: TableIdent::new(
            WarehouseName::new("local").expect("warehouse"),
            "research".parse::<Namespace>().expect("namespace"),
            TableName::new("findings").expect("table"),
        ),
        table_version: 7,
        snapshot_id,
        plan_task_digest: governed_plan_digest(&[json!({"task": "opaque"})]).expect("plan digest"),
        principal_subject: subject.to_owned(),
        purpose: purpose.to_owned(),
        effective_projection: projection.into_iter().map(str::to_owned).collect(),
        identity_context_digest: governed_evidence_digest(
            "lakecat.verified-identity-context.digest.v1",
            &json!({
                "principal": {"subject": subject},
                "attestation-state": "verified",
            }),
        )
        .expect("identity digest"),
        authorization_receipt_digest: governed_authorization_digest(&json!({
            "allowed": true,
            "receipt": "opaque",
        }))
        .expect("authorization digest"),
        policy_decision_digest: governed_policy_digest(&json!({"policy": "issue-time"}))
            .expect("policy digest"),
    })
    .expect("governed proof")
}

pub(crate) fn fresh_authorization_digest(evidence: &str) -> String {
    governed_authorization_digest(&json!({"fresh": evidence})).expect("fresh digest")
}

pub(crate) fn mint<P: Permission>(
    policy: &Arc<AllowPolicy>,
    space: &MemorySpace,
    subject: &str,
) -> Capability<P, MemorySpace> {
    mint_capability_for_id(
        policy.as_ref(),
        subject,
        space.resource_id(),
        &MintOptions {
            context: RequestContext::new().with_purpose("research"),
            ..MintOptions::default()
        },
    )
    .expect("mint cognition capability")
}
