//! Review the fixed synthetic demo vocabulary and publish it centrally.
//! This is deliberately restricted to the known fixture, not an auto-approver.
use anyhow::{Result, ensure};
use clap::Parser;
use pinax::{
    Registry,
    ontology::{BindingReview, ConceptStatus, Ontology, store::OntologyStore},
};
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Parser)]
struct Arguments {
    #[arg(long)]
    registry: PathBuf,
    #[arg(long)]
    store: PathBuf,
    #[arg(long)]
    config: PathBuf,
}
fn bounded(path: &std::path::Path, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= limit, "file exceeds bound");
    Ok(bytes)
}
fn main() -> Result<()> {
    let args = Arguments::parse();
    let registry = Registry::from_json(&bounded(&args.registry, pinax::MAX_DOCUMENT_BYTES)?)?;
    ensure!(
        registry.enterprise() == "acme" && registry.tables().len() == 1,
        "Only the synthetic demo registry is supported"
    );
    let table = registry.table("rows")?;
    ensure!(
        table
            .columns
            .iter()
            .map(|c| (c.id, c.name.as_str(), c.semantic.as_str()))
            .collect::<Vec<_>>()
            == vec![
                (1, "id", "row.id"),
                (2, "tenant", "row.tenant"),
                (3, "private_email", "row.private_email")
            ],
        "Fixture schema changed; review vocabulary again"
    );
    let mut document = Ontology::bootstrap(&registry)?.document().clone();
    for concept in &mut document.concepts {
        let (label, definition, aliases) = match concept.label.as_str() {
            "rows" => (
                "Customer records",
                "Synthetic tenant-scoped customer records for the governed execution demonstration.",
                vec!["customer table"],
            ),
            "row.id" => (
                "Customer identifier",
                "Stable identifier of a synthetic customer record; not a tenant identifier.",
                vec!["account number", "customer id"],
            ),
            "row.tenant" => (
                "Tenant identifier",
                "Enterprise tenant that owns this synthetic record; used by the owner's mandatory predicate.",
                vec!["tenant id"],
            ),
            "row.private_email" => (
                "Private email address",
                "Private contact address excluded from the demo agent's permitted projection.",
                vec!["email", "contact address"],
            ),
            _ => anyhow::bail!("Unreviewed concept"),
        };
        concept.label = label.into();
        concept.definition = definition.into();
        concept.aliases = aliases.into_iter().map(str::to_owned).collect();
        concept.status = ConceptStatus::Approved {
            reviewer: "querygraph-demo-steward".into(),
            evidence: "demo/pinax/ontology-review.md".into(),
        };
    }
    for binding in &mut document.bindings {
        binding.review = BindingReview::Approved {
            reviewer: "querygraph-demo-steward".into(),
            evidence: "demo/pinax/ontology-review.md".into(),
        };
    }
    let ontology = Ontology::new(document, &registry)?;
    let digest = ontology.digest()?;
    let store = OntologyStore::new(args.store.clone());
    match store.head()? {
        None => {
            store.publish(&ontology, None, &digest)?;
        }
        Some(head) => ensure!(
            head == digest,
            "An ontology is already published; use the reviewed revision workflow"
        ),
    }
    let mut config: serde_json::Value = serde_json::from_slice(&bounded(&args.config, 64 * 1024)?)?;
    let store_path = args.store.canonicalize()?;
    config["ontology"] = serde_json::json!({"store":store_path,"expected_digest":digest});
    let pending = args.config.with_extension("ontology-pending");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)?;
    file.write_all(&serde_json::to_vec_pretty(&config)?)?;
    file.sync_all()?;
    std::fs::rename(pending, &args.config)?;
    println!(
        "{}",
        serde_json::json!({"ontology_digest":digest,"tables":1,"fields":3,"concepts":4,"store":store_path})
    );
    Ok(())
}
