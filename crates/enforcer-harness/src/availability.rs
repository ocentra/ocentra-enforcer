//! Typed availability and exact-version probing over the bounded runner.

use enforcer_core::error::Result;
use enforcer_domain::harness_types::{
    HarnessCapturedOutput, HarnessExecutionTermination, HarnessStepVersion,
    HarnessToolAvailability, HarnessToolSpec,
};

use crate::execution::{execute_allowlisted_bounded, ExecuteRequest};

/// Closed result of one reviewed availability probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessToolProbeResult {
    availability: HarnessToolAvailability,
    termination: Option<HarnessExecutionTermination>,
    observed_version: Option<HarnessStepVersion>,
}

impl HarnessToolProbeResult {
    fn misconfigured() -> Self {
        Self {
            availability: HarnessToolAvailability::Misconfigured,
            termination: None,
            observed_version: None,
        }
    }

    fn from_termination(termination: HarnessExecutionTermination) -> Self {
        let availability = match termination {
            HarnessExecutionTermination::MissingExecutable => HarnessToolAvailability::Missing,
            HarnessExecutionTermination::SpawnFailed | HarnessExecutionTermination::NonZeroExit => {
                HarnessToolAvailability::Failed
            }
            HarnessExecutionTermination::TimedOut => HarnessToolAvailability::TimedOut,
            HarnessExecutionTermination::OutputLimitExceeded => {
                HarnessToolAvailability::MalformedOutput
            }
            HarnessExecutionTermination::Completed => HarnessToolAvailability::MalformedOutput,
        };
        Self {
            availability,
            termination: Some(termination),
            observed_version: None,
        }
    }

    /// Typed availability state returned by the probe.
    #[must_use]
    pub const fn availability(&self) -> HarnessToolAvailability {
        self.availability
    }

    /// Underlying bounded-execution causality, absent only before execution.
    #[must_use]
    pub const fn termination(&self) -> Option<HarnessExecutionTermination> {
        self.termination
    }

    /// Valid normalized version observed on the reviewed output stream.
    #[must_use]
    pub const fn observed_version(&self) -> Option<&HarnessStepVersion> {
        self.observed_version.as_ref()
    }
}

/// Probe one reviewed tool using only its immutable probe command contract.
pub fn probe_allowlisted_tool(
    request: &ExecuteRequest,
    spec: &HarnessToolSpec,
) -> Result<HarnessToolProbeResult> {
    let probe = match spec.probe() {
        Some(probe) => probe,
        None => return Ok(HarnessToolProbeResult::misconfigured()),
    };
    let expected_version = match spec
        .expected_version()
        .and_then(|version| HarnessStepVersion::from_manifest(version.as_str()))
    {
        Some(version) => version,
        None => return Ok(HarnessToolProbeResult::misconfigured()),
    };
    let probe_spec = match spec.probe_execution_spec() {
        Ok(probe_spec) => probe_spec,
        Err(_) => return Ok(HarnessToolProbeResult::misconfigured()),
    };
    let mut probe_request = request.clone();
    probe_request.command = probe.command().to_vec();
    let execution = execute_allowlisted_bounded(&probe_request, &probe_spec)?;
    if execution.termination() != HarnessExecutionTermination::Completed {
        return Ok(HarnessToolProbeResult::from_termination(
            execution.termination(),
        ));
    }

    let selected_output = match probe.output() {
        enforcer_domain::harness_types::HarnessProbeOutput::Stdout => execution.stdout(),
        enforcer_domain::harness_types::HarnessProbeOutput::Stderr => execution.stderr(),
    };
    let observed_version = match parse_version(selected_output) {
        Some(version) => version,
        None => {
            return Ok(HarnessToolProbeResult {
                availability: HarnessToolAvailability::MalformedOutput,
                termination: Some(HarnessExecutionTermination::Completed),
                observed_version: None,
            })
        }
    };
    let availability = if observed_version.as_str() == expected_version.as_str() {
        HarnessToolAvailability::Available
    } else {
        HarnessToolAvailability::VersionMismatch
    };
    Ok(HarnessToolProbeResult {
        availability,
        termination: Some(HarnessExecutionTermination::Completed),
        observed_version: Some(observed_version),
    })
}

fn parse_version(output: &HarnessCapturedOutput) -> Option<HarnessStepVersion> {
    let normalized = output.as_str().replace("\r\n", "\n");
    if normalized
        .chars()
        .any(|character| character.is_control() && character != '\n')
    {
        return None;
    }
    let trimmed = normalized.trim();
    if trimmed.is_empty()
        || trimmed.contains('\n')
        || trimmed.contains('\r')
        || trimmed.chars().any(char::is_control)
    {
        return None;
    }
    HarnessStepVersion::try_new(trimmed.to_owned()).ok()
}
