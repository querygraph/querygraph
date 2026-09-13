//! Isolated integration fixture: real LakeCat, Sail, and TypeSec authentication.
//! The runner generates its development-only Cargo manifest outside the repo.

use lakecat_core::{Namespace, Principal, PrincipalKind, TableIdent, TableName, WarehouseName};
use lakecat_graph::NoopCatalogGraphSink;
use lakecat_lineage::HashOnlyLineageSink;
use lakecat_security::typesec_integration::TypeSecGovernanceEngine;
use lakecat_service::{LakeCatState, app, typesec_typedid::TypeSecTypeDidVerifier};
use lakecat_store::{
    CatalogStore, MemoryCatalogStore, PolicyBinding, ProjectRecord, TableRecord, WarehouseRecord,
};
use std::{collections::BTreeMap, io::Write, path::PathBuf, sync::Arc};
use typesec::integrations::{
    DidMessageBody, StaticDidResolver, TypeDidConversation, TypeDidMode, TypeDidProfile,
};
use typesec::{Did, DidEnvelope, Ed25519DidKey, Ed25519DidKeyStore, TypeDidGateway};

mod read_mutation;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("fixture directory required")?,
    )
    .canonicalize()?;
    let seed = [7_u8; 32];
    let agent_key = Ed25519DidKey::from_seed(&seed);
    let service_key = Ed25519DidKey::from_seed(b"lakecat-live-registry-fixture");
    let mut public = vec![0xed, 0x01];
    public.extend_from_slice(&agent_key.signing_public());
    let agent = Did::parse(format!("did:key:z{}", bs58::encode(public).into_string()))?;
    let service = Did::key(service_key.signing_public());
    let resolver = StaticDidResolver::new()
        .with_document(agent_key.document(agent.clone()))
        .with_document(service_key.document(service.clone()));
    let recipient_document = service_key.document(service.clone());
    let keys = Ed25519DidKeyStore::new()
        .with_key(agent.clone(), agent_key)
        .with_key(service.clone(), service_key);
    if std::env::args().nth(2).as_deref() == Some("credential") {
        let method = std::env::args().nth(3).ok_or("method required")?;
        let path = std::env::args().nth(4).ok_or("path required")?;
        let body_hash = std::env::args().nth(5).ok_or("body hash required")?;
        let id = uuid::Uuid::new_v4().to_string();
        let payload = serde_json::to_vec(
            &serde_json::json!({"method":method,"path":path,"body_sha256":body_hash}),
        )?;
        let envelope = DidEnvelope::typedid(
            &id,
            agent.clone(),
            service.clone(),
            DidMessageBody::agent_message(format!("lakecat:http:{method}:{path}"), "internal"),
            TypeDidConversation::new(
                id.clone(),
                TypeDidMode::RequestReply,
                TypeDidProfile::ed25519_x25519_chacha20().id,
                "https",
            ),
            &payload,
            &resolver,
            &keys,
        )?;
        println!("{}", serde_json::to_string(&envelope)?);
        return Ok(());
    }
    std::fs::write(directory.join("agent.seed"), seed)?;
    std::fs::write(
        directory.join("recipient.json"),
        serde_json::to_vec(&recipient_document)?,
    )?;
    std::fs::write(
        directory.join("identity.json"),
        br#"{"seed":"agent.seed","recipient_document":"recipient.json"}"#,
    )?;
    let gateway = Arc::new(TypeDidGateway::new(
        Arc::new(resolver),
        Arc::new(keys),
        service,
    ));
    let warehouse = WarehouseName::new("warehouse")?;
    let namespace = Namespace::new(vec!["acme".into()])?;
    let principal = Principal::new(agent.to_string(), PrincipalKind::Agent)?;
    let store = MemoryCatalogStore::new();
    store
        .upsert_project(ProjectRecord::new(
            "fixture",
            None,
            None,
            BTreeMap::new(),
            principal.clone(),
        )?)
        .await?;
    let location = url::Url::from_directory_path(&directory)
        .map_err(|_| "fixture file URL")?
        .to_string();
    store
        .upsert_warehouse(WarehouseRecord::new(
            warehouse.clone(),
            "fixture",
            Some(location.clone()),
            BTreeMap::new(),
            principal,
        )?)
        .await?;
    store
        .create_namespace(&warehouse, namespace.clone())
        .await?;
    let seed_path = directory.join("seed.json");
    let mut execute_permission = ", table.execute_scan";
    let mut read_mutation = None;
    let mut successor_metadata = serde_json::Value::Null;
    let mut successor_location = String::new();
    if seed_path.exists() {
        let seed: serde_json::Value = serde_json::from_slice(&std::fs::read(seed_path)?)?;
        if let Some(value) = seed.get("read_mutation") {
            read_mutation = Some(serde_json::from_value::<read_mutation::ReadMutation>(
                value.clone(),
            )?);
            successor_metadata = seed["successor_metadata"].clone();
            successor_location = seed["successor_location"]
                .as_str()
                .ok_or("successor metadata pointer")?
                .into();
        }
        if seed["execute_allowed"] == false {
            execute_permission = "";
        }
        let ident = TableIdent::new(
            warehouse.clone(),
            namespace.clone(),
            TableName::new("rows")?,
        );
        store
            .create_table(TableRecord::new(
                ident.clone(),
                seed["location"].as_str().ok_or("seed location")?.into(),
                Some(
                    seed["metadata_location"]
                        .as_str()
                        .ok_or("seed metadata pointer")?
                        .into(),
                ),
                seed["metadata"].clone(),
                Principal::new(agent.to_string(), PrincipalKind::Agent)?,
            ))
            .await?;
        store.upsert_policy_binding(PolicyBinding::new("fixture-rows", warehouse.clone(), Some(namespace.clone()), Some(ident.name.clone()), true,
            serde_json::json!({"lakecat:read-restriction":{"allowed-columns":["id"],"row-predicate":{"type":"eq","term":"tenant","value":"acme"}},
                "permission":[{"action":"read","constraint":[{"leftOperand":"purpose","operator":"eq","rightOperand":"analytics"}]}]}))?).await?;
    }
    let resources = ["customers", "transactions", "rows"]
        .iter()
        .map(|name| {
            Ok(
                TableIdent::new(warehouse.clone(), namespace.clone(), TableName::new(*name)?)
                    .stable_id(),
            )
        })
        .collect::<Result<Vec<_>, lakecat_core::LakeCatError>>()?;
    let policy = format!(
        "roles:\n  - name: fixture_operator\n    permissions: [table.create, table.load, table.commit, table.plan_scan{execute_permission}]\n    resources: {}\nassignments:\n  - subject: {}\n    roles: [fixture_operator]\n",
        serde_json::to_string(&resources)?,
        serde_json::to_string(&agent.to_string())?
    );
    let governance = TypeSecGovernanceEngine::rbac_from_yaml(&policy)?;
    let state = LakeCatState::new(warehouse, store).with_table_location_root(location)?;
    let sail: Arc<dyn lakecat_core::sail::SailCatalogEngine> = match read_mutation {
        Some(mutation) => Arc::new(read_mutation::MutatingReader {
            inner: state.sail.clone(),
            store: state.store.clone(),
            mutation,
            marker: directory.join("read-mutation-count"),
            successor_metadata,
            successor_location,
        }),
        None => state.sail.clone(),
    };
    let state = state
        .with_integrations(
            sail,
            governance,
            NoopCatalogGraphSink::new(),
            HashOnlyLineageSink::new(),
        )
        .with_typedid_verifier(TypeSecTypeDidVerifier::new(gateway));
    let port = std::env::var("QG_FIXTURE_PORT")
        .ok()
        .map(|value| value.parse::<u16>())
        .transpose()?
        .unwrap_or(0);
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    println!(
        "{}",
        serde_json::json!({"origin":format!("http://{}/", listener.local_addr()?),"subject":agent.to_string()})
    );
    std::io::stdout().flush()?;
    axum::serve(listener, app(state)).await?;
    Ok(())
}
