use super::*;
use lakecat_core::governed_scan::{GovernedScanCatalogIdentity, GovernedScanProofEvidence};
use serde_json::json;

fn fixture() -> (OwnerResult, TableIdent, Principal, ScanIntent) {
    let table: TableIdent = serde_json::from_value(json!({
        "warehouse": "warehouse", "namespace": ["acme"], "name": "rows"
    }))
    .unwrap();
    let principal = Principal::new("did:example:agent", PrincipalKind::Agent).unwrap();
    let hash = content_hash_json(&json!({"fixture": true})).unwrap();
    let proof = GovernedScanProof::issue(GovernedScanProofEvidence {
        catalog_identity: GovernedScanCatalogIdentity::new("lakecat://warehouse").unwrap(),
        table: table.clone(),
        table_version: 0,
        snapshot_id: 42,
        plan_task_digest: hash.clone(),
        principal_subject: principal.subject.clone(),
        purpose: "analytics".into(),
        effective_projection: vec!["id".into()],
        identity_context_digest: hash.clone(),
        authorization_receipt_digest: hash.clone(),
        policy_decision_digest: hash.clone(),
    })
    .unwrap();
    let mut result = OwnerResult {
        snapshot_id: 42,
        columns: vec!["id".into()],
        rows: vec![json!({"id": 2})],
        evidence: json!({
            "proof": proof, "catalog_state": "state",
            "revalidated_at": "2026-09-12T12:00:00Z",
            "execution_authorization_digest": hash,
            "fresh_authorization_digest": hash,
            "fresh_policy_decision_digest": hash,
            "lineage": {"event_hash": hash, "open_lineage_hash": hash}
        }),
        evidence_digest: String::new(),
    };
    rehash(&mut result);
    let intent = ScanIntent {
        table: "rows".into(),
        columns: vec!["id".into()],
        purpose: "analytics".into(),
        limit: 1,
    };
    (result, table, principal, intent)
}

fn rehash(result: &mut OwnerResult) {
    result.evidence["row_digest"] = json!(
        content_hash_json(&("lakecat.executed-rows.v1", &result.columns, &result.rows)).unwrap()
    );
    result.evidence_digest =
        content_hash_json(&("lakecat.executed-result-evidence.v1", &result.evidence)).unwrap();
}

#[test]
fn accepts_bound_owner_result_and_rejects_changed_rows() {
    let (mut result, table, principal, intent) = fixture();
    assert!(validate_owner(&result, &table, &principal, &intent, 42, Some("state")).is_ok());
    result.rows[0]["id"] = json!(3);
    assert!(validate_owner(&result, &table, &principal, &intent, 42, Some("state")).is_err());
}

#[test]
fn rehashed_evidence_cannot_expand_scope_or_omit_release_evidence() {
    for mutation in 0..7 {
        let (mut result, table, principal, intent) = fixture();
        match mutation {
            0 => result.rows[0]["private_email"] = json!("protected"),
            1 => result.rows.push(json!({"id": 3})),
            2 => result.evidence["catalog_state"] = json!("different"),
            3 => result.evidence["revalidated_at"] = Value::Null,
            4 => result.evidence["lineage"]["event_hash"] = json!("invalid"),
            5 => result.snapshot_id = 43,
            _ => result.evidence["proof"]["purpose"] = json!("marketing"),
        }
        rehash(&mut result);
        assert!(validate_owner(&result, &table, &principal, &intent, 42, Some("state")).is_err());
    }
}

#[test]
fn valid_proof_cannot_be_reused_for_another_caller_or_observation() {
    let (result, table, principal, intent) = fixture();
    let other = Principal::new("did:example:other", PrincipalKind::Agent).unwrap();
    assert!(validate_owner(&result, &table, &other, &intent, 42, Some("state")).is_err());
    assert!(validate_owner(&result, &table, &principal, &intent, 43, Some("state")).is_err());
    assert!(validate_owner(&result, &table, &principal, &intent, 42, None).is_err());
}
