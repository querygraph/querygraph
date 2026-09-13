use super::*;
use crate::registry_service::RegistryCatalog;
use crate::registry_service::deployment::{RegistryDeploymentStatus, catalog::CatalogTableState};
use async_trait::async_trait;
use lakecat_core::{
    LakeCatResult, PrincipalKind,
    sail::{DeferredSailCatalogEngine, SailCatalogEngine},
};
use pinax::{
    Registry,
    adapters::{CatalogBinding, CatalogCreatePlan},
};
use std::{collections::BTreeMap, sync::Mutex};

enum Failure {
    None,
    LoseNextReply,
    ChangeNextState,
}
enum Tokens {
    Present,
    Absent,
}
struct State {
    tables: BTreeMap<String, Value>,
    commits: Vec<(String, Value)>,
    creates: Vec<String>,
    failure: Failure,
}
struct Catalog {
    state: Mutex<State>,
    targets: BTreeMap<String, Value>,
    tokens: Tokens,
    engine: DeferredSailCatalogEngine,
}

#[async_trait]
impl RegistryCatalog for Catalog {
    async fn load_metadata(&self, table: &TableIdent, _: &Principal) -> LakeCatResult<Value> {
        self.state
            .lock()
            .unwrap()
            .tables
            .get(table.name.as_str())
            .cloned()
            .ok_or_else(|| LakeCatError::NotFound {
                object: "table",
                name: table.name.as_str().into(),
            })
    }
    fn engine(&self) -> &dyn SailCatalogEngine {
        &self.engine
    }
}

#[async_trait]
impl RegistryDeploymentCatalog for Catalog {
    async fn load_state(
        &self,
        table: &TableIdent,
        principal: &Principal,
    ) -> LakeCatResult<CatalogTableState> {
        let metadata = self.load_metadata(table, principal).await?;
        let token = match self.tokens {
            Tokens::Present => Some(content_hash_json(&metadata)?),
            Tokens::Absent => None,
        };
        CatalogTableState::new(metadata, token)
    }
    async fn create_registry_table(
        &self,
        table: &TableIdent,
        _: &Principal,
        _: &CatalogCreatePlan,
    ) -> LakeCatResult<()> {
        let mut state = self.state.lock().unwrap();
        assert!(!state.tables.contains_key(table.name.as_str()));
        state.creates.push(table.name.as_str().into());
        state.tables.insert(
            table.name.as_str().into(),
            self.targets[table.name.as_str()].clone(),
        );
        Ok(())
    }
    async fn commit_registry_table(
        &self,
        table: &TableIdent,
        _: &Principal,
        token: &str,
        request: &RegistryCatalogCommit,
        key: &str,
    ) -> LakeCatResult<()> {
        assert!(key.starts_with("sha256:"));
        let mut state = self.state.lock().unwrap();
        state.commits.push((
            table.name.as_str().into(),
            serde_json::to_value(request).unwrap(),
        ));
        let failure = std::mem::replace(&mut state.failure, Failure::None);
        if matches!(failure, Failure::ChangeNextState) {
            state.tables.get_mut(table.name.as_str()).unwrap()["properties"]["pinax.registry-digest"] =
                json!("concurrent");
        }
        if token != content_hash_json(&state.tables[table.name.as_str()])? {
            return Err(LakeCatError::Conflict("owner rejected stale state".into()));
        }
        state.tables.insert(
            table.name.as_str().into(),
            self.targets[table.name.as_str()].clone(),
        );
        if matches!(failure, Failure::LoseNextReply) {
            return Err(LakeCatError::Unavailable("reply lost after commit".into()));
        }
        Ok(())
    }
}

fn metadata(registry: &Registry, binding: &CatalogBinding, table: &str) -> Value {
    let contract = binding.table_contract(registry, table).unwrap();
    let last_id = contract.schema()["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| field["id"].as_i64().unwrap())
        .max()
        .unwrap();
    json!({"table-uuid":"11111111-1111-1111-1111-111111111111","current-schema-id":0,"last-column-id":last_id,
        "schemas":[contract.schema()],"properties":contract.properties()})
}

fn fixture(add_column: bool) -> (RegistryDeploymentPlan, Catalog, Principal) {
    let previous = crate::registry_service::tests::registry();
    let mut target = serde_json::to_value(&previous).unwrap();
    let mut extra = target["tables"][0].clone();
    extra["name"] = json!("transactions");
    target["tables"].as_array_mut().unwrap().push(extra);
    if add_column {
        target["tables"][0]["revision"] = json!(2);
        target["tables"][0]["columns"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "id":2,"name":"note","data_type":{"kind":"string"},"nullability":"optional",
                "semantic":"customer.note","description":"Optional note","classification":"internal"
            }));
    }
    let target = Registry::from_json(&serde_json::to_vec(&target).unwrap()).unwrap();
    let binding = CatalogBinding::new("warehouse").unwrap();
    let catalog = Catalog {
        state: Mutex::new(State {
            tables: BTreeMap::from([(
                "customers".into(),
                metadata(&previous, &binding, "customers"),
            )]),
            commits: vec![],
            creates: vec![],
            failure: Failure::None,
        }),
        targets: target
            .tables()
            .iter()
            .map(|table| (table.name.clone(), metadata(&target, &binding, &table.name)))
            .collect(),
        tokens: Tokens::Present,
        engine: DeferredSailCatalogEngine,
    };
    (
        RegistryDeploymentPlan::new(&previous, &target, &binding).unwrap(),
        catalog,
        Principal::new("did:example:operator", PrincipalKind::Agent).unwrap(),
    )
}

#[tokio::test]
async fn deployment_applies_all_pins_then_restarts_without_repeating_completed_tables() {
    let (plan, catalog, principal) = fixture(false);
    assert_eq!(
        plan.apply(&catalog, &principal).await.unwrap().status(),
        RegistryDeploymentStatus::Ready
    );
    assert_eq!(
        plan.apply(&catalog, &principal).await.unwrap().status(),
        RegistryDeploymentStatus::Ready
    );
    let state = catalog.state.lock().unwrap();
    assert_eq!(state.commits.len(), 1);
    assert_eq!(state.creates, ["transactions"]);
    assert_eq!(state.commits[0].1["updates"].as_array().unwrap().len(), 1);
    assert_eq!(state.commits[0].1["updates"][0]["action"], "set-properties");
}

#[tokio::test]
async fn uncertain_write_stops_then_reconciliation_recovers_without_repeating_it() {
    let (plan, catalog, principal) = fixture(false);
    catalog.state.lock().unwrap().failure = Failure::LoseNextReply;
    assert!(matches!(
        plan.apply(&catalog, &principal).await,
        Err(RegistryApplyError::RequiresReconciliation { .. })
    ));
    assert!(catalog.state.lock().unwrap().creates.is_empty());
    assert_eq!(
        plan.reconcile(&catalog, &principal).await.unwrap().status(),
        RegistryDeploymentStatus::Pending
    );
    assert_eq!(
        plan.apply(&catalog, &principal).await.unwrap().status(),
        RegistryDeploymentStatus::Ready
    );
    assert_eq!(catalog.state.lock().unwrap().commits.len(), 1);
}

#[tokio::test]
async fn complete_preflight_rejects_later_drift_and_missing_tokens_before_any_write() {
    let (plan, mut catalog, principal) = fixture(false);
    catalog.tokens = Tokens::Absent;
    assert!(matches!(
        plan.apply(&catalog, &principal).await,
        Err(RegistryApplyError::ConditionalCommitUnavailable)
    ));
    catalog.tokens = Tokens::Present;
    catalog
        .state
        .lock()
        .unwrap()
        .tables
        .insert("transactions".into(), json!({}));
    assert!(matches!(
        plan.apply(&catalog, &principal).await,
        Err(RegistryApplyError::Drift(_))
    ));
    let state = catalog.state.lock().unwrap();
    assert!(state.commits.is_empty() && state.creates.is_empty());
}

#[tokio::test]
async fn concurrent_catalog_change_is_rejected_by_the_owner_and_stops_later_writes() {
    let (plan, catalog, principal) = fixture(false);
    catalog.state.lock().unwrap().failure = Failure::ChangeNextState;
    assert!(matches!(
        plan.apply(&catalog, &principal).await,
        Err(RegistryApplyError::RequiresReconciliation { .. })
    ));
    assert!(catalog.state.lock().unwrap().creates.is_empty());
    assert_eq!(
        plan.reconcile(&catalog, &principal).await.unwrap().status(),
        RegistryDeploymentStatus::Blocked
    );
}

#[tokio::test]
async fn additive_schema_update_preserves_ids_and_rejects_retired_id_reuse() {
    let (plan, catalog, principal) = fixture(true);
    assert_eq!(
        plan.apply(&catalog, &principal).await.unwrap().status(),
        RegistryDeploymentStatus::Ready
    );
    {
        let state = catalog.state.lock().unwrap();
        let request = &state.commits[0].1;
        assert_eq!(request["updates"][0]["action"], "add-schema");
        assert_eq!(request["updates"][0]["schema"]["schema-id"], 1);
        assert_eq!(request["updates"][0]["schema"]["fields"][0]["id"], 1);
        assert_eq!(request["updates"][0]["schema"]["fields"][1]["id"], 2);
        assert_eq!(
            request["requirements"][2],
            json!({"type":"assert-last-assigned-field-id","last-assigned-field-id":1})
        );
    }
    let (plan, catalog, principal) = fixture(true);
    catalog
        .state
        .lock()
        .unwrap()
        .tables
        .get_mut("customers")
        .unwrap()["last-column-id"] = json!(2);
    assert!(matches!(
        plan.apply(&catalog, &principal).await,
        Err(RegistryApplyError::Preflight(_))
    ));
    assert!(catalog.state.lock().unwrap().commits.is_empty());
}
