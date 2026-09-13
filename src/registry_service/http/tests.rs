use super::*;
use lakecat_core::{Namespace, TableName, WarehouseName};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

fn table() -> TableIdent {
    TableIdent::new(
        WarehouseName::new("warehouse").expect("warehouse"),
        Namespace::new(vec!["acme".into()]).expect("namespace"),
        TableName::new("customers").expect("table"),
    )
}

fn fixture(status: &str, body: &str) -> (Url, thread::JoinHandle<String>) {
    fixture_with_headers(status, body, "")
}

fn fixture_with_headers(
    status: &str,
    body: &str,
    headers: &str,
) -> (Url, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("local listener");
    let url = Url::parse(&format!(
        "http://{}/",
        listener.local_addr().expect("address")
    ))
    .expect("URL");
    let status = status.to_owned();
    let body = body.to_owned();
    let extra_headers = headers.to_owned();
    let task = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("request");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read timeout");
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let count = stream.read(&mut buffer).expect("read request");
            assert!(count > 0);
            bytes.extend_from_slice(&buffer[..count]);
            if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..end]);
                let length: usize = headers
                    .lines()
                    .find_map(|line| {
                        line.to_lowercase()
                            .strip_prefix("content-length: ")
                            .map(str::to_owned)
                    })
                    .map(|value| value.parse().expect("content length"))
                    .unwrap_or(0);
                if bytes.len() >= end + 4 + length {
                    break;
                }
            }
            assert!(bytes.len() < 64 * 1024);
        }
        write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra_headers}Connection: close\r\n\r\n{body}", body.len()).expect("response");
        String::from_utf8(bytes).expect("request UTF8")
    });
    (url, task)
}

fn principal() -> Principal {
    Principal::new("did:example:agent", PrincipalKind::Agent).expect("principal")
}

#[tokio::test]
async fn catalog_state_is_bound_to_one_read_and_missing_support_is_explicit() {
    let token = format!("sha256:{}", "a".repeat(64));
    let headers = format!("x-lakecat-table-state: {token}\r\n");
    let (origin, task) = fixture_with_headers(
        "200 OK",
        r#"{"metadata":{"current-schema-id":3}}"#,
        &headers,
    );
    let catalog = LakeCatHttpCatalog::new(origin, principal().subject, "{}").unwrap();
    let state = catalog.load_state(&table(), &principal()).await.unwrap();
    assert_eq!(state.token(), Some(token.as_str()));
    assert_eq!(state.metadata()["current-schema-id"], 3);
    task.join().unwrap();

    let (origin, task) = fixture("200 OK", r#"{"metadata":{"current-schema-id":3}}"#);
    let catalog = LakeCatHttpCatalog::new(origin, principal().subject, "{}").unwrap();
    assert!(
        catalog
            .load_state(&table(), &principal())
            .await
            .unwrap()
            .token()
            .is_none()
    );
    task.join().unwrap();
}

#[tokio::test]
async fn scan_request_preserves_projection_snapshot_and_limit() {
    let (origin, task) = fixture(
        "200 OK",
        r#"{
        "table":{"namespace":["acme"],"name":"customers"},
        "status":"completed","snapshot-id":42,"planned-by":"lakecat",
        "lakecat-plan-tasks":[{"task":"opaque"}],"residual-filter":null
    }"#,
    );
    let catalog = LakeCatHttpCatalog::new(origin, principal().subject, "{}").expect("client");
    let planned = catalog
        .plan_scan(ScanPlanningRequest {
            table: table(),
            principal: principal(),
            metadata_location: None,
            table_metadata: json!({}),
            projection: vec!["customer_id".into()],
            filters: vec![],
            limit: Some(10),
            snapshot_id: Some(42),
            start_snapshot_id: None,
            end_snapshot_id: None,
        })
        .await
        .expect("plan");
    assert_eq!(planned.snapshot_id, Some(42));
    assert_eq!(planned.scan_tasks, [json!({"task":"opaque"})]);
    let request = task.join().expect("server");
    assert!(
        request.starts_with("POST /catalog/v1/warehouse/namespaces/acme/tables/customers/plan ")
    );
    let (_, body) = request.split_once("\r\n\r\n").expect("body");
    let body: Value = serde_json::from_str(body).expect("request JSON");
    assert_eq!(body["projection"], json!(["customer_id"]));
    assert_eq!(body["snapshot-id"], 42);
    assert_eq!(body["limit"], 10);
}

#[tokio::test]
async fn metadata_requests_use_the_configured_origin_and_credential() {
    let (origin, task) = fixture("200 OK", r#"{"metadata":{"current-snapshot-id":42}}"#);
    let catalog = LakeCatHttpCatalog::new(origin, principal().subject, r#"{"test":"credential"}"#)
        .expect("client");
    let metadata = catalog
        .load_metadata(&table(), &principal())
        .await
        .expect("metadata");
    assert_eq!(metadata["current-snapshot-id"], 42);
    let request = task.join().expect("server");
    assert!(request.starts_with("GET /catalog/v1/warehouse/namespaces/acme/tables/customers "));
    assert!(request.contains("x-lakecat-typedid-envelope: {\"test\":\"credential\"}"));
    assert!(request.contains("x-lakecat-agent-did: did:example:agent"));
}

#[tokio::test]
async fn denied_catalog_responses_do_not_echo_backend_secrets() {
    let (origin, task) = fixture("403 Forbidden", "SECRET");
    let catalog = LakeCatHttpCatalog::new(origin, principal().subject, "{}").expect("client");
    let error = catalog
        .load_metadata(&table(), &principal())
        .await
        .expect_err("denied");
    assert!(matches!(error, LakeCatError::Forbidden(_)));
    assert!(!error.to_string().contains("SECRET"));
    task.join().expect("server");
}

#[test]
fn credentials_cannot_be_sent_to_another_principal_or_non_origin_url() {
    for origin in [
        "https://user:password@example.com/",
        "https://example.com/path",
        "file:///tmp/catalog",
    ] {
        assert!(matches!(
            LakeCatHttpCatalog::new(Url::parse(origin).expect("URL"), principal().subject, "{}"),
            Err(LakeCatError::InvalidArgument(_))
        ));
    }
    let catalog = LakeCatHttpCatalog::new(
        Url::parse("http://127.0.0.1:1/").expect("URL"),
        principal().subject,
        "{}",
    )
    .expect("client");
    let other = Principal::new("did:example:other", PrincipalKind::Agent).expect("other");
    assert!(matches!(
        catalog.table_url(&table(), &other),
        Err(LakeCatError::Forbidden(_))
    ));
}

#[tokio::test]
async fn only_authoritative_missing_table_responses_allow_creation() {
    for (body, missing_table) in [
        (
            r#"{"error":{"type":"NoSuchTableException","code":404}}"#,
            true,
        ),
        (
            r#"{"error":{"type":"NoSuchNamespaceException","code":404}}"#,
            false,
        ),
        (
            r#"{"error":{"type":"NoSuchTableException","code":500}}"#,
            false,
        ),
        ("route not found", false),
    ] {
        let (origin, task) = fixture("404 Not Found", body);
        let catalog = LakeCatHttpCatalog::new(origin, principal().subject, "{}").expect("client");
        let error = catalog
            .load_metadata(&table(), &principal())
            .await
            .expect_err("missing response");
        assert_eq!(
            matches!(
                error,
                LakeCatError::NotFound {
                    object: "table",
                    ..
                }
            ),
            missing_table
        );
        task.join().expect("server");
    }
}
