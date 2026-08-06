use chrono::{TimeDelta, Utc};
use lakecat_core::governed_scan::governed_policy_digest;
use serde_json::json;
use typesec_integrations::{ReceiptError, ReceiptVerifier};
use typesec_memory::CognitionCommitStatus;

use super::test_support::{
    Fixture, TestClock, application_config, build_application_with_clocks,
    fresh_authorization_digest, projection, proof,
};
use super::{
    CognitionApplicationError, CognitionBindingError, CognitionMemoryError, LakeCatAuthorityError,
};
use marciana_cognition::current_policy_decision_id;

#[tokio::test]
async fn revocation_and_changed_grant_fail_before_commit() {
    let fixture = Fixture::new().await;
    fixture.lakecat.deny();
    assert!(matches!(
        fixture
            .application
            .apply_for_test(&fixture.write, &fixture.proposal)
            .await,
        Err(CognitionApplicationError::Authority(
            LakeCatAuthorityError::Denied
        ))
    ));
    assert_eq!(fixture.store.commit_count(), 0);

    let changed = proof(fixture.proof.principal_subject(), 43, projection());
    fixture.lakecat.allow(
        changed,
        &fresh_authorization_digest("changed-grant"),
        fixture.clock.now(),
    );
    assert!(matches!(
        fixture
            .application
            .apply_for_test(&fixture.write, &fixture.proposal)
            .await,
        Err(CognitionApplicationError::Binding(
            CognitionBindingError::FreshProofMismatch
        ))
    ));
    assert_eq!(fixture.store.commit_count(), 0);
}

#[tokio::test]
async fn receipt_verifies_and_replay_returns_identical_signed_bytes() {
    let fixture = Fixture::new().await;
    let provider_revalidated_at = fixture.lakecat.fresh_evidence().revalidated_at;
    let current_decision = current_policy_decision_id(
        &fixture.fresh_authorization_digest,
        fixture.proof.policy_decision_digest(),
        &provider_revalidated_at,
    )
    .expect("current decision");
    assert_ne!(
        current_decision,
        current_policy_decision_id(
            &fresh_authorization_digest("changed-authorization"),
            fixture.proof.policy_decision_digest(),
            &provider_revalidated_at,
        )
        .expect("changed authorization decision")
    );
    assert_ne!(
        current_decision,
        current_policy_decision_id(
            &fixture.fresh_authorization_digest,
            &governed_policy_digest(&json!({"policy": "changed"})).expect("changed policy"),
            &provider_revalidated_at,
        )
        .expect("changed policy decision")
    );
    let mut regenerated = fixture.proposal.clone();
    regenerated.created_at = fixture.proposal.created_at + TimeDelta::minutes(1);

    let first = fixture
        .application
        .apply_for_test(&fixture.write, &fixture.proposal)
        .await
        .expect("apply cognition");
    assert_eq!(first.outcome.status, CognitionCommitStatus::Applied);
    let debug = format!("{first:?}");
    assert_eq!(
        debug,
        "GovernedCognitionResult { status: Applied, effect: Mutated, evidence: \"<redacted>\" }"
    );
    assert!(!debug.contains(&first.receipt_token));
    assert!(!debug.contains(&first.outcome.audit.subject));
    assert!(!debug.contains(&first.outcome.backend_commit_hash));
    assert_eq!(first.outcome.audit.policy_decision_id, current_decision);
    assert!(
        first
            .receipt
            .policy_decision_digest()
            .starts_with("sha256:")
    );
    assert_eq!(
        first.receipt.authorization_receipt_digest(),
        fixture.proof.authorization_receipt_digest()
    );
    assert_eq!(
        first.receipt.input_snapshot_digest(),
        fixture.proposal.input_snapshot
    );
    assert_ne!(
        first.receipt.input_snapshot_digest(),
        fixture.proof.grant_id()
    );
    let serialized_receipt = serde_json::to_string(&first.receipt).expect("serialize receipt");
    let serialized_evidence =
        serde_json::to_string(&first.outcome.audit).expect("serialize audit evidence");
    for raw_current_value in [
        fixture.fresh_authorization_digest.as_str(),
        fixture.proof.policy_decision_digest(),
        current_decision.as_str(),
    ] {
        assert!(!serialized_receipt.contains(raw_current_value));
    }
    for raw_fresh_digest in [
        fixture.fresh_authorization_digest.as_str(),
        fixture.proof.policy_decision_digest(),
    ] {
        assert!(!serialized_evidence.contains(raw_fresh_digest));
    }
    let verified = ReceiptVerifier::new(fixture.verifying_key)
        .verify_cognition(
            &first.receipt_token,
            first.receipt.issued_at() + TimeDelta::seconds(1),
        )
        .expect("verify commit receipt");
    assert_eq!(verified, first.receipt);

    let changed_policy =
        governed_policy_digest(&json!({"policy": "still-allowing-replay"})).expect("policy digest");
    let mut fresh = fixture.lakecat.fresh_evidence();
    fresh.fresh_policy_decision_digest = changed_policy;
    fixture.lakecat.set_fresh(fresh);

    let replay = fixture
        .application
        .apply_for_test(&fixture.write, &regenerated)
        .await
        .expect("recover cognition");
    assert_eq!(replay.outcome.status, CognitionCommitStatus::AlreadyApplied);
    assert_eq!(replay.receipt, first.receipt);
    assert_eq!(
        replay.receipt_token.as_bytes(),
        first.receipt_token.as_bytes()
    );
    assert_eq!(fixture.store.commit_count(), 1);
}

#[tokio::test]
async fn bogus_backend_outcome_is_rejected_before_receipt_signing() {
    let fixture = Fixture::new().await;
    fixture.store.corrupt_next_outcome();
    assert!(matches!(
        fixture
            .application
            .apply_for_test(&fixture.write, &fixture.proposal)
            .await,
        Err(CognitionApplicationError::Memory(
            CognitionMemoryError::InvalidCommitOutcome
        ))
    ));
    assert_eq!(fixture.store.commit_count(), 1);
}

#[tokio::test]
async fn future_backend_time_blocks_first_issuance_without_pinning_the_failed_attempt() {
    let fixture = Fixture::new().await;
    let request = fixture.signed_request();
    let receipt_clock = TestClock::new(Utc::now() + TimeDelta::seconds(1));
    let application = build_application_with_clocks(
        fixture.store.clone(),
        fixture.clock.clone(),
        receipt_clock.clone(),
        &request,
        fixture.lakecat.clone(),
        application_config(
            fixture.space.clone(),
            fixture.proof.clone(),
            fixture.sources.clone(),
            fixture.mapping.clone(),
        ),
    )
    .expect("application with separate receipt clock");
    let proposal = application
        .plan_for_test(&fixture.read)
        .await
        .expect("plan for future commit test");
    fixture.store.future_date_next_outcome();
    let error = application
        .apply_for_test(&fixture.write, &proposal)
        .await
        .expect_err("future commit time must block receipt issuance");
    assert!(matches!(
        error,
        CognitionApplicationError::Receipt(ReceiptError::NotYetValid { .. })
    ));
    assert_eq!(fixture.store.commit_count(), 1);
    let reads_after_commit = fixture.store.read_count();

    receipt_clock.advance(TimeDelta::seconds(31));
    let first = application
        .apply_for_test(&fixture.write, &proposal)
        .await
        .expect("issue after local clock catches up");
    assert_eq!(first.outcome.status, CognitionCommitStatus::AlreadyApplied);
    assert_eq!(first.receipt.issued_at(), receipt_clock.now());
    assert_eq!(first.receipt.prepared_at(), first.outcome.audit.prepared_at);
    assert_eq!(first.receipt.committed_at(), first.outcome.committed_at);
    assert!(first.receipt.issued_at() >= first.receipt.committed_at());
    assert_eq!(
        first.receipt.expires_at(),
        first.receipt.prepared_at() + TimeDelta::minutes(5)
    );
    assert_eq!(fixture.store.read_count(), reads_after_commit);

    let replay = application
        .apply_for_test(&fixture.write, &proposal)
        .await
        .expect("recover already issued receipt");
    assert_eq!(replay.receipt, first.receipt);
    assert_eq!(replay.receipt_token, first.receipt_token);
    assert_eq!(fixture.store.commit_count(), 1);
}

#[tokio::test]
async fn changed_authorizing_policy_is_bound_as_current_not_original_evidence() {
    let fixture = Fixture::new().await;
    let changed_policy =
        governed_policy_digest(&json!({"policy": "current-authorizing"})).expect("policy digest");
    let mut fresh = fixture.lakecat.fresh_evidence();
    fresh.fresh_policy_decision_digest = changed_policy.clone();
    let provider_revalidated_at = fresh.revalidated_at;
    fixture.lakecat.set_fresh(fresh);
    let expected_current = current_policy_decision_id(
        &fixture.fresh_authorization_digest,
        &changed_policy,
        &provider_revalidated_at,
    )
    .expect("current decision");

    let result = fixture
        .application
        .apply_for_test(&fixture.write, &fixture.proposal)
        .await
        .expect("apply under changed authorizing policy");
    assert_eq!(result.outcome.audit.policy_decision_id, expected_current);
    assert_eq!(
        result.outcome.audit.authorization_receipt_digest,
        fixture.proof.authorization_receipt_digest()
    );
    assert_eq!(
        result.receipt.authorization_receipt_digest(),
        fixture.proof.authorization_receipt_digest()
    );
    assert_ne!(
        result.receipt.authorization_receipt_digest(),
        fixture.fresh_authorization_digest.as_str()
    );
    assert_ne!(
        result.receipt.authorization_receipt_digest(),
        changed_policy.as_str()
    );
}
