use std::collections::BTreeMap;
use std::sync::Mutex;

use lakecat_core::governed_scan::{
    GovernedScanCatalogIdentity, GovernedScanProof, governed_scan_digests,
};
use typesec_memory::{
    GovernedSourceScope, GovernedSourceVerification, GovernedSourceVerificationError,
    GovernedSourceVerifier, MemoryDraft, MemorySpace, Resource, governed_source_draft_digest,
};

/// Exact, consuming verifier for the governed rows staged by one test scan.
pub(crate) struct ExactGovernedSourceVerifier {
    scope: GovernedSourceScope,
    subject: String,
    space_id: String,
    purpose: String,
    evidence: Vec<u8>,
    remaining_drafts: Mutex<BTreeMap<String, usize>>,
}

impl ExactGovernedSourceVerifier {
    pub(crate) fn new(
        catalog: &str,
        proof: &GovernedScanProof,
        subject: &str,
        space: &MemorySpace,
        purpose: &str,
        drafts: &[MemoryDraft],
    ) -> Self {
        let configured_catalog =
            GovernedScanCatalogIdentity::new(catalog).expect("configured catalog identity");
        assert_eq!(configured_catalog, proof.catalog_identity().clone());
        let digests = governed_scan_digests(proof).expect("governed scan digests");
        let scope = GovernedSourceScope::from_digest(digests.source_scope_digest().to_owned())
            .expect("canonical source scope");
        let mut remaining_drafts = BTreeMap::new();
        for draft in drafts {
            let digest = governed_source_draft_digest(draft).expect("governed draft digest");
            *remaining_drafts.entry(digest).or_insert(0) += 1;
        }
        Self {
            scope,
            subject: subject.to_owned(),
            space_id: space.resource_id().to_owned(),
            purpose: purpose.to_owned(),
            evidence: serde_json::to_vec(proof).expect("serialize governed proof"),
            remaining_drafts: Mutex::new(remaining_drafts),
        }
    }

    pub(crate) fn scope(&self) -> &GovernedSourceScope {
        &self.scope
    }

    pub(crate) fn evidence(&self) -> &[u8] {
        &self.evidence
    }
}

impl GovernedSourceVerifier for ExactGovernedSourceVerifier {
    fn verify(
        &self,
        request: &GovernedSourceVerification<'_>,
    ) -> Result<(), GovernedSourceVerificationError> {
        if request.scope() != &self.scope
            || request.subject().as_str() != self.subject
            || request.space_id() != self.space_id
            || request.context().purpose.as_deref() != Some(self.purpose.as_str())
            || request.evidence() != self.evidence.as_slice()
        {
            return Err(GovernedSourceVerificationError::Unavailable);
        }
        let mut remaining = self
            .remaining_drafts
            .lock()
            .map_err(|_| GovernedSourceVerificationError::Unavailable)?;
        let count = remaining
            .get_mut(request.draft_digest())
            .ok_or(GovernedSourceVerificationError::Unavailable)?;
        *count = count
            .checked_sub(1)
            .ok_or(GovernedSourceVerificationError::Unavailable)?;
        if *count == 0 {
            remaining.remove(request.draft_digest());
        }
        Ok(())
    }
}
