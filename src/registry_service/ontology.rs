//! Host-owned central ontology consultation after signed intent verification.
use super::*;
use pinax::ontology::{Ontology, store::OntologyStore};
use std::path::PathBuf;

/// Pinned central store. Agents cannot select or mutate its path or digest.
#[derive(Clone)]
pub struct CentralOntology {
    store: OntologyStore,
    expected_digest: String,
}

impl CentralOntology {
    /// Validate the central snapshot at blocking service bootstrap.
    pub fn load(
        path: PathBuf,
        expected_digest: String,
        registry: &Registry,
    ) -> Result<Self, RegistryServiceError> {
        let source = Self {
            store: OntologyStore::new(path),
            expected_digest,
        };
        source.read(registry)?;
        Ok(source)
    }

    fn read(&self, registry: &Registry) -> Result<Ontology, RegistryServiceError> {
        let ontology = self.store.load(registry)?;
        if ontology.digest()? != self.expected_digest {
            return Err(pinax::ontology::OntologyError::Conflict.into());
        }
        Ok(ontology)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OntologyIntent {
    purpose: String,
    query: String,
    limit: usize,
    cursor: Option<DiscoveryCursor>,
}

impl RegistryService {
    pub(super) async fn load_ontology(&self) -> Result<Ontology, RegistryServiceError> {
        let source = self
            .ontology
            .clone()
            .ok_or(RegistryServiceError::Configuration)?;
        let registry = self.registry.clone();
        tokio::task::spawn_blocking(move || source.read(&registry))
            .await
            .map_err(|_| RegistryServiceError::Configuration)?
    }

    /// Attach a deployment-validated ontology source. No request can replace it.
    pub fn with_ontology(mut self, ontology: CentralOntology) -> Self {
        self.ontology = Some(ontology);
        self
    }

    /// Consult the central ontology and return only freshly authorized metadata.
    ///
    /// Signs `/mcp/discover_pinax_ontology` over exact JSON containing purpose,
    /// query, result limit (1-100), and optional registry cursor. Each call examines
    /// at most 100 tables. Continue even when a page has no matches but a cursor.
    /// Missing or changed central publications fail closed. Blocking reads are
    /// bounded and observed; cancellation cannot interrupt an OS file read already
    /// running, but that task cannot publish or release an agent response.
    pub async fn discover_ontology(
        &self,
        intent_json: &str,
        envelope: &PyTypeDidEnvelope,
    ) -> Result<Value, RegistryServiceError> {
        let principal = self.authenticate(intent_json, envelope, RegistryOperation::Ontology)?;
        let intent: OntologyIntent = serde_json::from_str(intent_json)?;
        if intent.query.trim().is_empty()
            || intent.query.len() > 1024
            || !(1..=100).contains(&intent.limit)
        {
            return Err(pinax::ontology::OntologyError::Invalid("search bounds").into());
        }
        let ontology = self.load_ontology().await?;
        let page = self
            .governed()
            .discover(&principal, &intent.purpose, intent.cursor.as_ref(), 100)
            .await?;
        let search = ontology.search(&page, &intent.query, intent.limit)?;
        let exports = ontology.export(&page)?;
        Ok(serde_json::json!({"search":search,"exports":exports,"next_cursor":page.next_cursor()}))
    }
}
