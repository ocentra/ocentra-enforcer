//! BOUNDARY-INVARIANT: this adapter accepts one reviewed Trivy JSON shape,
//! converts it into the shared typed outcome, and never turns an unavailable,
//! malformed, or failed external engine into a native pass.
//!
//! Reference-only Trivy adapter for local IaC misconfiguration evidence.
//!
//! The product does not require Trivy: native Rust predicates remain the
//! product path. This module proves only that one pinned, optional external
//! engine can be bounded, parsed, and normalized without turning absence or
//! malformed output into a pass.

use std::path::{Component, Path};

use enforcer_core::error::Result;
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::harness_types::{
    HarnessCommandArgument, HarnessDiagnosticMessage, HarnessDiagnosticPath,
    HarnessExecutionTermination, HarnessExternalRuleId, HarnessExternalSeverity, HarnessRunId,
    HarnessSourceLine, HarnessToolName,
};
use enforcer_domain::paths::RepoRoot;
use serde::Deserialize;

use super::seam::{AdapterOutcome, EngineFindingEnvelope};
use crate::execution::{execute_unrecorded_bounded, ExecuteRequest};

/// Validates CP10's recorded Trivy-to-component mapping contract.
pub mod mapping;

const EXECUTABLE: &str = "trivy";
const ENGINE_VERSION: &str = "0.68.2";
const RUN_ID: &str = "cp07-trivy-live";

/// Parse one Trivy JSON report into the shared honest adapter outcome.
///
/// PROPERTY-TEST: `tests/cyberskills_trivy.rs::trivy_parser_is_total_and_deterministic`
/// exercises arbitrary text through this boundary twice and requires the same
/// acceptance result on both passes.
pub fn parse_report(raw: &str) -> Result<AdapterOutcome> {
    let report: TrivyReport = serde_json::from_str(raw)
        .map_err(|error| DecodeError::new("trivy.report", error.to_string()))?;
    let ran = u32::try_from(report.results.len())
        .unwrap_or(u32::MAX)
        .max(1);
    let findings = report
        .results
        .iter()
        .flat_map(|result| {
            result
                .misconfigurations
                .iter()
                .map(move |item| (result, item))
        })
        .map(|(result, item)| {
            let line = item
                .cause_metadata
                .as_ref()
                .map(|metadata| metadata.start_line)
                .filter(|line| *line > 0)
                .unwrap_or(1);
            let message = if item.message.trim().is_empty() {
                item.title.clone()
            } else {
                format!("{}: {}", item.title, item.message)
            };
            EngineFindingEnvelope {
                rule_id: HarnessExternalRuleId::from_adapter(&item.id),
                severity: HarnessExternalSeverity::from_adapter(&item.severity),
                file: HarnessDiagnosticPath::from_adapter(&result.target),
                line: HarnessSourceLine::from_external(line.into()),
                message: HarnessDiagnosticMessage::from_adapter(&message),
                threat_id: None,
            }
        })
        .collect();
    Ok(AdapterOutcome::Ran { ran, findings })
}

/// Run the reviewed local Trivy command through the shared bounded process seam.
pub fn run(repo_root: RepoRoot, target_directory: &str) -> Result<AdapterOutcome> {
    validate_target_directory(target_directory)?;
    let request = ExecuteRequest {
        repo_root,
        cwd: Some(target_directory.to_owned()),
        run_id: HarnessRunId::from_adapter(RUN_ID),
        tool: HarnessToolName::from_adapter(EXECUTABLE),
        language: None,
        command: reviewed_command()?,
        crate_name: None,
        package_name: None,
        domain: None,
        tags: vec![],
    };
    let execution = execute_unrecorded_bounded(&request)?;
    Ok(outcome_from_execution(&execution))
}

/// Return the exact command template used by the optional live pilot.
pub fn reviewed_command() -> std::result::Result<Vec<HarnessCommandArgument>, DecodeError> {
    let mut command = Vec::with_capacity(9);
    command.push(HarnessCommandArgument::try_new(executable_name())?);
    for value in [
        "config",
        "--format",
        "json",
        "--skip-check-update",
        "--exit-code",
        "0",
        "--quiet",
        ".",
    ] {
        command.push(HarnessCommandArgument::try_new(value.to_owned())?);
    }
    Ok(command)
}

/// Pinned pilot version recorded in the CP07 evidence packet.
#[must_use]
pub const fn pinned_version() -> &'static str {
    ENGINE_VERSION
}

fn executable_name() -> String {
    std::env::var("OCENTRA_TRIVY_EXECUTABLE")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| EXECUTABLE.to_owned())
}

fn validate_target_directory(target: &str) -> Result<()> {
    let path = Path::new(target);
    if target.trim().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(DecodeError::new(
            "trivy.targetDirectory",
            "target directory must be a non-empty repository-relative path",
        )
        .into());
    }
    Ok(())
}

fn outcome_from_execution(
    execution: &enforcer_domain::harness_types::HarnessBoundedExecution,
) -> AdapterOutcome {
    match execution.termination() {
        HarnessExecutionTermination::MissingExecutable => AdapterOutcome::Skipped { ran: 0 },
        HarnessExecutionTermination::Completed => match parse_report(execution.stdout().as_str()) {
            Ok(outcome) => outcome,
            Err(error) => AdapterOutcome::Errored {
                error_message: error.to_string(),
            },
        },
        termination => AdapterOutcome::Errored {
            error_message: format!(
                "trivy {}: {}",
                termination.as_str(),
                execution.stderr().as_str().trim()
            ),
        },
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TrivyReport {
    // DEFAULT-JUSTIFICATION: a report may omit results when the scan has no
    // target-level findings.
    #[serde(default)]
    results: Vec<TrivyResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TrivyResult {
    target: String,
    // DEFAULT-JUSTIFICATION: Trivy may omit a result's misconfiguration list
    // when the target produced no findings.
    #[serde(default)]
    misconfigurations: Vec<TrivyMisconfiguration>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TrivyMisconfiguration {
    #[serde(rename = "ID")]
    id: String,
    title: String,
    // DEFAULT-JUSTIFICATION: older Trivy output may omit an optional detail
    // message while still providing the title and rule identity.
    #[serde(default)]
    message: String,
    severity: String,
    cause_metadata: Option<TrivyCauseMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TrivyCauseMetadata {
    start_line: u32,
}

#[cfg(test)]
mod tests {
    use super::{
        outcome_from_execution, parse_report, pinned_version, reviewed_command, AdapterOutcome,
    };
    use enforcer_domain::harness_types::{
        HarnessBoundedExecution, HarnessCapturedOutput, HarnessExecutionTermination,
    };

    #[test]
    fn recorded_trivy_report_normalizes_to_shared_finding() -> Result<(), Box<dyn std::error::Error>>
    {
        let outcome = parse_report(include_str!(
            "../../../../tests/fixtures/cyberskills_adapters/trivy/good/recorded.json"
        ))?;
        let AdapterOutcome::Ran { ran, findings } = outcome else {
            return Err("expected a ran outcome".into());
        };
        assert_eq!(ran, 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id.as_str(), "AVD-AWS-0086");
        assert_eq!(findings[0].file.as_str(), "IAC-1.1.fail.tf");
        assert_eq!(
            findings[0].line.finding_line().map(|line| line.get()),
            Some(1)
        );
        Ok(())
    }

    #[test]
    fn reviewed_command_and_version_are_pinned() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(pinned_version(), "0.68.2");
        let command = reviewed_command()?;
        assert!(command[0].as_str().to_ascii_lowercase().contains("trivy"));
        assert!(command.windows(2).any(|pair| {
            pair[0].as_str() == "--skip-check-update" && pair[1].as_str() == "--exit-code"
        }));
        Ok(())
    }

    #[test]
    fn missing_tool_is_skip_not_pass() {
        let execution = HarnessBoundedExecution::from_parts(
            HarnessExecutionTermination::MissingExecutable,
            HarnessCapturedOutput::from_owned(String::new()),
            HarnessCapturedOutput::from_owned(String::new()),
            None,
            false,
        );
        assert_eq!(
            outcome_from_execution(&execution),
            AdapterOutcome::Skipped { ran: 0 }
        );
    }

    #[test]
    fn nonzero_timeout_and_overflow_are_errors() {
        for termination in [
            HarnessExecutionTermination::NonZeroExit,
            HarnessExecutionTermination::TimedOut,
            HarnessExecutionTermination::OutputLimitExceeded,
        ] {
            let execution = HarnessBoundedExecution::from_parts(
                termination,
                HarnessCapturedOutput::from_owned(String::new()),
                HarnessCapturedOutput::from_owned("failure".to_owned()),
                None,
                false,
            );
            assert!(matches!(
                outcome_from_execution(&execution),
                AdapterOutcome::Errored { .. }
            ));
        }
    }

    #[test]
    fn malformed_report_is_an_error() {
        let execution = HarnessBoundedExecution::from_parts(
            HarnessExecutionTermination::Completed,
            HarnessCapturedOutput::from_owned("not json".to_owned()),
            HarnessCapturedOutput::from_owned(String::new()),
            None,
            true,
        );
        assert!(matches!(
            outcome_from_execution(&execution),
            AdapterOutcome::Errored { .. }
        ));
    }
}
