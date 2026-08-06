use chrono::TimeDelta;
use typesec_integrations::ReceiptVerifier;
use typesec_memory::{CognitionCommitStatus, CognitionEffect};

use super::test_support::Fixture;

#[tokio::test]
async fn no_change_is_authorized_committed_receipted_and_recovered_without_mutation() {
    let fixture = Fixture::no_change().await;
    assert_eq!(fixture.proposal.effect, CognitionEffect::NoChange);
    assert!(fixture.proposal.drafts.is_empty());
    assert!(fixture.proposal.plan.steps.is_empty());

    let authority_before_apply = fixture.lakecat.revalidation_count();
    let reads_before_apply = fixture.store.read_count();
    let first = fixture
        .application
        .apply_for_test(&fixture.write, &fixture.proposal)
        .await
        .expect("commit authorized no-change decision");

    assert_eq!(first.outcome.status, CognitionCommitStatus::Applied);
    assert_eq!(first.outcome.effect, CognitionEffect::NoChange);
    assert_eq!(first.outcome.audit.effect, CognitionEffect::NoChange);
    assert!(first.outcome.affected_ids.is_empty());
    assert!(first.outcome.audit.affected_ids.is_empty());
    assert_eq!(first.outcome.prior_version, first.outcome.resulting_version);
    assert_eq!(first.receipt.effect(), CognitionEffect::NoChange);
    assert!(first.receipt.affected_ids().is_empty());
    assert_eq!(
        first.receipt.prior_version(),
        first.receipt.resulting_version()
    );
    assert_eq!(fixture.store.commit_count(), 1);
    assert_eq!(
        fixture.lakecat.revalidation_count(),
        authority_before_apply + 1
    );
    assert!(fixture.store.read_count() > reads_before_apply);
    assert_eq!(
        ReceiptVerifier::new(fixture.verifying_key)
            .verify_cognition(
                &first.receipt_token,
                first.receipt.issued_at() + TimeDelta::seconds(1),
            )
            .expect("verify no-change receipt"),
        first.receipt
    );

    let reads_before_recovery = fixture.store.read_count();
    let authority_before_recovery = fixture.lakecat.revalidation_count();
    let replay = fixture
        .application
        .apply_for_test(&fixture.write, &fixture.proposal)
        .await
        .expect("recover authorized no-change decision");

    assert_eq!(replay.outcome.status, CognitionCommitStatus::AlreadyApplied);
    assert_eq!(replay.outcome.effect, CognitionEffect::NoChange);
    assert_eq!(replay.receipt, first.receipt);
    assert_eq!(replay.receipt_token, first.receipt_token);
    assert_eq!(fixture.store.read_count(), reads_before_recovery);
    assert_eq!(
        fixture.lakecat.revalidation_count(),
        authority_before_recovery + 1
    );
    assert_eq!(fixture.store.commit_count(), 1);
}
