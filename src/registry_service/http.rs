//! LakeCat REST adapter with a deployment-owned TypeDID credential.
//!
//! The remote LakeCat verifier authenticates the credential. The local adapter
//! confines its use to the configured principal and origin, rejects redirects,
//! bounds reads, and never falls back to an unsigned identity header.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use lakecat_core::sail::{
    CommitPlan, CommitPreparationRequest, FetchScanTasksPlan, FetchScanTasksRequest,
    SailCatalogEngine, ScanPlan, ScanPlanningRequest,
};
use lakecat_core::{LakeCatError, LakeCatResult, Principal, PrincipalKind, TableIdent};
use reqwest::{
    Client, Url,
    header::{HeaderMap, HeaderValue},
};
use serde::Deserialize;
use serde_json::{Value, json};

use super::RegistryCatalog;
use super::deployment::catalog::{
    CatalogTableState, RegistryCatalogCommit, RegistryDeploymentCatalog,
};

const MAX_CATALOG_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
pub mod credentials;

/// Authenticated LakeCat metadata and planning client for one configured agent.
/// The server must have a real TypeDID verifier; its conservative default rejects
/// these credentials. This adapter does not implement row execution.
pub struct LakeCatHttpCatalog {
    origin: Url,
    client: Client,
    principal: Principal,
    credentials: Arc<dyn credentials::LakeCatCredentials>,
}

impl LakeCatHttpCatalog {
    /// Configure a trusted origin and single-use TypeSec envelope for an agent.
    ///
    /// Credentials are supplied by the host, not by MCP arguments. LakeCat
    /// verifies the envelope and must resolve it to `subject`. Use HTTPS outside
    /// isolated local development. This constructor permits only one request;
    /// use [`Self::with_credentials`] for multi-request operations. Failed
    /// reads/plans are never retried automatically.
    ///
    /// # Errors
    /// Rejects non-origin URLs, embedded credentials, unsupported schemes,
    /// malformed/oversized credentials, and invalid principal/header values.
    pub fn new(origin: Url, subject: String, typedid_envelope: &str) -> LakeCatResult<Self> {
        if typedid_envelope.is_empty() || typedid_envelope.len() > 32 * 1024 {
            return Err(LakeCatError::InvalidArgument(
                "invalid LakeCat credential size".into(),
            ));
        }
        let envelope: Value = serde_json::from_str(typedid_envelope)
            .map_err(|_| LakeCatError::InvalidArgument("invalid LakeCat credential JSON".into()))?;
        if !envelope.is_object() {
            return Err(LakeCatError::InvalidArgument(
                "LakeCat credential must be an envelope object".into(),
            ));
        }
        HeaderValue::from_str(typedid_envelope).map_err(|_| {
            LakeCatError::InvalidArgument("invalid LakeCat credential header".into())
        })?;
        Self::with_credentials(
            origin,
            subject,
            Arc::new(credentials::OneShotEnvelope(std::sync::Mutex::new(Some(
                typedid_envelope.into(),
            )))),
        )
    }

    /// Configure a host-owned provider that mints a fresh envelope per request.
    ///
    /// # Errors
    /// Rejects non-origin URLs and invalid principal/header values.
    pub fn with_credentials(
        origin: Url,
        subject: String,
        credentials: Arc<dyn credentials::LakeCatCredentials>,
    ) -> LakeCatResult<Self> {
        if !matches!(origin.scheme(), "http" | "https")
            || origin.host_str().is_none()
            || !origin.username().is_empty()
            || origin.password().is_some()
            || origin.query().is_some()
            || origin.fragment().is_some()
            || origin.path() != "/"
        {
            return Err(LakeCatError::InvalidArgument(
                "LakeCat URL must be an HTTP(S) origin".into(),
            ));
        }
        let principal = Principal::new(subject, PrincipalKind::Agent)?;
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-lakecat-agent-did",
            HeaderValue::from_str(&principal.subject).map_err(|_| {
                LakeCatError::InvalidArgument("invalid LakeCat subject header".into())
            })?,
        );
        let client = Client::builder()
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| LakeCatError::Internal("cannot configure LakeCat client".into()))?;
        Ok(Self {
            origin,
            client,
            principal,
            credentials,
        })
    }

    fn table_url(&self, table: &TableIdent, principal: &Principal) -> LakeCatResult<Url> {
        if principal != &self.principal {
            return Err(LakeCatError::Forbidden(
                "catalog credential does not belong to this principal".into(),
            ));
        }
        let mut url = self.origin.clone();
        url.path_segments_mut()
            .map_err(|_| LakeCatError::InvalidArgument("invalid catalog origin".into()))?
            .clear()
            .extend([
                "catalog",
                "v1",
                table.warehouse.as_str(),
                "namespaces",
                &table.namespace.parts().join("\u{1f}"),
                "tables",
                table.name.as_str(),
            ]);
        Ok(url)
    }

    async fn read_json(&self, request: reqwest::RequestBuilder) -> LakeCatResult<Value> {
        self.read_response(request).await.map(|(body, _)| body)
    }

    async fn read_response(
        &self,
        request: reqwest::RequestBuilder,
    ) -> LakeCatResult<(Value, Option<String>)> {
        let mut request = request
            .build()
            .map_err(|_| LakeCatError::InvalidArgument("invalid LakeCat request".into()))?;
        let credential = self.credentials.envelope(
            request.method().as_str(),
            request.url(),
            request
                .body()
                .and_then(reqwest::Body::as_bytes)
                .unwrap_or_default(),
        )?;
        if credential.len() > 32 * 1024 {
            return Err(LakeCatError::InvalidArgument(
                "LakeCat credential exceeds size limit".into(),
            ));
        }
        let mut header = HeaderValue::from_str(&credential).map_err(|_| {
            LakeCatError::InvalidArgument("invalid LakeCat credential header".into())
        })?;
        header.set_sensitive(true);
        request
            .headers_mut()
            .insert("x-lakecat-typedid-envelope", header);
        let mut response = self
            .client
            .execute(request)
            .await
            .map_err(|_| LakeCatError::Internal("LakeCat request failed".into()))?;
        let status = response.status().as_u16();
        let state_token = {
            let mut values = response.headers().get_all("x-lakecat-table-state").iter();
            let first = values.next();
            if values.next().is_none() {
                first
                    .and_then(|value| value.to_str().ok())
                    .filter(|value| value.len() <= 256)
                    .map(str::to_owned)
            } else {
                None
            }
        };
        match status {
            200..=299 | 404 => {}
            401 | 403 => return Err(LakeCatError::Forbidden("LakeCat request denied".into())),
            409 => return Err(LakeCatError::Conflict("LakeCat state changed".into())),
            _ => {
                return Err(LakeCatError::Internal(
                    "LakeCat returned an unsuccessful status".into(),
                ));
            }
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_CATALOG_RESPONSE_BYTES as u64)
        {
            return Err(LakeCatError::Internal(
                "LakeCat response exceeds size limit".into(),
            ));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| LakeCatError::Internal("LakeCat response read failed".into()))?
        {
            if bytes.len() + chunk.len() > MAX_CATALOG_RESPONSE_BYTES {
                return Err(LakeCatError::Internal(
                    "LakeCat response exceeds size limit".into(),
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        let body: Value = serde_json::from_slice(&bytes)
            .map_err(|_| LakeCatError::Internal("invalid LakeCat response JSON".into()))?;
        if status == 404 {
            if body["error"]["type"] == "NoSuchTableException" && body["error"]["code"] == 404 {
                return Err(LakeCatError::NotFound {
                    object: "table",
                    name: "requested table".into(),
                });
            }
            return Err(LakeCatError::Internal(
                "LakeCat did not establish table absence".into(),
            ));
        }
        Ok((body, state_token))
    }
}

#[async_trait]
impl RegistryDeploymentCatalog for LakeCatHttpCatalog {
    async fn load_state(
        &self,
        table: &TableIdent,
        principal: &Principal,
    ) -> LakeCatResult<CatalogTableState> {
        let url = self.table_url(table, principal)?;
        let (mut response, token) = self.read_response(self.client.get(url)).await?;
        let metadata = response
            .get_mut("metadata")
            .map(Value::take)
            .ok_or_else(|| LakeCatError::Internal("LakeCat omitted table metadata".into()))?;
        CatalogTableState::new(metadata, token)
    }

    async fn create_registry_table(
        &self,
        table: &TableIdent,
        principal: &Principal,
        request: &pinax::adapters::CatalogCreatePlan,
    ) -> LakeCatResult<()> {
        let mut url = self.table_url(table, principal)?;
        url.path_segments_mut()
            .map_err(|_| LakeCatError::InvalidArgument("invalid catalog URL".into()))?
            .pop();
        if request.path() != url.path() || request.body()["name"] != table.name.as_str() {
            return Err(LakeCatError::InvalidArgument(
                "registry create request has a different table scope".into(),
            ));
        }
        self.read_json(self.client.post(url).json(request.body()))
            .await?;
        Ok(())
    }

    async fn commit_registry_table(
        &self,
        table: &TableIdent,
        principal: &Principal,
        token: &str,
        request: &RegistryCatalogCommit,
        idempotency_key: &str,
    ) -> LakeCatResult<()> {
        let url = self.table_url(table, principal)?;
        self.read_json(
            self.client
                .post(url)
                .header("x-lakecat-expected-table-state", token)
                .header("Idempotency-Key", idempotency_key)
                .json(request),
        )
        .await?;
        Ok(())
    }
}

#[async_trait]
impl RegistryCatalog for LakeCatHttpCatalog {
    async fn load_execution_state(
        &self,
        table: &TableIdent,
        principal: &Principal,
    ) -> LakeCatResult<CatalogTableState> {
        RegistryDeploymentCatalog::load_state(self, table, principal).await
    }

    async fn execute_governed(
        &self,
        table: &TableIdent,
        principal: &Principal,
        state: &CatalogTableState,
        intent: &pinax::adapters::ScanIntent,
        snapshot: i64,
    ) -> LakeCatResult<Value> {
        let mut url = self.table_url(table, principal)?;
        let path = url
            .path()
            .strip_prefix("/catalog/v1/")
            .ok_or_else(|| LakeCatError::Internal("invalid catalog execution scope".into()))?
            .to_owned();
        url.set_path(&format!("/querygraph/v1/{path}/execute"));
        let token = state.token().ok_or_else(|| {
            LakeCatError::NotSupported("catalog execution state is unavailable".into())
        })?;
        self.read_json(self.client.post(url).header("x-lakecat-expected-table-state", token).json(&json!({
            "projection":intent.columns,"purpose":intent.purpose,"limit":intent.limit,"snapshot_id":snapshot
        }))).await
    }
    async fn load_metadata(
        &self,
        table: &TableIdent,
        principal: &Principal,
    ) -> LakeCatResult<Value> {
        let url = self.table_url(table, principal)?;
        let mut response = self.read_json(self.client.get(url)).await?;
        response
            .get_mut("metadata")
            .filter(|metadata| metadata.is_object())
            .map(Value::take)
            .ok_or_else(|| LakeCatError::Internal("LakeCat omitted table metadata".into()))
    }

    fn engine(&self) -> &dyn SailCatalogEngine {
        self
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct PlanResponse {
    table: ResponseTable,
    status: String,
    snapshot_id: Option<i64>,
    planned_by: String,
    #[serde(default)]
    lakecat_plan_tasks: Vec<Value>,
    residual_filter: Option<Value>,
}

#[derive(Deserialize)]
struct ResponseTable {
    namespace: Vec<String>,
    name: String,
}

#[async_trait]
impl SailCatalogEngine for LakeCatHttpCatalog {
    async fn plan_scan(&self, request: ScanPlanningRequest) -> LakeCatResult<ScanPlan> {
        let mut url = self.table_url(&request.table, &request.principal)?;
        url.path_segments_mut()
            .map_err(|_| LakeCatError::InvalidArgument("invalid catalog URL".into()))?
            .push("plan");
        let response = self
            .read_json(self.client.post(url).json(&json!({
                "projection":request.projection,"filters":request.filters,"limit":request.limit,
                "snapshot-id":request.snapshot_id,"start-snapshot-id":request.start_snapshot_id,
                "end-snapshot-id":request.end_snapshot_id
            })))
            .await?;
        let response: PlanResponse = serde_json::from_value(response)
            .map_err(|_| LakeCatError::Internal("invalid LakeCat planning response".into()))?;
        if response.status != "completed"
            || response.snapshot_id != request.snapshot_id
            || response.table.namespace != request.table.namespace.parts()
            || response.table.name != request.table.name.as_str()
        {
            return Err(LakeCatError::Conflict(
                "LakeCat plan did not complete at the requested snapshot".into(),
            ));
        }
        Ok(ScanPlan {
            planned_by: response.planned_by,
            snapshot_id: response.snapshot_id,
            scan_tasks: response.lakecat_plan_tasks,
            residual_filter: response.residual_filter,
        })
    }

    async fn prepare_commit(&self, _: CommitPreparationRequest) -> LakeCatResult<CommitPlan> {
        Err(LakeCatError::NotSupported(
            "registry catalog adapter only reads and plans".into(),
        ))
    }

    async fn fetch_scan_tasks(
        &self,
        _: FetchScanTasksRequest,
    ) -> LakeCatResult<FetchScanTasksPlan> {
        Err(LakeCatError::NotSupported(
            "scan task fetch requires governed execution authorization".into(),
        ))
    }
}

#[cfg(test)]
mod tests;
