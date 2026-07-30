//! Typed application orchestration for MCP harness run and retention actions.

use enforcer_harness::{
    execution::ExecuteRequest,
    retention::{self, PruneOutcome},
    storage::RunOutcome,
};

use crate::boundary::harness_run::{HarnessPruneRequest, HarnessRunRequest};

/// Typed application failures preserve their source through the router's
/// final JSON presentation boundary.
#[derive(Debug, thiserror::Error)]
pub enum HarnessRunApplicationError {
    #[error(transparent)]
    Config(#[from] enforcer_config::error::ConfigLoadError),
    #[error(transparent)]
    Harness(#[from] enforcer_core::error::Error),
}

/// Execute a decoded MCP command and persist its native harness record.
pub fn execute(request: HarnessRunRequest) -> Result<RunOutcome, HarnessRunApplicationError> {
    let config_path =
        std::path::Path::new(request.repo_root.as_str()).join("ocentra-enforcer.config.json");
    let config = enforcer_config::load_project_config(&config_path)?.harness;
    let HarnessRunRequest {
        repo_root,
        cwd,
        run_id,
        tool,
        language,
        command,
        crate_name,
        package_name,
        domain,
        tags,
    } = request;
    Ok(enforcer_harness::execution::execute(
        &ExecuteRequest {
            repo_root,
            cwd,
            run_id,
            tool,
            language,
            command,
            crate_name,
            package_name,
            domain,
            tags,
        },
        &config,
    )?)
}

/// Apply the frozen-MJS compatibility pruning contract through the typed
/// harness engine. Native retention remains authoritative-root-only.
pub fn prune_frozen_compat(
    request: &HarnessPruneRequest,
) -> Result<PruneOutcome, HarnessRunApplicationError> {
    let config_path =
        std::path::Path::new(request.repo_root.as_str()).join("ocentra-enforcer.config.json");
    let config = enforcer_config::load_project_config(&config_path)?.harness;
    Ok(retention::prune_runs_frozen_compat(
        std::path::Path::new(request.repo_root.as_str()),
        &config,
    )?)
}
