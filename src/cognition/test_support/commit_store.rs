use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeDelta, Utc};
use typesec_memory::{
    CognitionApplyError, CognitionCommitError, CognitionCommitOutcome, CognitionCommitStatus,
    CognitionCommitStore, CognitionEffect, CognitionIdempotencyKey, CognitionSourcePrecondition,
    InMemoryStore, MemoryId, MemoryStore, PreparedCognitionCommit, StoreError, StoreQuery,
    StoredRecord,
};

#[derive(Clone, Default)]
pub(crate) struct FakeCommitStore {
    records: Arc<InMemoryStore>,
    state: Arc<Mutex<CommitState>>,
    reads: Arc<AtomicUsize>,
    corrupt_outcome: Arc<AtomicUsize>,
    next_read_error: Arc<Mutex<Option<String>>>,
    next_commit_error: Arc<Mutex<Option<String>>>,
}

#[derive(Default)]
struct CommitState {
    applications: BTreeMap<CognitionIdempotencyKey, StoredApplication>,
    version: u64,
}

#[derive(Clone)]
struct StoredApplication {
    proposal_digest: String,
    outcome: CognitionCommitOutcome,
}

impl FakeCommitStore {
    pub(crate) fn commit_count(&self) -> usize {
        self.state
            .lock()
            .expect("fake store lock")
            .applications
            .len()
    }

    pub(crate) fn read_count(&self) -> usize {
        self.reads.load(Ordering::Relaxed)
    }

    pub(crate) fn corrupt_next_outcome(&self) {
        self.corrupt_outcome.store(1, Ordering::Relaxed);
    }

    pub(crate) fn future_date_next_outcome(&self) {
        self.corrupt_outcome.store(2, Ordering::Relaxed);
    }

    pub(crate) fn fail_next_read(&self, message: impl Into<String>) {
        *self.next_read_error.lock().expect("fake store lock") = Some(message.into());
    }

    pub(crate) fn fail_next_commit(&self, message: impl Into<String>) {
        *self.next_commit_error.lock().expect("fake store lock") = Some(message.into());
    }

    fn verify_sources(&self, commit: &PreparedCognitionCommit) -> Result<(), CognitionCommitError> {
        for expected in commit.source_preconditions() {
            let current = self
                .records
                .get(&expected.id)?
                .ok_or_else(|| CognitionCommitError::StaleSource(expected.id.clone()))?;
            let actual =
                CognitionSourcePrecondition::for_record(&current).map_err(cognition_store_error)?;
            if actual.record_digest != expected.record_digest {
                return Err(CognitionCommitError::StaleSource(expected.id.clone()));
            }
        }
        Ok(())
    }
}

impl MemoryStore for FakeCommitStore {
    fn put(&self, record: StoredRecord) -> Result<(), StoreError> {
        self.records.put(record)
    }

    fn get(&self, id: &MemoryId) -> Result<Option<StoredRecord>, StoreError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        if let Some(message) = self.next_read_error.lock().expect("fake store lock").take() {
            return Err(StoreError::Backend(message));
        }
        self.records.get(id)
    }

    fn query(&self, query: &StoreQuery) -> Result<Vec<StoredRecord>, StoreError> {
        self.records.query(query)
    }

    fn invalidate(&self, id: &MemoryId, at: DateTime<Utc>) -> Result<(), StoreError> {
        self.records.invalidate(id, at)
    }

    fn tombstone(&self, id: &MemoryId) -> Result<bool, StoreError> {
        self.records.tombstone(id)
    }
}

impl CognitionCommitStore for FakeCommitStore {
    fn recover_cognition(
        &self,
        key: &CognitionIdempotencyKey,
        proposal_digest: &str,
    ) -> Result<Option<CognitionCommitOutcome>, CognitionCommitError> {
        let state = self.state.lock().expect("fake store lock");
        let Some(stored) = state.applications.get(key) else {
            return Ok(None);
        };
        if stored.proposal_digest != proposal_digest {
            return Err(CognitionCommitError::IdempotencyConflict);
        }
        let mut outcome = stored.outcome.clone();
        outcome.status = CognitionCommitStatus::AlreadyApplied;
        Ok(Some(outcome))
    }

    fn commit_cognition(
        &self,
        commit: PreparedCognitionCommit,
    ) -> Result<CognitionCommitOutcome, CognitionCommitError> {
        if let Some(message) = self
            .next_commit_error
            .lock()
            .expect("fake store lock")
            .take()
        {
            return Err(CognitionCommitError::Store(StoreError::Backend(message)));
        }
        if let Some(recovered) =
            self.recover_cognition(commit.idempotency_key(), commit.proposal_digest())?
        {
            return Ok(recovered);
        }
        self.verify_sources(&commit)?;
        self.records.apply_batch(commit.operations().to_vec())?;

        let idempotency_key = commit.idempotency_key().clone();
        let proposal_digest = commit.proposal_digest().to_owned();
        let effect = commit.effect();
        let audit = commit.audit().clone();

        let mut state = self.state.lock().expect("fake store lock");
        let prior = state.version;
        if effect == CognitionEffect::Mutated {
            state.version += 1;
        }
        let outcome = CognitionCommitOutcome {
            status: CognitionCommitStatus::Applied,
            effect,
            backend_commit_hash: format!("fake-commit-{}", state.applications.len() + 1),
            prior_version: prior.to_string(),
            resulting_version: state.version.to_string(),
            affected_ids: audit.affected_ids.clone(),
            committed_at: audit.prepared_at,
            audit,
        };
        let mut outcome = outcome;
        match self.corrupt_outcome.swap(0, Ordering::Relaxed) {
            1 => outcome.audit.proposal_digest = format!("sha256:{}", "0".repeat(64)),
            2 => outcome.committed_at += TimeDelta::seconds(30),
            _ => {}
        }
        state.applications.insert(
            idempotency_key,
            StoredApplication {
                proposal_digest,
                outcome: outcome.clone(),
            },
        );
        Ok(outcome)
    }
}

fn cognition_store_error(error: CognitionApplyError) -> CognitionCommitError {
    CognitionCommitError::Store(StoreError::Backend(error.to_string()))
}
