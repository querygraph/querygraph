use super::*;
use pinax::ontology::{
    BindingReview, ConceptStatus, Ontology, OntologyError, store::OntologyStore,
};

#[tokio::test]
async fn central_discovery_verifies_scope_consults_store_and_rejects_changed_head() {
    let (service, catalog) = setup(Verdict::Allow, BackendMode::Available, false);
    let directory =
        std::env::temp_dir().join(format!("querygraph-ontology-{}", uuid::Uuid::new_v4()));
    let store = OntologyStore::new(directory.clone());
    let mut d = Ontology::bootstrap(&service.registry)
        .unwrap()
        .document()
        .clone();
    for c in &mut d.concepts {
        c.status = ConceptStatus::Approved {
            reviewer: "test-steward".into(),
            evidence: "review:1".into(),
        };
        c.aliases.push("account reference".into());
    }
    for b in &mut d.bindings {
        b.review = BindingReview::Approved {
            reviewer: "test-steward".into(),
            evidence: "review:1".into(),
        };
    }
    let ontology = Ontology::new(d, &service.registry).unwrap();
    let digest = ontology.digest().unwrap();
    store.publish(&ontology, None, &digest).unwrap();
    let source = super::super::ontology::CentralOntology::load(
        directory.clone(),
        digest.clone(),
        &service.registry,
    )
    .unwrap();
    let service = Arc::new(service.with_ontology(source));
    let intent = json!({"purpose":"analytics","query":"account reference","limit":10}).to_string();
    let payload = json!({"bodySha256":format!("{:x}",Sha256::digest(intent.as_bytes()))});
    let sign = |resource| {
        PyTypeDidEnvelope::signed(
            "registry-test-agent",
            "did:example:registry",
            "invoke",
            resource,
            payload.clone(),
        )
    };
    assert!(matches!(
        service
            .discover_ontology(&intent, &sign("/mcp/discover_pinax_tables"))
            .await,
        Err(RegistryServiceError::Authentication)
    ));
    let envelope = sign("/mcp/discover_pinax_ontology");
    let result = service.discover_ontology(&intent, &envelope).await.unwrap();
    assert_eq!(result["search"]["ontology_digest"], digest);
    assert!(!result["search"]["matches"].as_array().unwrap().is_empty());
    assert_eq!(catalog.loads.load(Ordering::Relaxed), 0);
    let mut server = crate::mcp::McpServer::new().with_registry_service(service.clone());
    let modern = json!({"jsonrpc":"2.0","id":"ontology-modern","method":"tools/call","params":{
        "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}},
        "name":"discover_pinax_ontology","arguments":{"intent":intent,"envelope":envelope}}});
    let response: Value =
        serde_json::from_str(&server.handle_line_async(&modern.to_string()).await.unwrap())
            .unwrap();
    assert_eq!(response["result"]["resultType"], "complete");
    assert_eq!(
        response["result"]["structuredContent"]["search"]["ontology_digest"],
        digest
    );
    assert_eq!(response["result"]["structuredContent"], result);
    let mut next = ontology.document().clone();
    next.revision += 1;
    next.previous_digest = Some(digest.clone());
    next.concepts[0].aliases.push("changed".into());
    let next = Ontology::new(next, &service.registry).unwrap();
    store
        .publish(&next, Some(&digest), &next.digest().unwrap())
        .unwrap();
    assert!(matches!(
        service.discover_ontology(&intent, &envelope).await,
        Err(RegistryServiceError::Ontology(OntologyError::Conflict))
    ));
    std::fs::remove_dir_all(directory).unwrap();
}
