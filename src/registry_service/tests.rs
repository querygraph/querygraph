use super::*;

#[cfg(unix)]
mod transport;

#[tokio::test]
async fn signed_discovery_never_loads_catalog_or_accepts_a_scan_signature() {
    let (service, catalog) = setup(Verdict::Allow, BackendMode::Available, false);
    let intent = json!({"purpose":"analytics","limit":10}).to_string();
    let payload = json!({"bodySha256":format!("{:x}",Sha256::digest(intent.as_bytes()))});
    let scan_signature = PyTypeDidEnvelope::signed(
        "registry-test-agent",
        "did:example:registry",
        "invoke",
        "/mcp/plan_pinax_scan",
        payload.clone(),
    );
    assert!(matches!(
        service.discover(&intent, &scan_signature).await,
        Err(RegistryServiceError::Authentication)
    ));
    let discovery_signature = PyTypeDidEnvelope::signed(
        "registry-test-agent",
        "did:example:registry",
        "invoke",
        "/mcp/discover_pinax_tables",
        payload,
    );
    let page = service
        .discover(&intent, &discovery_signature)
        .await
        .expect("discovery");
    let page = serde_json::to_value(page).expect("page");
    assert_eq!(page["tables"][0]["name"], "customers");
    assert_eq!(page["tables"][0]["columns"][0]["name"], "customer_id");
    assert_eq!(catalog.loads.load(Ordering::Relaxed), 0);
    assert!(catalog.requests.lock().expect("requests").is_empty());
}
use lakecat_core::sail::{
    CommitPlan, CommitPreparationRequest, FetchScanTasksPlan, FetchScanTasksRequest,
    ScanPlanningRequest,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use typesec_core::{
    ResourceId,
    policy::{PolicyResult, SubjectId},
};

#[tokio::test]
async fn mcp_scan_uses_the_attached_host_and_returns_only_planning_evidence() {
    let (service, _) = setup(Verdict::Allow, BackendMode::Available, false);
    let mut server = crate::mcp::McpServer::new().with_registry_service(Arc::new(service));
    let init = json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{
        "protocolVersion":crate::mcp::MCP_PROTOCOL_VERSION,"capabilities":{},"clientInfo":{"name":"test","version":"1"}}});
    server
        .handle_line_async(&init.to_string())
        .await
        .expect("initialize response");
    server
        .handle_line_async(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
        .await;
    let (intent, envelope) = request("analytics");
    let request = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
        "name":"plan_pinax_scan","arguments":{"intent":intent,"envelope":envelope}}});
    let response = server
        .handle_line_async(&request.to_string())
        .await
        .expect("tool response");
    let response: Value = serde_json::from_str(&response).expect("response JSON");
    assert_eq!(response["result"]["isError"], false);
    assert_eq!(response["result"]["structuredContent"]["snapshot_id"], 42);
    assert!(!response.to_string().contains("private"));
    assert!(
        response["result"]["structuredContent"]
            .get("rows")
            .is_none()
    );
}

pub(super) fn registry() -> Registry {
    Registry::from_json(br#"{
        "version":"pinax.v1","enterprise":"acme","tables":[{
            "name":"customers","revision":1,"profile":"custom",
            "metadata":{"owner":"platform","steward":"data","description":"Customer identifiers","retention_days":30},
            "security":{"policy":"enterprise","revision":1,"classification":"internal","purposes":["analytics"]},
            "columns":[{"id":1,"name":"customer_id","data_type":{"kind":"string"},"nullability":"required",
                "semantic":"customer.id","description":"Customer identifier","classification":"internal"}],
            "primary_key":[1]
        }]}"#).expect("valid test registry")
}

enum Verdict {
    Allow,
    Deny,
    Unresolved,
}
struct Policy(Verdict);
impl PolicyEngine for Policy {
    fn check(&self, _: &SubjectId, _: &str, _: &ResourceId) -> PolicyResult {
        match self.0 {
            Verdict::Allow => PolicyResult::Allow,
            Verdict::Deny => PolicyResult::Deny("test denial".into()),
            Verdict::Unresolved => PolicyResult::delegate("test", "unresolved"),
        }
    }
}

enum BackendMode {
    Available,
    Unavailable,
    WrongSnapshot,
    ChangedRegistry,
}
struct Catalog {
    metadata: Value,
    loads: AtomicUsize,
    requests: Mutex<Vec<ScanPlanningRequest>>,
    mode: BackendMode,
}

#[async_trait]
impl RegistryCatalog for Catalog {
    async fn load_metadata(&self, _: &TableIdent, _: &Principal) -> LakeCatResult<Value> {
        let previous = self.loads.fetch_add(1, Ordering::Relaxed);
        let mut metadata = self.metadata.clone();
        if previous > 0 && matches!(self.mode, BackendMode::ChangedRegistry) {
            metadata["properties"]["pinax.registry-digest"] = json!("changed-during-planning");
        }
        Ok(metadata)
    }
    fn engine(&self) -> &dyn SailCatalogEngine {
        self
    }
}

#[async_trait]
impl SailCatalogEngine for Catalog {
    async fn prepare_commit(&self, _: CommitPreparationRequest) -> LakeCatResult<CommitPlan> {
        Err(LakeCatError::NotSupported("read-only fixture".into()))
    }
    async fn fetch_scan_tasks(
        &self,
        _: FetchScanTasksRequest,
    ) -> LakeCatResult<FetchScanTasksPlan> {
        Err(LakeCatError::NotSupported("read-only fixture".into()))
    }
    async fn plan_scan(&self, request: ScanPlanningRequest) -> LakeCatResult<ScanPlan> {
        let snapshot_id = request.snapshot_id;
        self.requests.lock().expect("fixture mutex").push(request);
        match self.mode {
            BackendMode::Unavailable => Err(LakeCatError::NotSupported("Sail unavailable".into())),
            BackendMode::Available | BackendMode::WrongSnapshot | BackendMode::ChangedRegistry => {
                Ok(ScanPlan {
                    snapshot_id: match self.mode {
                        BackendMode::WrongSnapshot => Some(43),
                        _ => snapshot_id,
                    },
                    planned_by: "fixture".into(),
                    scan_tasks: vec![json!({"private_path":"s3://private/customer.parquet"})],
                    residual_filter: None,
                })
            }
        }
    }
}

fn setup(verdict: Verdict, mode: BackendMode, drift: bool) -> (RegistryService, Arc<Catalog>) {
    let registry = registry();
    let binding = CatalogBinding::new("warehouse").expect("warehouse");
    let create = binding
        .create_plan(&registry, "customers")
        .expect("create plan");
    let mut metadata = lakecat_core::sail::initial_table_metadata(
        "11111111-1111-1111-1111-111111111111",
        "s3://private/customers",
        &create.body()["schema"],
        None,
        None,
        &create.body()["properties"],
    );
    metadata["current-snapshot-id"] = json!(42);
    if drift {
        metadata["properties"]["pinax.registry-digest"] = json!("stale");
    }
    let catalog = Arc::new(Catalog {
        metadata,
        loads: AtomicUsize::new(0),
        requests: Mutex::new(Vec::new()),
        mode,
    });
    let policy =
        RegistryPolicy::new("enterprise".into(), 1, Arc::new(Policy(verdict))).expect("policy");
    let service = RegistryService::new(
        registry,
        binding,
        policy,
        catalog.clone(),
        "did:example:registry".into(),
    )
    .expect("service");
    (service, catalog)
}

fn request(purpose: &str) -> (String, PyTypeDidEnvelope) {
    let intent =
        json!({"table":"customers","columns":["customer_id"],"purpose":purpose,"limit":10})
            .to_string();
    let envelope = PyTypeDidEnvelope::signed(
        "registry-test-agent",
        "did:example:registry",
        "invoke",
        "/mcp/plan_pinax_scan",
        json!({"bodySha256":format!("{:x}",Sha256::digest(intent.as_bytes()))}),
    );
    (intent, envelope)
}

#[tokio::test]
async fn signed_intent_binds_identity_and_hides_backend_tasks() {
    let (service, catalog) = setup(Verdict::Allow, BackendMode::Available, false);
    let (intent, envelope) = request("analytics");
    let prepared = service
        .plan_scan(&intent, &envelope)
        .await
        .expect("authorized plan");
    assert_eq!(prepared.principal().subject, envelope.sender);
    let summary = serde_json::to_value(prepared.summary()).expect("summary JSON");
    assert_eq!(summary["snapshot_id"], 42);
    assert_eq!(summary["columns"], json!(["customer_id"]));
    assert!(!summary.to_string().contains("private"));
    let calls = catalog.requests.lock().expect("requests");
    assert_eq!(calls[0].projection, ["customer_id"]);
    assert_eq!(calls[0].limit, Some(10));
    assert_eq!(calls[0].principal.subject, envelope.sender);
}

#[tokio::test]
async fn tampered_body_or_identity_fails_before_catalog_access() {
    let (service, catalog) = setup(Verdict::Allow, BackendMode::Available, false);
    let (intent, mut envelope) = request("analytics");
    assert!(matches!(
        service.plan_scan(&(intent.clone() + " "), &envelope).await,
        Err(RegistryServiceError::Authentication)
    ));
    envelope.sender = "did:example:someone-else".into();
    assert!(matches!(
        service.plan_scan(&intent, &envelope).await,
        Err(RegistryServiceError::Authentication)
    ));
    assert_eq!(catalog.loads.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn denial_unresolved_purpose_and_drift_never_plan() {
    for (verdict, purpose, drift) in [
        (Verdict::Deny, "analytics", false),
        (Verdict::Unresolved, "analytics", false),
        (Verdict::Allow, "advertising", false),
        (Verdict::Allow, "analytics", true),
    ] {
        let expected_unresolved = matches!(verdict, Verdict::Unresolved);
        let (service, catalog) = setup(verdict, BackendMode::Available, drift);
        let (intent, envelope) = request(purpose);
        let failure = service
            .plan_scan(&intent, &envelope)
            .await
            .err()
            .expect("rejected");
        match failure {
            RegistryServiceError::Scan(PinaxScanError::Denied) => {
                assert!(!drift && !expected_unresolved)
            }
            RegistryServiceError::Scan(PinaxScanError::Drift) => assert!(drift),
            RegistryServiceError::Scan(PinaxScanError::Unresolved) => {
                assert!(expected_unresolved)
            }
            other => panic!("unexpected rejection: {other:?}"),
        }
        assert!(catalog.requests.lock().expect("requests").is_empty());
    }
}

#[tokio::test]
async fn backend_failure_and_changed_snapshot_are_not_successful_empty_results() {
    let (intent, envelope) = request("analytics");
    let (service, _) = setup(Verdict::Allow, BackendMode::Unavailable, false);
    assert!(matches!(
        service.plan_scan(&intent, &envelope).await,
        Err(RegistryServiceError::Scan(PinaxScanError::Backend(
            LakeCatError::NotSupported(_)
        )))
    ));
    let (service, _) = setup(Verdict::Allow, BackendMode::WrongSnapshot, false);
    assert!(matches!(
        service.plan_scan(&intent, &envelope).await,
        Err(RegistryServiceError::Scan(PinaxScanError::Drift))
    ));
    let (service, _) = setup(Verdict::Allow, BackendMode::ChangedRegistry, false);
    assert!(matches!(
        service.plan_scan(&intent, &envelope).await,
        Err(RegistryServiceError::Scan(PinaxScanError::Drift))
    ));
}

mod ontology;
