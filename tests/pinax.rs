use std::process::{Command, Output};

use querygraph::pinax::Registry;
use serde_json::Value;

fn command(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_querygraph"))
        .arg("pinax")
        .args(args)
        .output()
        .expect("Pinax compatibility command starts")
}

fn json(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("command emits JSON")
}

#[test]
fn compatibility_commands_share_registry_types_and_catalog_contracts() {
    let generated = json(command(&[
        "init",
        "--enterprise",
        "acme",
        "--owner",
        "platform",
        "--steward",
        "data",
        "--policy",
        "enterprise",
    ]));
    // This assignment proves that the compatibility path exposes the crate's
    // actual validated type, without a copied DTO or conversion layer.
    let registry: pinax::Registry =
        Registry::from_json(&serde_json::to_vec(&generated).expect("JSON encodes"))
            .expect("generated standard contracts validate");
    assert_eq!(registry.tables().len(), 3);
    let digest = registry.digest().expect("validated registry has a digest");
    let dir = tempfile::tempdir().expect("temporary authoring directory");
    let path = dir.path().join("registry.json");
    std::fs::write(&path, generated.to_string()).expect("write authoring input");
    let path = path.to_str().expect("temporary path is UTF-8");

    let validated = json(command(&["validate", "--registry", path]));
    assert_eq!(validated["digest"], digest);
    assert_eq!(validated["tables"], 3);
    let checked = json(command(&["check", "--current", path, "--next", path]));
    assert_eq!(checked["compatible"], true);
    assert_eq!(checked["digest"], digest);

    let plans = json(command(&[
        "catalog-plan",
        "--registry",
        path,
        "--warehouse",
        "production",
    ]));
    let plans = plans.as_array().expect("catalog plans are an array");
    assert_eq!(plans.len(), 3);
    for plan in plans {
        assert_eq!(
            plan["path"],
            "/catalog/v1/production/namespaces/acme/tables"
        );
        assert_eq!(plan["body"]["properties"]["pinax.registry-digest"], digest);
    }
    assert_eq!(
        std::fs::read_to_string(path).expect("authoring input remains readable"),
        generated.to_string()
    );
}

#[test]
fn compatibility_commands_reject_invalid_and_incompatible_documents() {
    let dir = tempfile::tempdir().expect("temporary authoring directory");
    let previous = dir.path().join("previous.json");
    let next = dir.path().join("next.json");
    std::fs::write(
        &previous,
        r#"{"version":"pinax.v1","enterprise":"acme","tables":[]}"#,
    )
    .expect("write previous snapshot");
    std::fs::write(
        &next,
        r#"{"version":"pinax.v1","enterprise":"other","tables":[]}"#,
    )
    .expect("write incompatible snapshot");
    let previous = previous.to_str().expect("temporary path is UTF-8");
    let next = next.to_str().expect("temporary path is UTF-8");
    let rejected = command(&["check", "--current", previous, "--next", next]);
    assert!(!rejected.status.success());
    assert!(rejected.stdout.is_empty());

    std::fs::write(next, "{}").expect("write malformed document");
    let rejected = command(&["validate", "--registry", next]);
    assert!(!rejected.status.success());
    assert!(rejected.stdout.is_empty());
}

#[test]
fn deployment_command_reviews_all_registry_pins_and_rejects_incompatible_targets() {
    let previous = json(command(&[
        "init",
        "--enterprise",
        "acme",
        "--owner",
        "platform",
        "--steward",
        "data",
        "--policy",
        "enterprise",
    ]));
    let mut target = previous.clone();
    target["tables"][0]["revision"] = 2.into();
    let classification = target["tables"][0]["security"]["classification"].clone();
    let columns = target["tables"][0]["columns"]
        .as_array_mut()
        .expect("columns");
    let next_id = columns
        .iter()
        .map(|column| column["id"].as_u64().expect("field ID"))
        .max()
        .expect("existing fields")
        + 1;
    columns.push(serde_json::json!({
        "id": next_id, "name": "integration_note", "data_type": {"kind": "string"},
        "nullability": "optional", "semantic": "customer.integration_note",
        "description": "Optional integration note", "classification": classification,
    }));
    let dir = tempfile::tempdir().expect("temporary authoring directory");
    let current_path = dir.path().join("current.json");
    let target_path = dir.path().join("target.json");
    std::fs::write(&current_path, previous.to_string()).expect("current registry");
    std::fs::write(&target_path, target.to_string()).expect("target registry");
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_querygraph"))
            .args(["registry-deployment", "plan", "--current"])
            .arg(&current_path)
            .arg("--target")
            .arg(&target_path)
            .args(["--warehouse", "production"])
            .output()
            .expect("deployment command starts")
    };
    let plan = json(run());
    assert_ne!(plan["previous_digest"], plan["target_digest"]);
    let tables = plan["tables"].as_array().expect("table transitions");
    assert_eq!(tables.len(), 3);
    for table in tables {
        assert_eq!(
            table["target"]["properties"]["pinax.registry-digest"],
            plan["target_digest"]
        );
        assert_eq!(table["transition"]["operation"], "update");
    }
    target["enterprise"] = "other".into();
    std::fs::write(&target_path, target.to_string()).expect("incompatible target");
    let rejected = run();
    assert!(!rejected.status.success());
    assert!(rejected.stdout.is_empty());
}
