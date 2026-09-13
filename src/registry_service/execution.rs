//! Pinax authorization and catalog observation around owner-executed rows.
use super::*;
use lakecat_core::{content_hash_json, governed_scan::GovernedScanProof};

#[cfg(test)]
mod tests;

/// Bounded rows and owner evidence released after fresh Pinax/catalog checks.
/// Private fields prevent constructing this result from agent arguments.
#[derive(Serialize)]
pub struct RegistryExecution {
    #[serde(skip_serializing_if = "Option::is_none")]
    semantic_exports: Option<Value>,
    summary: RegistryScanSummary,
    rows: Vec<Value>,
    evidence: Value,
    evidence_digest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnerResult {
    snapshot_id: i64,
    columns: Vec<String>,
    rows: Vec<Value>,
    evidence: Value,
    evidence_digest: String,
}

impl RegistryService {
    /// Execute a signed Pinax intent through the authenticated catalog owner.
    /// Sign `invoke` on `/mcp/execute_pinax_scan`; planning signatures cannot
    /// authorize this operation. No caller-supplied tasks or paths are accepted.
    ///
    /// # Errors
    /// Rejects invalid identity/intent, denied scope, unavailable execution,
    /// changed catalog state, and inconsistent owner evidence. No partial rows
    /// are returned, and no request is retried.
    pub async fn execute_scan(
        &self,
        intent_json: &str,
        envelope: &PyTypeDidEnvelope,
    ) -> Result<RegistryExecution, RegistryServiceError> {
        let principal = self.authenticate(intent_json, envelope, RegistryOperation::Execute)?;
        let intent: ScanIntent = serde_json::from_str(intent_json)?;
        let governed = self.governed();
        governed.authorize_scan(&principal, &intent).await?;
        if !(1..=1000).contains(&intent.limit) {
            return Err(RegistryServiceError::Catalog(
                LakeCatError::InvalidArgument("execution row bound must be 1 to 1000".into()),
            ));
        }
        let table = self.binding.table_ident(&self.registry, &intent.table)?;
        let state = self
            .catalog
            .load_execution_state(&table, &principal)
            .await
            .map_err(RegistryServiceError::Catalog)?;
        let plan = governed
            .plan_scan(self.catalog.engine(), &principal, state.metadata(), &intent)
            .await?;
        let snapshot = plan.snapshot_id.ok_or(PinaxScanError::Drift)?;
        let value = self
            .catalog
            .execute_governed(&table, &principal, &state, &intent, snapshot)
            .await
            .map_err(RegistryServiceError::Catalog)?;
        let result: OwnerResult = serde_json::from_value(value).map_err(|_| {
            RegistryServiceError::Catalog(LakeCatError::Internal(
                "invalid owner execution response".into(),
            ))
        })?;
        validate_owner(
            &result,
            &table,
            &principal,
            &intent,
            snapshot,
            state.token(),
        )
        .map_err(RegistryServiceError::Catalog)?;
        let semantic_exports = if self.ontology.is_some() {
            let ontology = self.load_ontology().await?;
            let page = governed.describe_scan(&principal, &intent).await?;
            Some(ontology.export_rows(&page, &result.rows)?)
        } else {
            None
        };
        let current = self
            .catalog
            .load_execution_state(&table, &principal)
            .await
            .map_err(RegistryServiceError::Catalog)?;
        if current.metadata() != state.metadata() || current.token() != state.token() {
            return Err(PinaxScanError::Drift.into());
        }
        governed.authorize_scan(&principal, &intent).await?;
        Ok(RegistryExecution {
            semantic_exports,
            summary: RegistryScanSummary {
                enterprise: self.registry.enterprise().to_owned(),
                table: intent.table.clone(),
                revision: self.registry.table(&intent.table)?.revision,
                registry_digest: self.registry.digest()?,
                snapshot_id: snapshot,
                columns: intent.columns,
                purpose: intent.purpose,
                limit: intent.limit,
                planned_by: plan.planned_by,
            },
            rows: result.rows,
            evidence: result.evidence,
            evidence_digest: result.evidence_digest,
        })
    }
}

fn validate_owner(
    result: &OwnerResult,
    table: &TableIdent,
    principal: &Principal,
    intent: &ScanIntent,
    snapshot: i64,
    token: Option<&str>,
) -> LakeCatResult<()> {
    let invalid =
        || LakeCatError::Conflict("owner result does not match governed execution scope".into());
    let proof: GovernedScanProof =
        serde_json::from_value(result.evidence["proof"].clone()).map_err(|_| invalid())?;
    proof.validate_integrity().map_err(|_| invalid())?;
    if result.snapshot_id != snapshot
        || proof.snapshot_id() != snapshot
        || proof.table() != table
        || proof.principal_subject() != principal.subject
        || proof.purpose() != intent.purpose
        || proof.effective_projection() != intent.columns
        || result.columns != intent.columns
        || result.rows.len() as u64 > intent.limit
        || token.is_none()
        || result.evidence["catalog_state"].as_str() != token
        || !result.evidence["revalidated_at"]
            .as_str()
            .is_some_and(|timestamp| chrono::DateTime::parse_from_rfc3339(timestamp).is_ok())
        || result.rows.iter().any(|row| {
            row.as_object().is_none_or(|object| {
                object.len() != result.columns.len()
                    || result.columns.iter().any(|name| !object.contains_key(name))
            })
        })
    {
        return Err(invalid());
    }
    if result.evidence["row_digest"]
        != content_hash_json(&("lakecat.executed-rows.v1", &result.columns, &result.rows))?
        || result.evidence_digest
            != content_hash_json(&("lakecat.executed-result-evidence.v1", &result.evidence))?
    {
        return Err(invalid());
    }
    for path in [
        "/execution_authorization_digest",
        "/fresh_authorization_digest",
        "/fresh_policy_decision_digest",
        "/lineage/event_hash",
        "/lineage/open_lineage_hash",
    ] {
        if !result
            .evidence
            .pointer(path)
            .and_then(Value::as_str)
            .is_some_and(|hash| {
                hash.strip_prefix("sha256:").is_some_and(|hex| {
                    hex.len() == 64
                        && hex
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                })
            })
        {
            return Err(invalid());
        }
    }
    Ok(())
}
