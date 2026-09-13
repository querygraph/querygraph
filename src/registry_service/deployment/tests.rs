use super::*;
use async_trait::async_trait;
use lakecat_core::{
    LakeCatResult, PrincipalKind,
    sail::{DeferredSailCatalogEngine, SailCatalogEngine},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Mutex;

struct Catalog {
    metadata: BTreeMap<String, Value>,
    requests: Mutex<Vec<String>>,
    engine: DeferredSailCatalogEngine,
}

#[async_trait]
impl RegistryCatalog for Catalog {
    async fn load_metadata(&self, table: &TableIdent, _: &Principal) -> LakeCatResult<Value> {
        self.requests
            .lock()
            .expect("requests")
            .push(table.name.as_str().into());
        self.metadata
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

fn fixture() -> (Registry, Registry, CatalogBinding) {
    let previous = crate::registry_service::tests::registry();
    let mut document: pinax::RegistryDocument =
        serde_json::from_value(serde_json::to_value(&previous).expect("JSON")).expect("document");
    let mut new_table = document.tables[0].clone();
    new_table.name = "transactions".into();
    document.tables.push(new_table);
    (
        previous,
        Registry::new(document).expect("target"),
        CatalogBinding::new("warehouse").expect("binding"),
    )
}

fn metadata(registry: &Registry, binding: &CatalogBinding, table: &str) -> Value {
    let contract = binding.table_contract(registry, table).expect("contract");
    json!({"current-schema-id":0,"schemas":[contract.schema()],"properties":contract.properties()})
}

fn principal() -> Principal {
    Principal::new("did:example:operator", PrincipalKind::Agent).expect("principal")
}

fn catalog(metadata: BTreeMap<String, Value>) -> Catalog {
    Catalog {
        metadata,
        requests: Mutex::new(Vec::new()),
        engine: DeferredSailCatalogEngine,
    }
}

#[test]
fn unchanged_tables_are_included_when_the_whole_registry_digest_changes() {
    let (previous, target, binding) = fixture();
    let plan = RegistryDeploymentPlan::new(&previous, &target, &binding).expect("plan");
    let json = serde_json::to_value(plan).expect("plan JSON");
    assert_eq!(json["tables"].as_array().expect("tables").len(), 2);
    assert_eq!(json["tables"][0]["transition"]["operation"], "update");
    assert_ne!(
        json["tables"][0]["transition"]["previous"]["properties"]["pinax.registry-digest"],
        json["tables"][0]["target"]["properties"]["pinax.registry-digest"]
    );
    assert_eq!(json["tables"][1]["transition"]["operation"], "create");
}

#[tokio::test]
async fn reconciliation_distinguishes_partial_deployment_from_ready() {
    let (previous, target, binding) = fixture();
    let plan = RegistryDeploymentPlan::new(&previous, &target, &binding).expect("plan");
    let mut state = BTreeMap::from([(
        "customers".into(),
        metadata(&previous, &binding, "customers"),
    )]);
    let old = catalog(state.clone());
    let report = plan
        .reconcile(&old, &principal())
        .await
        .expect("reconciliation");
    assert_eq!(report.status(), RegistryDeploymentStatus::Pending);
    let json = serde_json::to_value(report).expect("JSON");
    assert_eq!(json["tables"][0]["status"], "pending_update");
    assert_eq!(json["tables"][1]["status"], "pending_create");
    assert_eq!(
        *old.requests.lock().expect("requests"),
        ["customers", "transactions"]
    );
    state.insert("customers".into(), metadata(&target, &binding, "customers"));
    let partial = plan
        .reconcile(&catalog(state.clone()), &principal())
        .await
        .expect("partial");
    assert_eq!(partial.status(), RegistryDeploymentStatus::Pending);
    state.insert(
        "transactions".into(),
        metadata(&target, &binding, "transactions"),
    );
    let complete = plan
        .reconcile(&catalog(state), &principal())
        .await
        .expect("complete observations");
    assert_eq!(complete.status(), RegistryDeploymentStatus::Ready);
}

#[tokio::test]
async fn ambiguous_write_is_reconciled_by_state_without_repeating_a_mutation() {
    let (previous, target, binding) = fixture();
    let plan = RegistryDeploymentPlan::new(&previous, &target, &binding).expect("plan");
    // The previous caller lost a write response, but both target bindings exist.
    let state = catalog(BTreeMap::from([
        ("customers".into(), metadata(&target, &binding, "customers")),
        (
            "transactions".into(),
            metadata(&target, &binding, "transactions"),
        ),
    ]));
    assert_eq!(
        plan.reconcile(&state, &principal())
            .await
            .expect("observed target")
            .status(),
        RegistryDeploymentStatus::Ready
    );
    assert_eq!(state.requests.lock().expect("requests").len(), 2);
}

#[tokio::test]
async fn drift_or_missing_predecessor_blocks_cutover() {
    let (previous, target, binding) = fixture();
    let plan = RegistryDeploymentPlan::new(&previous, &target, &binding).expect("plan");
    let absent = plan
        .reconcile(&catalog(BTreeMap::new()), &principal())
        .await
        .expect("missing observations");
    assert_eq!(absent.status(), RegistryDeploymentStatus::Blocked);
    let mut changed = metadata(&target, &binding, "customers");
    changed["schemas"][0]["fields"][0]["type"] = json!("long");
    let drift = plan
        .reconcile(
            &catalog(BTreeMap::from([("customers".into(), changed)])),
            &principal(),
        )
        .await
        .expect("drift observations");
    assert_eq!(drift.status(), RegistryDeploymentStatus::Blocked);
}
