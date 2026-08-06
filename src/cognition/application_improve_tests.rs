use typesec_memory::CognitionCommitStatus;

use super::test_support::Fixture;

#[tokio::test]
async fn improve_keeps_the_proposal_internal_and_returns_committed_evidence() {
    let fixture = Fixture::new().await;

    let result = fixture
        .application
        .improve(&fixture.read, &fixture.write)
        .await
        .expect("improve commits the internally planned proposal");

    assert_eq!(result.outcome.status, CognitionCommitStatus::Applied);
    assert_eq!(fixture.store.commit_count(), 1);
    // One revalidation happens while constructing this fixture's internal
    // test seam; `improve` still performs both planning and pre-commit gates.
    assert!(fixture.lakecat.revalidation_count() >= 3);
}
