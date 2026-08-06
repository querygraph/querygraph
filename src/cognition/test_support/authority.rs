use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lakecat_core::governed_scan::{
    GovernedScanCatalogIdentity, GovernedScanProof, governed_scan_digests,
};

use crate::cognition::{FreshLakeCatAuthority, LakeCatAuthorityError, LakeCatCognitionAuthority};

#[derive(Clone)]
pub(crate) struct FakeLakeCatAuthority {
    catalog_identity: GovernedScanCatalogIdentity,
    result: Arc<Mutex<Result<FreshLakeCatAuthority, LakeCatAuthorityError>>>,
    revalidations: Arc<AtomicUsize>,
}

impl FakeLakeCatAuthority {
    pub(crate) fn allowing(
        catalog_identity: &str,
        proof: GovernedScanProof,
        fresh_authorization_digest: &str,
        revalidated_at: DateTime<Utc>,
    ) -> Self {
        let catalog_identity = GovernedScanCatalogIdentity::new(catalog_identity)
            .expect("canonical fake catalog identity");
        let fresh = allowed(
            &catalog_identity,
            proof,
            fresh_authorization_digest,
            revalidated_at,
        );
        Self {
            catalog_identity,
            result: Arc::new(Mutex::new(Ok(fresh))),
            revalidations: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub(crate) fn allow(
        &self,
        proof: GovernedScanProof,
        fresh_authorization_digest: &str,
        revalidated_at: DateTime<Utc>,
    ) {
        *self.result.lock().expect("fake authority lock") = Ok(allowed(
            &self.catalog_identity,
            proof,
            fresh_authorization_digest,
            revalidated_at,
        ));
    }

    pub(crate) fn set_fresh(&self, fresh: FreshLakeCatAuthority) {
        *self.result.lock().expect("fake authority lock") = Ok(fresh);
    }

    pub(crate) fn fresh_evidence(&self) -> FreshLakeCatAuthority {
        self.result
            .lock()
            .expect("fake authority lock")
            .as_ref()
            .expect("fake authority is allowing")
            .clone()
    }

    pub(crate) fn deny(&self) {
        *self.result.lock().expect("fake authority lock") = Err(LakeCatAuthorityError::Denied);
    }

    pub(crate) fn unavailable(&self) {
        *self.result.lock().expect("fake authority lock") = Err(LakeCatAuthorityError::Unavailable);
    }

    pub(crate) fn revalidation_count(&self) -> usize {
        self.revalidations.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl LakeCatCognitionAuthority for FakeLakeCatAuthority {
    fn catalog_identity(&self) -> &GovernedScanCatalogIdentity {
        &self.catalog_identity
    }

    async fn revalidate(
        &self,
        _presented: &GovernedScanProof,
    ) -> Result<FreshLakeCatAuthority, LakeCatAuthorityError> {
        self.revalidations.fetch_add(1, Ordering::Relaxed);
        self.result.lock().expect("fake authority lock").clone()
    }
}

fn allowed(
    catalog_identity: &GovernedScanCatalogIdentity,
    proof: GovernedScanProof,
    fresh_authorization_digest: &str,
    revalidated_at: DateTime<Utc>,
) -> FreshLakeCatAuthority {
    let fresh_policy_decision_digest = proof.policy_decision_digest().to_owned();
    let current_grant_id = proof.grant_id().to_owned();
    let current_snapshot_digest = governed_scan_digests(&proof)
        .expect("governed scan digests")
        .snapshot_digest()
        .to_owned();
    let current_effective_projection = proof.effective_projection().to_vec();
    FreshLakeCatAuthority {
        catalog_identity: catalog_identity.clone(),
        proof,
        current_grant_id,
        current_snapshot_digest,
        current_effective_projection,
        fresh_authorization_digest: fresh_authorization_digest.to_owned(),
        fresh_policy_decision_digest,
        revalidated_at,
    }
}
