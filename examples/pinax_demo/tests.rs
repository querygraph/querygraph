use super::*;
use axum::{body::Body, http::Request};
use tower::ServiceExt;

#[tokio::test]
async fn rejects_cross_origin_and_unknown_operations_before_spawning() {
    let demo = Arc::new(Demo {
        root: PathBuf::from("/nonexistent-demo"),
        port: 18081,
        capacity: Semaphore::new(1),
    });
    for (host, marker) in [
        ("attacker.example:18081", Some("1")),
        ("localhost:18081", None),
    ] {
        let mut request = Request::post("/api/run/execute").header(header::HOST, host);
        if let Some(marker) = marker {
            request = request.header("x-querygraph-demo", marker);
        }
        let response = router(demo.clone())
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    let response = router(demo)
        .oneshot(
            Request::post("/api/run/arbitrary-command")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn rejects_busy_console_without_spawning() {
    let demo = Arc::new(Demo {
        root: PathBuf::from("/nonexistent-demo"),
        port: 18081,
        capacity: Semaphore::new(0),
    });
    let response = router(demo)
        .oneshot(
            Request::post("/api/run/execute")
                .header(header::HOST, "localhost:18081")
                .header("x-querygraph-demo", "1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn refuses_output_larger_than_the_console_budget() {
    let bytes = vec![0; 2 * 1024 * 1024 + 1];
    assert!(bounded_output(bytes.as_slice()).await.is_err());
}

#[tokio::test]
async fn format_selection_is_per_request_and_rejects_unknown_formats() {
    let demo = Arc::new(Demo {
        root: PathBuf::from("/nonexistent-demo"),
        port: 18081,
        capacity: Semaphore::new(1),
    });
    for (path, label, other) in [
        ("/?format=delta", "Delta Lake", "real Iceberg"),
        ("/?format=iceberg", "Iceberg", "real Delta Lake"),
        (
            "/slides?format=delta&part=all",
            "Delta Lake",
            "Real Iceberg",
        ),
    ] {
        let response = router(demo.clone())
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let text = std::str::from_utf8(&body).unwrap();
        assert!(text.contains(label));
        assert!(!text.contains(other));
        assert!(!text.contains("{{format"));
    }
    let response = router(demo)
        .oneshot(
            Request::post("/api/run/execute?format=unknown")
                .header(header::HOST, "localhost:18081")
                .header("x-querygraph-demo", "1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
