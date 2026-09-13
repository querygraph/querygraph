//! Loopback-only console for the isolated, public-fixture EC2 demonstration.
use anyhow::{Context, Result, bail};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use clap::Parser;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{net::Ipv4Addr, path::PathBuf, process::Stdio, sync::Arc, time::Duration};
use tokio::{io::AsyncReadExt, process::Command, sync::Semaphore};

#[derive(Parser)]
struct Arguments {
    #[arg(long)]
    demo_root: PathBuf,
    #[arg(long, default_value_t = 18081)]
    port: u16,
}

struct Demo {
    root: PathBuf,
    port: u16,
    capacity: Semaphore,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Operation {
    Lakehouse,
    Standards,
    Mcp,
    Ontology,
    Discover,
    Plan,
    Execute,
    DenyColumn,
    DenyPurpose,
    Semantic,
    Navigator,
    Qglake,
}

impl Operation {
    fn client_argument(self) -> Option<&'static str> {
        match self {
            Self::Ontology => Some("ontology"),
            Self::Discover => Some("discover"),
            Self::Plan => Some("plan"),
            Self::Execute => Some("execute"),
            Self::DenyColumn => Some("deny-column"),
            Self::DenyPurpose => Some("deny-purpose"),
            Self::Lakehouse
            | Self::Standards
            | Self::Mcp
            | Self::Semantic
            | Self::Navigator
            | Self::Qglake => None,
        }
    }

    fn expects_denial(self) -> bool {
        matches!(self, Self::DenyColumn | Self::DenyPurpose)
    }
}

fn router(demo: Arc<Demo>) -> Router {
    Router::new()
        .route(
            "/",
            get(|| async { Html(include_str!("../demo/pinax/console.html")) }),
        )
        .route(
            "/slides",
            get(|| async { Html(include_str!("../demo/pinax/slides.html")) }),
        )
        .route(
            "/assets/pinax-headboard.png",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "image/png")],
                    include_bytes!("../demo/pinax/assets/pinax-headboard.png").as_slice(),
                )
            }),
        )
        .route(
            "/assets/pinax-cover.png",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "image/png")],
                    include_bytes!("../demo/pinax/assets/pinax-cover.png").as_slice(),
                )
            }),
        )
        .route("/api/run/{operation}", post(run))
        .with_state(demo)
}

async fn run(
    State(demo): State<Arc<Demo>>,
    Path(operation): Path<Operation>,
    headers: HeaderMap,
) -> Response {
    // A custom header prevents cross-origin form submissions; exact Host checks
    // also reject DNS rebinding. No CORS middleware is installed.
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok());
    let local = format!("localhost:{}", demo.port);
    let loopback = format!("127.0.0.1:{}", demo.port);
    if !matches!(host, Some(value) if value == local || value == loopback)
        || headers
            .get("x-querygraph-demo")
            .and_then(|value| value.to_str().ok())
            != Some("1")
    {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error":"Use the local demo console"})),
        )
            .into_response();
    }
    let Ok(_permit) = demo.capacity.try_acquire() else {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({"error":"A demo operation is already running"})),
        )
            .into_response();
    };
    match tokio::time::timeout(Duration::from_secs(120), execute(&demo, operation)).await {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(error)) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({"error":error.to_string()})),
        )
            .into_response(),
        Err(_) => (
            StatusCode::GATEWAY_TIMEOUT,
            Json(json!({"error":"Demo operation exceeded 120 seconds"})),
        )
            .into_response(),
    }
}

async fn bounded_output(reader: impl tokio::io::AsyncRead + Unpin) -> Result<Vec<u8>> {
    const MAX_OUTPUT: u64 = 2 * 1024 * 1024;
    let mut output = Vec::new();
    reader.take(MAX_OUTPUT + 1).read_to_end(&mut output).await?;
    if output.len() as u64 > MAX_OUTPUT {
        bail!("Demo output exceeded 2 MiB");
    }
    Ok(output)
}

async fn execute(demo: &Demo, operation: Operation) -> Result<Value> {
    // The operator console shows only the public synthetic fixture's schema.
    // A fresh signed plan verifies the catalog/registry agreement; seed metadata
    // is labelled as such and is never substituted for live agent discovery.
    if matches!(operation, Operation::Lakehouse) {
        let plan = execute_operation(demo, Operation::Plan).await?;
        let path = demo.root.join("run/pinax/seed.json");
        let seed = tokio::task::spawn_blocking(move || -> Result<Value> {
            use std::io::Read;
            let mut bytes = Vec::new();
            std::fs::File::open(path)?
                .take(2 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > 2 * 1024 * 1024 {
                bail!("Fixture metadata exceeded 2 MiB");
            }
            Ok(serde_json::from_slice(&bytes)?)
        })
        .await??;
        let schemas = seed["metadata"]["schemas"]
            .as_array()
            .context("Fixture schemas")?;
        let schema_id = &seed["metadata"]["current-schema-id"];
        let schema = schemas
            .iter()
            .find(|schema| &schema["schema-id"] == schema_id)
            .context("Current fixture schema")?;
        return Ok(json!({"status":"passed", "result":{
            "schema_source":"retained synthetic Iceberg seed; live catalog agreement checked by the signed plan",
            "schema":schema, "catalog_plan":plan["result"],
            "engine":"Sail", "catalog":"LakeCat", "registry":"Pinax"
        }}));
    }
    let consultation = if matches!(
        operation,
        Operation::Navigator | Operation::Qglake | Operation::Semantic
    ) {
        Some(execute_operation(demo, Operation::Ontology).await?)
    } else {
        None
    };
    let mut result = execute_operation(demo, operation).await?;
    if let Some(consultation) = consultation {
        result["ontology_consultation"] = consultation["result"].clone();
    }
    Ok(result)
}

async fn execute_operation(demo: &Demo, operation: Operation) -> Result<Value> {
    let target = demo.root.join("src/querygraph/target/debug");
    let mut command = if let Some(argument) = operation.client_argument() {
        let mut command = Command::new(target.join("examples/pinax-client"));
        command
            .arg("--config")
            .arg(demo.root.join("run/pinax/registry-service.json"))
            .arg(argument);
        command
    } else if matches!(operation, Operation::Standards) {
        let mut command = Command::new(target.join("querygraph"));
        command.args([
            "pinax",
            "init",
            "--enterprise",
            "acme",
            "--owner",
            "platform",
            "--steward",
            "data",
            "--policy",
            "enterprise",
        ]);
        command
    } else if matches!(operation, Operation::Mcp) {
        let mut command = Command::new(target.join("examples/pinax-mcp-client"));
        command.args(["--url", "http://127.0.0.1:18082/mcp"]);
        command
    } else if matches!(operation, Operation::Navigator) {
        let mut command = Command::new(target.join("querygraph"));
        command.arg("navigator")
            .arg("--dataset-name").arg("Hazard vocabulary")
            .arg("--description").arg("Controlled vocabulary with multilingual technical terms, access control, governance, and public safety labels")
            .arg("--landing-page").arg("https://querygraph.ai/datasets/hazards")
            .arg("--data-url").arg("https://querygraph.ai/datasets/hazards.csv");
        command
    } else if matches!(operation, Operation::Qglake) {
        let mut command = Command::new(target.join("querygraph"));
        command.arg("qglake-story").arg("--json");
        command
    } else {
        let report = demo
            .root
            .join("reports")
            .join(format!("console-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&report)?;
        let mut command = Command::new(target.join("querygraph"));
        command
            .env("QG_SAIL_WAREHOUSE", report.join("warehouse"))
            .arg("dataverse-e2e")
            .arg("--live-sail")
            .arg("--sail-endpoint")
            .arg("http://127.0.0.1:15051")
            .arg("--sail-dir")
            .arg(report.join("sail"))
            .arg("--openlineage-file")
            .arg(report.join("events.jsonl"))
            .arg("--did-ledger-file")
            .arg(report.join("attestations.jsonl"));
        command
    };
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("Starting demo executable")?;
    let stdout = child.stdout.take().context("Missing stdout pipe")?;
    let stderr = child.stderr.take().context("Missing stderr pipe")?;
    let (status, stdout, stderr) = tokio::try_join!(
        async { child.wait().await.map_err(anyhow::Error::from) },
        bounded_output(stdout),
        bounded_output(stderr)
    )?;
    if operation.expects_denial() {
        if status.success()
            || !stdout.is_empty()
            || !String::from_utf8_lossy(&stderr).contains("Pinax scan denied")
        {
            bail!("Denial check failed: expected a rejected request with no result output");
        }
        return Ok(
            json!({"status":"denied", "rows_released":false, "detail":String::from_utf8_lossy(&stderr)}),
        );
    }
    if !status.success() {
        bail!(
            "Demo operation failed: {}",
            String::from_utf8_lossy(&stderr)
        );
    }
    Ok(json!({"status":"passed", "result":serde_json::from_slice::<Value>(&stdout)?}))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Arguments::parse();
    let root = args.demo_root.canonicalize()?;
    let demo = Arc::new(Demo {
        root,
        port: args.port,
        capacity: Semaphore::new(1),
    });
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, args.port)).await?;
    eprintln!("QueryGraph demo: http://127.0.0.1:{}", args.port);
    axum::serve(listener, router(demo)).await?;
    Ok(())
}

#[cfg(test)]
#[path = "pinax_demo/tests.rs"]
mod tests;
