//! Native process execution for the frozen MCP `ocentra_enforcer_run` tool.

use std::path::Path;
use std::process::Command;

use enforcer_core::error::Result;
use enforcer_domain::config_types::{CrateName, HarnessConfig};
use enforcer_domain::harness_types::{
    HarnessCapturedOutput, HarnessCommandArgument, HarnessDomainName, HarnessLanguage,
    HarnessPackageName, HarnessPinned, HarnessRunId, HarnessTag, HarnessTimestamp, HarnessToolName,
};
use enforcer_domain::paths::RepoRoot;
use enforcer_domain::telemetry_types::ProcessExitCode;

use crate::storage::{record_run, RunInput, RunOutcome};

/// Fully decoded, boundary-owned request for one recorded native command.
#[derive(Debug, Clone)]
pub struct ExecuteRequest {
    pub repo_root: RepoRoot,
    pub cwd: Option<String>,
    pub run_id: HarnessRunId,
    pub tool: HarnessToolName,
    pub language: Option<HarnessLanguage>,
    pub command: Vec<HarnessCommandArgument>,
    pub crate_name: Option<CrateName>,
    pub package_name: Option<HarnessPackageName>,
    pub domain: Option<HarnessDomainName>,
    pub tags: Vec<HarnessTag>,
}

/// Execute without a shell, capture both streams, then persist the real outcome.
pub fn execute(request: &ExecuteRequest, config: &HarnessConfig) -> Result<RunOutcome> {
    let started_at = timestamp_now()?;
    let cwd = request
        .cwd
        .as_deref()
        .map(|relative| Path::new(request.repo_root.as_str()).join(relative))
        .unwrap_or_else(|| Path::new(request.repo_root.as_str()).to_path_buf());
    let executable = request.command.first().ok_or_else(|| {
        enforcer_core::error::Error::InvalidConfig("run command must not be empty".to_owned())
    })?;
    let output = Command::new(executable.as_str())
        .args(
            request
                .command
                .iter()
                .skip(1)
                .map(HarnessCommandArgument::as_str),
        )
        .current_dir(cwd)
        .output();
    let (stdout, stderr, exit_code) = match output {
        Ok(output) => (
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
            output.status.code().unwrap_or(1),
        ),
        Err(error) => (
            String::new(),
            format!("Harness child process failed ({}): {}", error.kind(), error),
            1,
        ),
    };
    let ended_at = timestamp_now()?;
    record_run(
        &RunInput {
            repo_root: &request.repo_root,
            run_id: request.run_id.clone(),
            tool: request.tool.clone(),
            language: request.language,
            command: request.command.clone(),
            stdout: HarnessCapturedOutput::from_owned(stdout),
            stderr: HarnessCapturedOutput::from_owned(stderr),
            exit_code: ProcessExitCode::new(exit_code),
            crate_name: request.crate_name.clone(),
            package_name: request
                .package_name
                .as_ref()
                .map(|value| CrateName::try_new(value.as_str().to_owned()))
                .transpose()
                .map_err(|error| enforcer_core::error::Error::InvalidConfig(error.to_string()))?,
            domain: request.domain.clone(),
            tags: request.tags.clone(),
            pinned: HarnessPinned::Unpinned,
            started_at,
            ended_at,
        },
        config,
    )
}

fn timestamp_now() -> Result<HarnessTimestamp> {
    let millis = enforcer_core::platform::epoch_millis()?;
    HarnessTimestamp::try_new(enforcer_core::platform::iso8601_utc(millis)).map_err(Into::into)
}
