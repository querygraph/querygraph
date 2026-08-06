use super::test_support::{
    Fixture, TestClock, application_config, build_application, build_application_with_policy,
    fresh_authorization_digest, projection, proof,
};
use super::{
    CognitionApplicationError, CognitionBindingError, CognitionMemoryError, LakeCatAuthorityError,
};
use chrono::{TimeDelta, TimeZone, Utc};
use lakecat_core::governed_scan::{
    GovernedScanCatalogIdentity, MAX_GOVERNED_SCAN_PROJECTION_FIELDS,
};
use std::sync::Arc;
use typesec_core::ResourceId;
use typesec_core::policy::{PolicyEngine, PolicyResult, RequestContext, SubjectId};
use typesec_memory::{
    CognitionAuthorityError, CognitionAuthorityEvidence, CognitionAuthorityVerifier,
};

use marciana_cognition::{
    CLAIM_ALGORITHM, CLAIM_ALGORITHM_VERSION, CLAIM_JOB_ID, CONTEXT_REQUEST_DIGEST,
    CONTEXT_SUBJECT, PrimedAuthorityVerifier, current_policy_decision_id,
};

#[tokio::test]
async fn stale_future_and_catalog_authority_evidence_fail_before_reveal() {
    let fixture = Fixture::new().await;
    let base = fixture.lakecat.fresh_evidence();

    let mut stale = base.clone();
    stale.revalidated_at = fixture.clock.now() - TimeDelta::seconds(31);
    fixture.lakecat.set_fresh(stale);
    assert_binding_failure(&fixture, CognitionBindingError::StaleAuthorityEvidence).await;

    let mut future = base.clone();
    future.revalidated_at = fixture.clock.now() + TimeDelta::seconds(3);
    fixture.lakecat.set_fresh(future);
    assert_binding_failure(&fixture, CognitionBindingError::FutureAuthorityEvidence).await;

    let mut other_catalog = base;
    other_catalog.catalog_identity =
        GovernedScanCatalogIdentity::new("lakecat://other").expect("other catalog identity");
    fixture.lakecat.set_fresh(other_catalog);
    assert_binding_failure(&fixture, CognitionBindingError::CatalogMismatch).await;
}

#[tokio::test]
async fn current_grant_snapshot_and_projection_are_independent_bindings() {
    let fixture = Fixture::new().await;
    let base = fixture.lakecat.fresh_evidence();

    let mut changed_grant = base.clone();
    changed_grant.current_grant_id = fresh_authorization_digest("changed-grant");
    fixture.lakecat.set_fresh(changed_grant);
    assert_binding_failure(&fixture, CognitionBindingError::GrantMismatch).await;

    let mut changed_snapshot = base.clone();
    changed_snapshot.current_snapshot_digest = fresh_authorization_digest("changed-snapshot");
    fixture.lakecat.set_fresh(changed_snapshot);
    assert_binding_failure(&fixture, CognitionBindingError::SnapshotMismatch).await;

    let mut changed_projection = base.clone();
    changed_projection.current_effective_projection = vec!["memory_id".into()];
    fixture.lakecat.set_fresh(changed_projection);
    assert_binding_failure(&fixture, CognitionBindingError::ProjectionMismatch).await;

    let mut oversized_projection = base.clone();
    oversized_projection.current_effective_projection = (0..=MAX_GOVERNED_SCAN_PROJECTION_FIELDS)
        .map(|index| format!("field_{index:03}"))
        .collect();
    fixture.lakecat.set_fresh(oversized_projection);
    assert_binding_failure(&fixture, CognitionBindingError::ProjectionMismatch).await;

    let mut control_projection = base;
    control_projection.current_effective_projection = vec!["memory_id\nforged".into()];
    fixture.lakecat.set_fresh(control_projection);
    assert_binding_failure(&fixture, CognitionBindingError::ProjectionMismatch).await;
}

#[tokio::test]
async fn changed_proof_and_malformed_current_digest_fail_before_reveal() {
    let fixture = Fixture::new().await;
    let base = fixture.lakecat.fresh_evidence();

    let mut changed_proof = base.clone();
    changed_proof.proof = proof(fixture.proof.principal_subject(), 43, projection());
    fixture.lakecat.set_fresh(changed_proof);
    assert_binding_failure(&fixture, CognitionBindingError::FreshProofMismatch).await;

    let mut malformed = base;
    malformed.fresh_authorization_digest = "SHA256:not-canonical".into();
    fixture.lakecat.set_fresh(malformed);
    assert_binding_failure(&fixture, CognitionBindingError::InvalidAuthorityDigest).await;
}

#[test]
fn provider_revalidation_time_is_bound_into_the_current_decision_identity() {
    let authorization = fresh_authorization_digest("authorization");
    let policy = fresh_authorization_digest("policy");
    let observed_at = Utc.with_ymd_and_hms(2026, 8, 5, 12, 0, 0).unwrap();
    let baseline = current_policy_decision_id(&authorization, &policy, &observed_at)
        .expect("baseline current decision");

    assert_eq!(
        baseline,
        current_policy_decision_id(&authorization, &policy, &observed_at)
            .expect("repeat current decision")
    );
    assert_ne!(
        baseline,
        current_policy_decision_id(
            &authorization,
            &policy,
            &(observed_at + TimeDelta::nanoseconds(1)),
        )
        .expect("timestamp-drifted current decision")
    );
}

#[tokio::test]
async fn bounded_provider_clock_skew_is_not_used_as_typesec_local_time() {
    let fixture = Fixture::new().await;
    let mut fresh = fixture.lakecat.fresh_evidence();
    fresh.revalidated_at = fixture.clock.now() + TimeDelta::seconds(30);
    let expected_decision = current_policy_decision_id(
        &fresh.fresh_authorization_digest,
        &fresh.fresh_policy_decision_digest,
        &fresh.revalidated_at,
    )
    .expect("provider-time-bound decision");
    fixture.lakecat.set_fresh(fresh);
    let request = fixture.signed_request();
    let mut config = application_config(
        fixture.space.clone(),
        fixture.proof.clone(),
        fixture.sources.clone(),
        fixture.mapping.clone(),
    );
    config.authority_future_skew = TimeDelta::minutes(1);
    let application = build_application(
        fixture.store.clone(),
        fixture.clock.clone(),
        &request,
        fixture.lakecat.clone(),
        config,
    )
    .expect("application with bounded provider skew");
    let proposal = application
        .plan_for_test(&fixture.read)
        .await
        .expect("plan under bounded provider skew");

    let result = application
        .apply_for_test(&fixture.write, &proposal)
        .await
        .expect("bounded provider skew remains valid");

    assert_eq!(result.outcome.audit.policy_decision_id, expected_decision);
    assert!(
        result.outcome.audit.authority_revalidated_at <= result.outcome.audit.prepared_at,
        "TypeSec owns and orders its local authority timestamp"
    );
}

#[tokio::test]
async fn typed_denial_and_outage_remain_distinguishable_before_reveal() {
    let fixture = Fixture::new().await;
    let reads = fixture.store.read_count();

    fixture.lakecat.deny();
    assert!(matches!(
        fixture.application.plan_for_test(&fixture.read).await,
        Err(CognitionApplicationError::Authority(
            LakeCatAuthorityError::Denied
        ))
    ));
    assert_eq!(fixture.store.read_count(), reads);

    fixture.lakecat.unavailable();
    assert!(matches!(
        fixture.application.plan_for_test(&fixture.read).await,
        Err(CognitionApplicationError::Authority(
            LakeCatAuthorityError::Unavailable
        ))
    ));
    assert_eq!(fixture.store.read_count(), reads);
}

#[tokio::test]
async fn authority_bridge_failure_exposes_no_adapter_controlled_text() {
    const SECRET: &str = "secret backend response: bearer-token";

    let fixture = Fixture::new().await;
    let proposal = &fixture.proposal;
    let binding = proposal.binding.clone().expect("governed binding");
    let evidence = CognitionAuthorityEvidence {
        space_id: binding.space_id.clone(),
        subject: binding.subject.clone(),
        purpose: binding.purpose.clone(),
        governed_source_scope: binding.governed_source_scope.clone(),
        job_id: proposal.job_id.clone(),
        algorithm: proposal.algorithm.clone(),
        algorithm_version: proposal.algorithm_version.clone(),
        governed_scan_digest: binding.governed_scan_digest.clone(),
        snapshot_digest: binding.snapshot_digest.clone(),
        plan_task_digest: binding.plan_task_digest.clone(),
        authorization_receipt_digest: binding.authorization_receipt_digest.clone(),
        effective_projection: binding.effective_projection.clone(),
        typedid_request_digest: binding.typedid_request_digest.clone(),
        policy_decision_id: fresh_authorization_digest("current-policy"),
    };
    let verifier = PrimedAuthorityVerifier::new(Arc::new(fixture.clock.clone()));
    verifier.prime(evidence, u64::MAX).expect("prime authority");
    let context = RequestContext::new()
        .with_purpose(binding.purpose.clone())
        .with(CONTEXT_SUBJECT, binding.subject.clone())
        .with(
            CONTEXT_REQUEST_DIGEST,
            binding.typedid_request_digest.clone(),
        )
        .with(CLAIM_ALGORITHM, proposal.algorithm.clone())
        .with(CLAIM_ALGORITHM_VERSION, proposal.algorithm_version.clone())
        .with(CLAIM_JOB_ID, SECRET);

    let error = verifier
        .revalidate(&binding, &context)
        .expect_err("changed verified context must fail");
    assert_eq!(error, CognitionAuthorityError::Unavailable);
    let public_error = CognitionApplicationError::AuthorityState(error).to_string();
    assert!(!public_error.contains(SECRET));
    assert_eq!(
        public_error,
        "cognition authority bridge failed: cognition authority evidence is unavailable"
    );
}

#[tokio::test]
async fn expiry_is_rechecked_inside_the_typesec_authority_callback() {
    let fixture = Fixture::new().await;
    let request = fixture.signed_request();
    let application = build_application_with_policy(
        fixture.store.clone(),
        fixture.clock.clone(),
        &request,
        fixture.lakecat.clone(),
        application_config(
            fixture.space.clone(),
            fixture.proof.clone(),
            fixture.sources.clone(),
            fixture.mapping.clone(),
        ),
        Arc::new(ExpireOnWritePolicy(fixture.clock.clone())),
    )
    .expect("application with an expiry-crossing policy");
    let proposal = application
        .plan_for_test(&fixture.read)
        .await
        .expect("proposal bound to expiry-crossing application");

    match application.apply_for_test(&fixture.write, &proposal).await {
        Err(CognitionApplicationError::Memory(CognitionMemoryError::ProposalRejected)) => {}
        Err(error) => panic!("unexpected expiry-crossing error: {error:?}"),
        Ok(_) => panic!("expiry crossing unexpectedly committed"),
    }
    assert_eq!(fixture.store.commit_count(), 0);
}

struct ExpireOnWritePolicy(TestClock);

impl PolicyEngine for ExpireOnWritePolicy {
    fn check(&self, _subject: &SubjectId, _action: &str, _resource: &ResourceId) -> PolicyResult {
        PolicyResult::Allow
    }

    fn check_with_context(
        &self,
        _subject: &SubjectId,
        action: &str,
        _resource: &ResourceId,
        _context: &RequestContext,
    ) -> PolicyResult {
        if action == "write" {
            self.0.advance(TimeDelta::hours(1));
        }
        PolicyResult::Allow
    }
}

async fn assert_binding_failure(fixture: &Fixture, expected: CognitionBindingError) {
    let reads = fixture.store.read_count();
    let result = fixture.application.plan_for_test(&fixture.read).await;
    assert!(matches!(
        result,
        Err(CognitionApplicationError::Binding(error)) if error == expected
    ));
    assert_eq!(fixture.store.read_count(), reads);
}
