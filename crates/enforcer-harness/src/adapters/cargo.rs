//! BOUNDARY-INVARIANT: this adapter validates one closed Cargo command contract
//! before delegating all probing, execution, and parsing to shared harness seams.
//! Bounded in-memory Cargo pilot adapter.
//!
//! This module proves one real Cargo path without persisting a run. It owns
//! only the adapter composition and evidence envelope; process execution,
//! availability probing, and diagnostic parsing remain delegated to the
//! shared harness seams.

use std::fs;
use std::path::{Component, Path, PathBuf};

use enforcer_core::error::{Error, Result};
use enforcer_domain::boundary::hash::validate;
use enforcer_domain::harness_types::{
    HarnessExecutionTermination, HarnessProbeOutput, HarnessRunId, HarnessRunStatus,
    HarnessStepVersion, HarnessToolAvailability, HarnessToolSpec,
};
use enforcer_domain::hashes::Sha256;
use enforcer_domain::telemetry_types::ProcessExitCode;

use crate::availability::probe_allowlisted_tool;
use crate::execution::{execute_allowlisted_bounded, ExecuteRequest};
use crate::parsers::{parse_diagnostics, HarnessDiagnostic};

const CARGO_TOOL: &str = "cargo";
const CARGO_CHANNEL: &str = "+1.95.0";
const EXPECTED_CARGO_VERSION_RECORD: &str = "cargo 1.95.0 (f2d3ce0bd 2026-03-21)";
const CARGO_PROBE_COMMAND: [&str; 3] = [CARGO_TOOL, CARGO_CHANNEL, "--version"];
const CARGO_MAIN_COMMAND_PREFIX: [&str; 7] = [
    CARGO_TOOL,
    CARGO_CHANNEL,
    "check",
    "--offline",
    "--locked",
    "--message-format=json",
    "--target-dir",
];
const MAX_CONFIG_BYTES: u64 = 1_048_576;

/// Provenance marker for an input-tree digest supplied by a caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CargoInputTreeProvenance {
    /// The caller declared a digest; this packet does not enumerate the tree.
    DeclaredUnverified,
}

impl CargoInputTreeProvenance {
    /// Stable provenance spelling for later evidence consumers.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DeclaredUnverified => "declared-unverified",
        }
    }
}

/// Inputs required to run the one reviewed Cargo pilot.
pub struct CargoPilotInput<'a> {
    request: &'a ExecuteRequest,
    spec: &'a HarnessToolSpec,
    config_digest: Sha256,
    declared_input_tree_digest: Sha256,
}

impl<'a> CargoPilotInput<'a> {
    /// Construct a pilot request with separately framed manifest and lock bytes.
    pub fn try_new(
        request: &'a ExecuteRequest,
        spec: &'a HarnessToolSpec,
        declared_input_tree_digest: Sha256,
    ) -> Result<Self> {
        if request.tool.as_str() != CARGO_TOOL || spec.tool().as_str() != CARGO_TOOL {
            return Err(Error::InvalidConfig(
                "Cargo pilot requires the reviewed cargo tool identity".to_owned(),
            ));
        }
        if request.command != spec.command() {
            return Err(Error::InvalidConfig(
                "Cargo pilot request must equal the reviewed command contract".to_owned(),
            ));
        }
        validate_main_command(request.command.as_slice(), request)?;
        let probe = spec.probe().ok_or_else(|| {
            Error::InvalidConfig("Cargo pilot requires a reviewed version probe".to_owned())
        })?;
        let probe_command = probe.command();
        if probe_command.len() != CARGO_PROBE_COMMAND.len()
            || probe_command
                .iter()
                .zip(CARGO_PROBE_COMMAND)
                .any(|(actual, expected)| actual.as_str() != expected)
            || probe.output() != HarnessProbeOutput::Stdout
            || spec.expected_version().map(HarnessStepVersion::as_str)
                != Some(EXPECTED_CARGO_VERSION_RECORD)
        {
            return Err(Error::InvalidConfig(
                "Cargo pilot requires the exact pinned cargo stdout probe contract".to_owned(),
            ));
        }
        let config_digest = read_config_digest(request)?;
        Ok(Self {
            request,
            spec,
            config_digest,
            declared_input_tree_digest,
        })
    }
}

/// In-memory evidence returned by the Cargo pilot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoPilotEvidence {
    run_id: HarnessRunId,
    availability: HarnessToolAvailability,
    observed_version: Option<HarnessStepVersion>,
    probe_termination: Option<HarnessExecutionTermination>,
    execution_termination: Option<HarnessExecutionTermination>,
    exit_code: Option<ProcessExitCode>,
    command_digest: Sha256,
    config_digest: Sha256,
    captured_text_digest: Sha256,
    declared_input_tree_digest: Sha256,
    input_tree_provenance: CargoInputTreeProvenance,
    status: HarnessRunStatus,
    diagnostics: Vec<HarnessDiagnostic>,
}

impl CargoPilotEvidence {
    /// Stable run identity.
    #[must_use]
    pub const fn run_id(&self) -> &HarnessRunId {
        &self.run_id
    }

    /// Typed availability outcome.
    #[must_use]
    pub const fn availability(&self) -> HarnessToolAvailability {
        self.availability
    }

    /// Exact reviewed Cargo version observed by the probe.
    #[must_use]
    pub const fn observed_version(&self) -> Option<&HarnessStepVersion> {
        self.observed_version.as_ref()
    }

    /// Underlying availability-probe termination, distinct from main execution.
    #[must_use]
    pub const fn probe_termination(&self) -> Option<HarnessExecutionTermination> {
        self.probe_termination
    }

    /// Main execution termination, absent when the probe blocked execution.
    #[must_use]
    pub const fn execution_termination(&self) -> Option<HarnessExecutionTermination> {
        self.execution_termination
    }

    /// Main execution exit code, absent when no child was run.
    #[must_use]
    pub const fn exit_code(&self) -> Option<ProcessExitCode> {
        self.exit_code
    }

    /// Digest of the length-prefixed reviewed command arguments.
    #[must_use]
    pub const fn command_digest(&self) -> &Sha256 {
        &self.command_digest
    }

    /// Digest of separately framed Cargo.toml and Cargo.lock bytes.
    #[must_use]
    pub const fn config_digest(&self) -> &Sha256 {
        &self.config_digest
    }

    /// Digest of normalized captured text plus termination and exit code.
    #[must_use]
    pub const fn captured_text_digest(&self) -> &Sha256 {
        &self.captured_text_digest
    }

    /// Caller-declared input-tree digest.
    #[must_use]
    pub const fn declared_input_tree_digest(&self) -> &Sha256 {
        &self.declared_input_tree_digest
    }

    /// Provenance of the input-tree digest.
    #[must_use]
    pub const fn input_tree_provenance(&self) -> CargoInputTreeProvenance {
        self.input_tree_provenance
    }

    /// Derived run status; unavailable and non-completed outcomes never pass.
    #[must_use]
    pub const fn status(&self) -> HarnessRunStatus {
        self.status
    }

    /// Normalized Cargo/Rust diagnostics from a completed or non-zero run.
    #[must_use]
    pub fn diagnostics(&self) -> &[HarnessDiagnostic] {
        &self.diagnostics
    }
}

/// Run the reviewed Cargo pilot entirely in memory.
pub fn run_cargo_pilot(input: CargoPilotInput<'_>) -> Result<CargoPilotEvidence> {
    let command_parts = input
        .request
        .command
        .iter()
        .map(|argument| argument.as_str().as_bytes().to_vec())
        .collect::<Vec<_>>();
    let command_digest = digest_framed(command_parts.iter().map(Vec::as_slice));
    let config_digest = input.config_digest.clone();
    let probe = probe_allowlisted_tool(input.request, input.spec)?;
    let probe_termination = probe.termination();
    let base = |execution_termination, exit_code, captured_text_digest, diagnostics, status| {
        CargoPilotEvidence {
            run_id: input.request.run_id.clone(),
            availability: probe.availability(),
            observed_version: probe.observed_version().cloned(),
            probe_termination,
            execution_termination,
            exit_code,
            command_digest: command_digest.clone(),
            config_digest: config_digest.clone(),
            captured_text_digest,
            declared_input_tree_digest: input.declared_input_tree_digest.clone(),
            input_tree_provenance: CargoInputTreeProvenance::DeclaredUnverified,
            status,
            diagnostics,
        }
    };

    if probe.availability() != HarnessToolAvailability::Available {
        let captured_text_digest = digest_probe_blocked(
            probe.availability(),
            probe.termination(),
            probe.observed_version(),
        );
        return Ok(base(
            None,
            None,
            captured_text_digest,
            Vec::new(),
            HarnessRunStatus::Failed,
        ));
    }

    let after_probe_config_digest = read_config_digest(input.request)?;
    if after_probe_config_digest != input.config_digest {
        return Err(Error::InvalidConfig(
            "Cargo manifest or lock changed after review and before execution".to_owned(),
        ));
    }
    let before_main_config_digest = read_config_digest(input.request)?;
    if before_main_config_digest != input.config_digest {
        return Err(Error::InvalidConfig(
            "Cargo manifest or lock changed immediately before execution".to_owned(),
        ));
    }

    let execution = execute_allowlisted_bounded(input.request, input.spec)?;
    let termination = execution.termination();
    let exit_code = execution.exit_code();
    let captured_text_digest = digest_output(
        Some(termination),
        exit_code,
        termination.as_str(),
        execution.stdout().as_str(),
        execution.stderr().as_str(),
    );
    let diagnostics = diagnostics_for_execution(
        termination,
        input.request.run_id.as_str(),
        execution.stdout().as_str(),
        execution.stderr().as_str(),
    );
    let status = status_for_execution(termination, &diagnostics);
    Ok(base(
        Some(termination),
        exit_code,
        captured_text_digest,
        diagnostics,
        status,
    ))
}

fn digest_framed<'a, I>(parts: I) -> Sha256
where
    I: IntoIterator<Item = &'a [u8]>,
{
    let mut framed = Vec::new();
    for part in parts {
        let length = format!("{:016x}", part.len());
        framed.extend_from_slice(length.as_bytes());
        framed.extend_from_slice(part);
    }
    validate(&framed)
}

fn digest_output(
    termination: Option<HarnessExecutionTermination>,
    exit_code: Option<ProcessExitCode>,
    termination_label: &str,
    stdout: &str,
    stderr: &str,
) -> Sha256 {
    let termination_text = termination
        .map(|value| value.as_str())
        .unwrap_or(termination_label);
    let exit_code_text = exit_code.map(|value| value.get().to_string());
    let exit_code_bytes = exit_code_text.as_deref().unwrap_or("none");
    digest_framed([
        b"termination".as_slice(),
        termination_text.as_bytes(),
        b"exit-code".as_slice(),
        exit_code_bytes.as_bytes(),
        b"stdout".as_slice(),
        stdout.as_bytes(),
        b"stderr".as_slice(),
        stderr.as_bytes(),
    ])
}

fn digest_probe_blocked(
    availability: HarnessToolAvailability,
    termination: Option<HarnessExecutionTermination>,
    observed_version: Option<&HarnessStepVersion>,
) -> Sha256 {
    let termination = termination
        .map(HarnessExecutionTermination::as_str)
        .unwrap_or("none");
    let version = observed_version
        .map(HarnessStepVersion::as_str)
        .unwrap_or("none");
    digest_framed([
        b"availability".as_slice(),
        availability.as_str().as_bytes(),
        b"probe-termination".as_slice(),
        termination.as_bytes(),
        b"observed-version".as_slice(),
        version.as_bytes(),
    ])
}

fn diagnostics_for_execution(
    termination: HarnessExecutionTermination,
    run_id: &str,
    stdout: &str,
    stderr: &str,
) -> Vec<HarnessDiagnostic> {
    match termination {
        HarnessExecutionTermination::Completed | HarnessExecutionTermination::NonZeroExit => {
            parse_diagnostics(run_id, CARGO_TOOL, stdout, stderr)
        }
        HarnessExecutionTermination::MissingExecutable
        | HarnessExecutionTermination::SpawnFailed
        | HarnessExecutionTermination::TimedOut
        | HarnessExecutionTermination::OutputLimitExceeded => Vec::new(),
    }
}

fn status_for_execution(
    termination: HarnessExecutionTermination,
    diagnostics: &[HarnessDiagnostic],
) -> HarnessRunStatus {
    let has_error = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == enforcer_domain::severity::Severity::Error);
    if termination == HarnessExecutionTermination::Completed && !has_error {
        HarnessRunStatus::Passed
    } else {
        HarnessRunStatus::Failed
    }
}

fn validate_main_command(
    command: &[enforcer_domain::harness_types::HarnessCommandArgument],
    request: &ExecuteRequest,
) -> Result<()> {
    if command.len() != CARGO_MAIN_COMMAND_PREFIX.len() + 1
        || command
            .iter()
            .take(CARGO_MAIN_COMMAND_PREFIX.len())
            .zip(CARGO_MAIN_COMMAND_PREFIX)
            .any(|(actual, expected)| actual.as_str() != expected)
    {
        return Err(Error::InvalidConfig(
            "Cargo pilot requires the exact offline locked check command".to_owned(),
        ));
    }
    let target_dir = command.last().ok_or_else(|| {
        Error::InvalidConfig("Cargo target directory argument is required".to_owned())
    })?;
    validate_disposable_target_dir(request, target_dir.as_str())
}

fn validate_disposable_target_dir(request: &ExecuteRequest, target_dir: &str) -> Result<()> {
    let target = Path::new(target_dir);
    if target_dir.trim().is_empty()
        || target.components().any(|component| {
            matches!(
                component,
                Component::Prefix(_) | Component::RootDir | Component::ParentDir
            )
        })
    {
        return Err(Error::InvalidConfig(
            "Cargo target directory must be repository-relative".to_owned(),
        ));
    }
    let root = fs::canonicalize(request.repo_root.as_str()).map_err(|error| {
        Error::InvalidConfig(format!(
            "Cargo disposable repository root is invalid: {error}"
        ))
    })?;
    let cwd = request.cwd.as_deref().unwrap_or("");
    let candidate = Path::new(request.repo_root.as_str()).join(cwd).join(target);
    let existing = nearest_existing_path(&candidate).ok_or_else(|| {
        Error::InvalidConfig("Cargo target directory has no contained existing ancestor".to_owned())
    })?;
    let existing = fs::canonicalize(existing).map_err(|error| {
        Error::InvalidConfig(format!(
            "Cargo target directory containment failed: {error}"
        ))
    })?;
    if !existing.starts_with(&root) || existing == root {
        return Err(Error::InvalidConfig(
            "Cargo target directory must remain below the disposable repository root".to_owned(),
        ));
    }
    Ok(())
}

fn read_config_digest(request: &ExecuteRequest) -> Result<Sha256> {
    let cwd = reviewed_cwd(request)?;
    let cargo_toml = read_reviewed_config_file(&cwd, "Cargo.toml")?;
    let cargo_lock = read_reviewed_config_file(&cwd, "Cargo.lock")?;
    Ok(digest_framed([
        b"Cargo.toml".as_slice(),
        cargo_toml.as_slice(),
        b"Cargo.lock".as_slice(),
        cargo_lock.as_slice(),
    ]))
}

fn reviewed_cwd(request: &ExecuteRequest) -> Result<PathBuf> {
    let root = fs::canonicalize(request.repo_root.as_str()).map_err(|error| {
        Error::InvalidConfig(format!(
            "Cargo disposable repository root is invalid: {error}"
        ))
    })?;
    let candidate =
        Path::new(request.repo_root.as_str()).join(request.cwd.as_deref().unwrap_or(""));
    let cwd = fs::canonicalize(candidate).map_err(|error| {
        Error::InvalidConfig(format!("Cargo disposable cwd is invalid: {error}"))
    })?;
    if !cwd.starts_with(&root) {
        return Err(Error::InvalidConfig(
            "Cargo disposable cwd must remain under the repository root".to_owned(),
        ));
    }
    Ok(cwd)
}

fn read_reviewed_config_file(cwd: &Path, name: &str) -> Result<Vec<u8>> {
    let path = cwd.join(name);
    let metadata = fs::symlink_metadata(&path).map_err(|error| {
        Error::InvalidConfig(format!("Cargo {name} metadata could not be read: {error}"))
    })?;
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || has_reparse_point(&metadata)
    {
        return Err(Error::InvalidConfig(format!(
            "Cargo {name} must be a regular non-link file"
        )));
    }
    if metadata.len() > MAX_CONFIG_BYTES {
        return Err(Error::InvalidConfig(format!(
            "Cargo {name} exceeds the reviewed configuration size bound"
        )));
    }
    let bytes = fs::read(&path).map_err(|error| {
        Error::InvalidConfig(format!("Cargo {name} could not be read: {error}"))
    })?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_CONFIG_BYTES {
        return Err(Error::InvalidConfig(format!(
            "Cargo {name} changed beyond the reviewed configuration size bound"
        )));
    }
    Ok(bytes)
}

#[cfg(windows)]
fn has_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
const fn has_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

fn nearest_existing_path(path: &Path) -> Option<&Path> {
    let mut current = path;
    loop {
        if current.exists() {
            return Some(current);
        }
        current = current.parent()?;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        diagnostics_for_execution, digest_output, digest_probe_blocked, status_for_execution,
    };
    use enforcer_domain::harness_types::{
        HarnessExecutionTermination, HarnessRunStatus, HarnessToolAvailability,
    };
    use enforcer_domain::telemetry_types::ProcessExitCode;

    #[test]
    fn cargo_adapter_missing_executable_remains_failed_without_diagnostics() {
        let termination = HarnessExecutionTermination::MissingExecutable;
        let diagnostics = diagnostics_for_execution(termination, "cargo-test", "{}", "{}\n");
        assert!(diagnostics.is_empty());
        assert_eq!(
            status_for_execution(termination, &diagnostics),
            HarnessRunStatus::Failed
        );
    }

    #[test]
    fn cargo_adapter_timeout_remains_failed_without_diagnostics() {
        let termination = HarnessExecutionTermination::TimedOut;
        let diagnostics = diagnostics_for_execution(termination, "cargo-test", "{}", "{}");
        assert!(diagnostics.is_empty());
        assert_eq!(
            status_for_execution(termination, &diagnostics),
            HarnessRunStatus::Failed
        );
    }

    #[test]
    fn cargo_adapter_output_overflow_remains_failed_without_diagnostics() {
        let termination = HarnessExecutionTermination::OutputLimitExceeded;
        let diagnostics = diagnostics_for_execution(termination, "cargo-test", "{}", "{}");
        assert!(diagnostics.is_empty());
        assert_eq!(
            status_for_execution(termination, &diagnostics),
            HarnessRunStatus::Failed
        );
    }

    #[test]
    fn cargo_adapter_malformed_probe_remains_failed_without_main_diagnostics() {
        let digest = digest_probe_blocked(
            HarnessToolAvailability::MalformedOutput,
            Some(HarnessExecutionTermination::Completed),
            None,
        );
        let other_digest = digest_probe_blocked(
            HarnessToolAvailability::Missing,
            Some(HarnessExecutionTermination::MissingExecutable),
            None,
        );
        assert_ne!(digest, other_digest);
        let diagnostics = diagnostics_for_execution(
            HarnessExecutionTermination::Completed,
            "cargo-test",
            "malformed probe output",
            "",
        );
        assert!(diagnostics.is_empty());
        assert_eq!(
            status_for_execution(HarnessExecutionTermination::NonZeroExit, &diagnostics),
            HarnessRunStatus::Failed
        );
    }

    #[test]
    fn cargo_adapter_noncompleted_states_never_pass_or_fabricate_diagnostics() {
        for termination in [
            HarnessExecutionTermination::NonZeroExit,
            HarnessExecutionTermination::MissingExecutable,
            HarnessExecutionTermination::SpawnFailed,
            HarnessExecutionTermination::TimedOut,
            HarnessExecutionTermination::OutputLimitExceeded,
        ] {
            let diagnostics = diagnostics_for_execution(termination, "cargo-test", "", "");
            assert_eq!(
                status_for_execution(termination, &diagnostics),
                HarnessRunStatus::Failed
            );
            if termination != HarnessExecutionTermination::NonZeroExit {
                assert!(diagnostics.is_empty());
            }
        }
    }

    #[test]
    fn cargo_adapter_captured_text_digest_is_deterministic() {
        let first = digest_output(
            Some(HarnessExecutionTermination::Completed),
            Some(ProcessExitCode::new(0)),
            "completed",
            "stdout\n",
            "stderr\n",
        );
        let second = digest_output(
            Some(HarnessExecutionTermination::Completed),
            Some(ProcessExitCode::new(0)),
            "completed",
            "stdout\n",
            "stderr\n",
        );
        assert_eq!(first, second);
        assert_ne!(
            first,
            digest_output(
                Some(HarnessExecutionTermination::Completed),
                Some(ProcessExitCode::new(0)),
                "completed",
                "changed\n",
                "stderr\n",
            )
        );
    }
}
