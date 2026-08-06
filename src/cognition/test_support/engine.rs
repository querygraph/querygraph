use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use querygraph_memory::cognition::{
    CognitionEngine, CognitionError, CognitionRequest, ReferenceCognitionEngine,
};
use typesec_memory::CognitionProposal;

#[derive(Default)]
pub(crate) struct ObservingEngine {
    invocations: AtomicUsize,
    exposed_memories: AtomicUsize,
}

impl ObservingEngine {
    pub(crate) fn invocation_count(&self) -> usize {
        self.invocations.load(Ordering::Relaxed)
    }

    pub(crate) fn exposed_memory_count(&self) -> usize {
        self.exposed_memories.load(Ordering::Relaxed)
    }

    pub(crate) fn assert_uninvoked(&self) {
        assert_eq!(self.invocation_count(), 0);
        assert_eq!(self.exposed_memory_count(), 0);
    }
}

#[async_trait]
impl CognitionEngine for ObservingEngine {
    async fn propose(
        &self,
        request: CognitionRequest<'_>,
    ) -> Result<CognitionProposal, CognitionError> {
        self.invocations.fetch_add(1, Ordering::Relaxed);
        self.exposed_memories
            .fetch_add(request.input.memories().len(), Ordering::Relaxed);
        ReferenceCognitionEngine.propose(request).await
    }
}
