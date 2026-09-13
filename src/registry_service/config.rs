//! Filesystem bootstrap for a deployment-owned registry service.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};

use super::{RegistryPolicy, RegistryService, http::LakeCatHttpCatalog};
use pinax::{Registry, adapters::CatalogBinding};

#[cfg(test)]
mod tests;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ServiceConfig {
    ontology: Option<OntologyConfig>,
    registry: PathBuf,
    warehouse: String,
    policy: PathBuf,
    policy_id: String,
    policy_revision: u32,
    server_did: String,
    lakecat_origin: String,
    lakecat_subject: String,
    lakecat_envelope: Option<PathBuf>,
    lakecat_identity: Option<PathBuf>,
    activation: Option<ReviewedActivation>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OntologyConfig {
    store: PathBuf,
    expected_digest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewedActivation {
    expected_registry_digest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SigningIdentityConfig {
    seed: PathBuf,
    recipient_document: PathBuf,
}

/// Load a host-selected JSON configuration; relative paths resolve beside it.
///
/// The registry and policy are immutable for the service lifetime. Restart with
/// a reviewed configuration after a coordinated registry/policy deployment.
/// Credentials are read from a separate file, never from request arguments.
/// An optional reviewed activation digest also requires all catalog tables to
/// match before startup, within a single 30-second deadline. This observation
/// is not an exclusion lock; operators still own the maintenance window.
/// This blocking bootstrap must run before entering an async worker loop.
///
/// # Errors
/// Rejects malformed configuration, oversized files, invalid registry/policy,
/// unusable credentials, and catalog client configuration errors.
pub fn load_registry_service(path: &Path) -> Result<RegistryService> {
    let config: ServiceConfig = serde_json::from_slice(&read_bounded(path, 64 * 1024)?)
        .context("invalid registry service configuration JSON")?;
    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    let registry = Registry::from_json(&read_bounded(
        &directory.join(&config.registry),
        8 * 1024 * 1024,
    )?)?;
    let policy_bytes = read_bounded(&directory.join(&config.policy), 1024 * 1024)?;
    let policy_text = std::str::from_utf8(&policy_bytes).context("policy must be UTF-8")?;
    let engine = typesec_rbac::RbacEngine::from_yaml(policy_text)
        .map_err(|_| anyhow::anyhow!("invalid TypeSec RBAC policy"))?;
    let catalog = Arc::new(load_catalog(&config, directory)?);
    let policy = RegistryPolicy::new(config.policy_id, config.policy_revision, Arc::new(engine))?;
    let service = RegistryService::new(
        registry,
        CatalogBinding::new(&config.warehouse)?,
        policy,
        catalog.clone(),
        config.server_did,
    )?;
    if let Some(activation) = config.activation {
        if service.registry.digest()? != activation.expected_registry_digest {
            bail!("consumer registry differs from the reviewed activation digest");
        }
        let plan = super::deployment::RegistryDeploymentPlan::new(
            &service.registry,
            &service.registry,
            &service.binding,
        )?;
        let principal = lakecat_core::Principal::new(
            config.lakecat_subject,
            lakecat_core::PrincipalKind::Agent,
        )?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let report = runtime
            .block_on(async {
                tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    plan.reconcile(catalog.as_ref(), &principal),
                )
                .await
            })
            .context("consumer activation catalog check timed out")??;
        if report.status() != super::deployment::RegistryDeploymentStatus::Ready {
            bail!(
                "consumer activation requires every catalog table to match the reviewed registry"
            );
        }
    }
    let service = if let Some(ontology) = config.ontology {
        let source = super::ontology::CentralOntology::load(
            directory.join(ontology.store),
            ontology.expected_digest,
            &service.registry,
        )?;
        service.with_ontology(source)
    } else {
        service
    };
    Ok(service)
}

fn load_catalog(config: &ServiceConfig, directory: &Path) -> Result<LakeCatHttpCatalog> {
    let origin = config
        .lakecat_origin
        .parse()
        .context("invalid LakeCat origin")?;
    match (&config.lakecat_identity, &config.lakecat_envelope) {
        (Some(identity), None) => {
            let path = directory.join(identity);
            let identity: SigningIdentityConfig =
                serde_json::from_slice(&read_bounded(&path, 64 * 1024)?)
                    .context("invalid LakeCat identity configuration")?;
            let parent = path.parent().unwrap_or(directory);
            let seed = zeroize::Zeroizing::new(read_bounded(&parent.join(identity.seed), 32)?);
            let recipient: typesec_integrations::DidDocument = serde_json::from_slice(
                &read_bounded(&parent.join(identity.recipient_document), 64 * 1024)?,
            )
            .context("invalid LakeCat recipient DID document")?;
            let credentials = super::http::credentials::TypeSecLakeCatCredentials::from_seed(
                &seed,
                &config.lakecat_subject,
                recipient,
            )?;
            Ok(LakeCatHttpCatalog::with_credentials(
                origin,
                config.lakecat_subject.clone(),
                Arc::new(credentials),
            )?)
        }
        (None, Some(envelope)) => {
            let credential: serde_json::Value =
                serde_json::from_slice(&read_bounded(&directory.join(envelope), 32 * 1024)?)
                    .context("invalid LakeCat credential JSON")?;
            Ok(LakeCatHttpCatalog::new(
                origin,
                config.lakecat_subject.clone(),
                &credential.to_string(),
            )?)
        }
        _ => bail!("configure exactly one LakeCat signing identity or single-use envelope"),
    }
}

/// Perform a blocking, read-only reconciliation using host catalog credentials.
/// Relative registry/credential paths resolve beside the configuration file.
/// This bootstrap/CLI entry point must run outside an async runtime.
///
/// # Errors
/// Rejects malformed or incompatible registries/configuration, invalid credentials,
/// runtime initialization failures, and catalog denial/unavailability.
pub fn reconcile_registry_deployment(
    config_path: &Path,
    target_path: &Path,
) -> Result<super::deployment::RegistryReconciliation> {
    let (plan, catalog, principal) = deployment_inputs(config_path, target_path)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    Ok(runtime.block_on(plan.reconcile(&catalog, &principal))?)
}

/// Apply an operator-reviewed registry transition through conditional catalog writes.
///
/// # Errors
/// Rejects a target digest differing from the review, malformed configuration,
/// drift, unsupported conditional updates, and writes requiring reconciliation.
/// Must be called outside an async runtime; it never changes service configuration.
pub fn apply_registry_deployment(
    config_path: &Path,
    target_path: &Path,
    expected_target_digest: &str,
) -> Result<super::deployment::RegistryReconciliation> {
    let (plan, catalog, principal) = deployment_inputs(config_path, target_path)?;
    if plan.target_digest() != expected_target_digest {
        bail!("registry target digest differs from the reviewed plan");
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    Ok(runtime.block_on(plan.apply(&catalog, &principal))?)
}

fn deployment_inputs(
    config_path: &Path,
    target_path: &Path,
) -> Result<(
    super::deployment::RegistryDeploymentPlan,
    LakeCatHttpCatalog,
    lakecat_core::Principal,
)> {
    let config: ServiceConfig = serde_json::from_slice(&read_bounded(config_path, 64 * 1024)?)
        .context("invalid registry service configuration JSON")?;
    let directory = config_path.parent().unwrap_or_else(|| Path::new("."));
    let previous = read_registry(&directory.join(&config.registry))?;
    let target = read_registry(target_path)?;
    let plan = super::deployment::RegistryDeploymentPlan::new(
        &previous,
        &target,
        &CatalogBinding::new(&config.warehouse)?,
    )?;
    let catalog = load_catalog(&config, directory)?;
    let principal =
        lakecat_core::Principal::new(config.lakecat_subject, lakecat_core::PrincipalKind::Agent)?;
    Ok((plan, catalog, principal))
}

pub(super) fn read_registry(path: &Path) -> Result<Registry> {
    Ok(Registry::from_json(&read_bounded(
        path,
        pinax::MAX_DOCUMENT_BYTES,
    )?)?)
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)
        .with_context(|| format!("cannot open {}", path.display()))?
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        bail!("configuration input exceeds size limit: {}", path.display());
    }
    Ok(bytes)
}
