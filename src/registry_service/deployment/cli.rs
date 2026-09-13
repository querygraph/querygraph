//! Operator-facing review, conditional deployment, and reconciliation commands.

use anyhow::{Result, bail};
use clap::Subcommand;
use pinax::adapters::CatalogBinding;
use std::path::PathBuf;

use super::{RegistryDeploymentPlan, RegistryDeploymentStatus};
use crate::registry_service::config::{
    apply_registry_deployment, read_registry, reconcile_registry_deployment,
};

/// Review, apply, or reconcile a transition using authoritative Pinax contracts.
#[derive(Debug, Subcommand)]
pub enum RegistryDeploymentCommands {
    /// Emit every target binding and its predecessor or create request; no catalog I/O.
    Plan {
        /// Reviewed predecessor registry JSON file.
        #[arg(long)]
        current: PathBuf,
        /// Reviewed successor registry JSON file.
        #[arg(long)]
        target: PathBuf,
        /// Deployment-owned LakeCat warehouse.
        #[arg(long)]
        warehouse: String,
    },
    /// Read all bound tables; exit nonzero on pending deployment, drift, or catalog failure.
    Reconcile {
        /// Host configuration containing the predecessor registry and LakeCat credential.
        #[arg(long)]
        registry_config: PathBuf,
        /// Reviewed target registry JSON file.
        #[arg(long)]
        target: PathBuf,
    },
    /// Apply a reviewed target once per pending table using owner-enforced state checks.
    Apply {
        /// Host configuration containing the predecessor registry and operator credential.
        #[arg(long)]
        registry_config: PathBuf,
        /// Reviewed successor registry JSON file.
        #[arg(long)]
        target: PathBuf,
        /// Exact target_digest from the reviewed registry-deployment plan.
        #[arg(long)]
        expected_target_digest: String,
    },
}

impl RegistryDeploymentCommands {
    /// Run outside an async runtime, writing a JSON review/report to stdout.
    ///
    /// # Errors
    /// Rejects invalid files/configuration or incompatible transitions. A complete
    /// reconciliation report is still printed when pending/drift causes failure;
    /// catalog errors produce no misleading partial readiness report.
    pub fn run(self) -> Result<()> {
        match self {
            Self::Plan {
                current,
                target,
                warehouse,
            } => {
                let previous = read_registry(&current)?;
                let target = read_registry(&target)?;
                let plan = RegistryDeploymentPlan::new(
                    &previous,
                    &target,
                    &CatalogBinding::new(&warehouse)?,
                )?;
                println!("{}", serde_json::to_string_pretty(&plan)?);
            }
            Self::Reconcile {
                registry_config,
                target,
            } => {
                let report = reconcile_registry_deployment(&registry_config, &target)?;
                println!("{}", serde_json::to_string_pretty(&report)?);
                if report.status() != RegistryDeploymentStatus::Ready {
                    bail!("registry deployment is not ready");
                }
            }
            Self::Apply {
                registry_config,
                target,
                expected_target_digest,
            } => {
                let report =
                    apply_registry_deployment(&registry_config, &target, &expected_target_digest)?;
                println!("{}", serde_json::to_string_pretty(&report)?);
                if report.status() != RegistryDeploymentStatus::Ready {
                    bail!("registry deployment is not ready; reconcile before consumer cutover");
                }
            }
        }
        Ok(())
    }
}
