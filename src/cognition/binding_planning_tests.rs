use async_trait::async_trait;
use querygraph_memory::cognition::{
    CognitionEngine, CognitionError, CognitionRequest, ReferenceCognitionEngine,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use typesec_core::{CanRead, CanWrite};
use typesec_memory::{CognitionProposal, ConsolidationPlan, ConsolidationStep, MemoryId};

use super::test_support::{
    AllowPolicy, Fixture, ObservingEngine, application_config, build_application_from_verified,
    build_application_with_engine, build_application_with_sail_engine, mint,
};
use super::{
    CognitionApplicationError, CognitionBindingError, CognitionMemoryError, LakeCatAuthorityError,
};

#[derive(Clone, Copy)]
enum Tamper {
    Job,
    Subset,
    Reorder,
    Duplicate,
    AlgorithmPrefix,
    OtherOperation,
    SourceDigest,
}

struct TamperingEngine(Tamper);

#[derive(Default)]
struct UninvokedEngine {
    invocations: AtomicUsize,
}

#[async_trait]
impl CognitionEngine for UninvokedEngine {
    async fn propose(
        &self,
        _request: CognitionRequest<'_>,
    ) -> Result<CognitionProposal, CognitionError> {
        self.invocations.fetch_add(1, Ordering::Relaxed);
        unreachable!("a mismatched engine must not receive authorized input")
    }
}

#[async_trait]
impl CognitionEngine for TamperingEngine {
    async fn propose(
        &self,
        request: CognitionRequest<'_>,
    ) -> Result<CognitionProposal, CognitionError> {
        let mut proposal = ReferenceCognitionEngine.propose(request).await?;
        match self.0 {
            Tamper::Job => proposal.job_id = "job-substitution".into(),
            Tamper::Subset => {
                proposal.source_ids.pop();
            }
            Tamper::Reorder => proposal.source_ids.reverse(),
            Tamper::Duplicate => {
                let first = proposal
                    .source_ids
                    .first()
                    .cloned()
                    .unwrap_or_else(|| MemoryId::from_string("missing"));
                proposal.source_ids = vec![first.clone(), first];
            }
            Tamper::AlgorithmPrefix => {
                proposal.algorithm = "marciana.deduplicate.reference.extra".into();
            }
            Tamper::OtherOperation => {
                proposal.algorithm = "marciana.reconcile.reference".into();
            }
            Tamper::SourceDigest => proposal.source_digest = format!("sha256:{}", "0".repeat(64)),
        }
        Ok(proposal)
    }
}

#[tokio::test]
async fn source_capability_must_belong_to_the_verified_subject() {
    let fixture = Fixture::new().await;
    let other_policy = Arc::new(AllowPolicy);
    let other_read = mint::<CanRead>(&other_policy, &fixture.space, "did:key:other");
    assert!(matches!(
        fixture.application.plan_for_test(&other_read).await,
        Err(CognitionApplicationError::Binding(
            CognitionBindingError::ReadSubjectMismatch
        ))
    ));
    assert_eq!(fixture.store.commit_count(), 0);
}

#[tokio::test]
async fn write_capability_must_belong_to_the_verified_subject() {
    let fixture = Fixture::new().await;
    let other_policy = Arc::new(AllowPolicy);
    let other_write = mint::<CanWrite>(&other_policy, &fixture.space, "did:key:other");
    let authority_checks = fixture.lakecat.revalidation_count();

    assert!(matches!(
        fixture
            .application
            .apply_for_test(&other_write, &fixture.proposal)
            .await,
        Err(CognitionApplicationError::Binding(
            CognitionBindingError::WriteSubjectMismatch
        ))
    ));
    assert_eq!(fixture.lakecat.revalidation_count(), authority_checks);
    assert_eq!(fixture.store.commit_count(), 0);
}

#[tokio::test]
async fn host_selected_engine_family_mismatch_is_rejected_before_authority_or_vault_read() {
    let fixture = Fixture::new().await;
    let reads = fixture.store.read_count();
    let checks = fixture.lakecat.revalidation_count();
    let engine = Arc::new(UninvokedEngine::default());
    let request = fixture.signed_request();

    assert!(matches!(
        build_application_with_sail_engine(
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
            engine.clone(),
        ),
        Err(CognitionBindingError::EngineProfileMismatch)
    ));
    assert_eq!(engine.invocations.load(Ordering::Relaxed), 0);
    assert_eq!(fixture.store.read_count(), reads);
    assert_eq!(fixture.lakecat.revalidation_count(), checks);
}

#[tokio::test]
async fn engine_cannot_change_job_sources_algorithm_or_binding() {
    let fixture = Fixture::new().await;
    for tamper in [
        Tamper::Job,
        Tamper::Subset,
        Tamper::Reorder,
        Tamper::Duplicate,
        Tamper::AlgorithmPrefix,
        Tamper::OtherOperation,
        Tamper::SourceDigest,
    ] {
        let request = fixture.signed_request();
        let application = build_application_with_engine(
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
            Arc::new(TamperingEngine(tamper)),
        )
        .expect("trusted test binding");
        assert!(matches!(
            application.plan_for_test(&fixture.read).await,
            Err(CognitionApplicationError::Binding(
                CognitionBindingError::ProposalIntentMismatch
                    | CognitionBindingError::EngineOutputMismatch
            ))
        ));
    }
    assert_eq!(fixture.store.commit_count(), 0);
}

#[tokio::test]
async fn changed_job_algorithm_version_or_sources_cannot_reach_application() {
    let fixture = Fixture::new().await;
    let initial_authority_checks = fixture.lakecat.revalidation_count();
    let mut changed_job = fixture.proposal.clone();
    changed_job.job_id = "job-elsewhere".into();

    let mut changed_algorithm = fixture.proposal.clone();
    changed_algorithm.algorithm = "marciana.reconcile.reference".into();

    let mut changed_version = fixture.proposal.clone();
    changed_version.algorithm_version = "1".into();

    let mut changed_sources = fixture.proposal.clone();
    changed_sources.source_ids.pop();

    for changed in [
        changed_job,
        changed_algorithm,
        changed_version,
        changed_sources,
    ] {
        assert!(matches!(
            fixture
                .application
                .apply_for_test(&fixture.write, &changed)
                .await,
            Err(CognitionApplicationError::Binding(
                CognitionBindingError::ProposalIntentMismatch
            ))
        ));
    }
    assert_eq!(
        fixture.lakecat.revalidation_count(),
        initial_authority_checks,
        "proposal intent must fail before a LakeCat call"
    );
    assert_eq!(fixture.store.commit_count(), 0);
}

#[tokio::test]
async fn denial_exposes_no_plaintext_and_performs_no_vault_read() {
    let fixture = Fixture::new().await;
    let initial_reads = fixture.store.read_count();
    let request = fixture.signed_request();
    let observer = Arc::new(ObservingEngine::default());
    let application = build_application_with_engine(
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
        observer.clone(),
    )
    .expect("application with observing test engine");
    fixture.lakecat.deny();
    assert!(matches!(
        application.plan_for_test(&fixture.read).await,
        Err(CognitionApplicationError::Authority(
            LakeCatAuthorityError::Denied
        ))
    ));
    observer.assert_uninvoked();
    assert_eq!(fixture.store.read_count(), initial_reads);
}

#[tokio::test]
async fn store_failure_exposes_no_backend_controlled_text() {
    const SECRET: &str = "protected storage response: bearer-token";

    let fixture = Fixture::new().await;
    let request = fixture.signed_request();
    let observer = Arc::new(ObservingEngine::default());
    let application = build_application_with_engine(
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
        observer.clone(),
    )
    .expect("application with observing test engine");
    fixture.store.fail_next_read(SECRET);
    let error = application
        .plan_for_test(&fixture.read)
        .await
        .expect_err("store failure");

    assert!(matches!(
        error,
        CognitionApplicationError::Memory(CognitionMemoryError::BackendUnavailable)
    ));
    assert!(!error.to_string().contains(SECRET));
    observer.assert_uninvoked();
}

#[tokio::test]
async fn commit_failure_exposes_no_backend_controlled_text() {
    const SECRET: &str = "protected commit response: database-password";

    let fixture = Fixture::new().await;
    fixture.store.fail_next_commit(SECRET);
    let error = fixture
        .application
        .apply_for_test(&fixture.write, &fixture.proposal)
        .await
        .expect_err("commit failure");

    assert!(matches!(
        error,
        CognitionApplicationError::Memory(CognitionMemoryError::BackendUnavailable)
    ));
    assert!(!error.to_string().contains(SECRET));
    assert_eq!(fixture.store.commit_count(), 0);
}

#[tokio::test]
async fn out_of_source_plan_fails_before_backend_commit() {
    let fixture = Fixture::new().await;
    let mut proposal = fixture.proposal.clone();
    proposal.plan = ConsolidationPlan::new().then(ConsolidationStep::Invalidate {
        ids: vec![MemoryId::from_string("mem-outside-signed-source-set")],
    });
    assert!(matches!(
        fixture
            .application
            .apply_for_test(&fixture.write, &proposal)
            .await,
        Err(CognitionApplicationError::Binding(
            CognitionBindingError::InvalidProposalDigest
        ))
    ));
    assert_eq!(fixture.store.commit_count(), 0);
}

#[tokio::test]
async fn caller_cannot_replace_engine_plan_with_another_valid_source_mutation() {
    let fixture = Fixture::new().await;
    let authority_checks = fixture.lakecat.revalidation_count();
    let mut proposal = fixture.proposal.clone();
    proposal.plan = ConsolidationPlan::new().then(ConsolidationStep::Invalidate {
        ids: vec![fixture.sources[0].clone()],
    });

    assert!(matches!(
        fixture
            .application
            .apply_for_test(&fixture.write, &proposal)
            .await,
        Err(CognitionApplicationError::Binding(
            CognitionBindingError::PlannedProposalMismatch
        ))
    ));
    assert_eq!(fixture.lakecat.revalidation_count(), authority_checks);
    assert_eq!(fixture.store.commit_count(), 0);
}

#[tokio::test]
async fn equivalent_unplanned_application_cannot_attribute_an_external_proposal_to_its_engine() {
    let fixture = Fixture::new().await;
    let request = fixture.signed_request();
    let verified = request.open();
    let planned_application = build_application_from_verified(
        fixture.store.clone(),
        fixture.clock.clone(),
        verified.verified_context(),
        fixture.lakecat.clone(),
        application_config(
            fixture.space.clone(),
            fixture.proof.clone(),
            fixture.sources.clone(),
            fixture.mapping.clone(),
        ),
    )
    .expect("equivalent planned application");
    let proposal = planned_application
        .plan_for_test(&fixture.read)
        .await
        .expect("plan under exact verified request");
    let application = build_application_from_verified(
        fixture.store.clone(),
        fixture.clock.clone(),
        verified.verified_context(),
        fixture.lakecat.clone(),
        application_config(
            fixture.space.clone(),
            fixture.proof.clone(),
            fixture.sources.clone(),
            fixture.mapping.clone(),
        ),
    )
    .expect("equivalent unplanned application");
    let authority_checks = fixture.lakecat.revalidation_count();

    let error = application
        .apply_for_test(&fixture.write, &proposal)
        .await
        .expect_err("unplanned proposal must fail");
    assert!(
        matches!(
            error,
            CognitionApplicationError::Binding(CognitionBindingError::ProposalNotPlanned)
        ),
        "unexpected error: {error:?}"
    );
    assert_eq!(fixture.lakecat.revalidation_count(), authority_checks);
    assert_eq!(fixture.store.commit_count(), 0);
}

#[tokio::test]
async fn planning_stages_only_mapped_fields_from_a_broader_lakecat_grant() {
    let fixture = Fixture::with_projection(vec![
        "memory_id",
        "memory_text",
        "valid_from",
        "private_note",
    ])
    .await;
    assert!(
        fixture
            .proof
            .effective_projection()
            .iter()
            .any(|field| field == "private_note")
    );
    let projection = &fixture
        .proposal
        .binding
        .expect("proposal binding")
        .effective_projection;
    assert_eq!(projection, &["memory_id", "memory_text", "valid_from"]);
    assert!(!projection.iter().any(|field| field == "private_note"));
}
