// source owner: crates/enforcer-domain/src/harness_types.rs
// generator: cargo test -p enforcer-harness --test tool_adapter_contract
// contractHash: 2ccae7474073653d7be42bbd3903bac8c1c818c0b3ddf70c7350381e2837299a

use enforcer_core::error::{Error, Result};
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::config_types::HarnessConfig;
use enforcer_domain::harness_types::{
    HarnessBoundedExecution, HarnessCommandArgument, HarnessExecutionLimits,
    HarnessExecutionTermination, HarnessToolAvailability, HarnessToolDecision, HarnessToolName,
    HarnessToolRequirement, HarnessToolSpec,
};
use enforcer_domain::paths::RepoRoot;
use enforcer_harness::execution::{
    execute_allowlisted_bounded, validate_allowlisted_request, ExecuteRequest,
};

fn spec() -> Result<HarnessToolSpec> {
    spec_with_command(
        vec![
            HarnessCommandArgument::try_new("cargo".to_owned())?,
            HarnessCommandArgument::try_new("check".to_owned())?,
        ],
        10_000,
        1_048_576,
    )
}

fn spec_with_command(
    command: Vec<HarnessCommandArgument>,
    max_wall_time_ms: u64,
    max_output_bytes: u64,
) -> Result<HarnessToolSpec> {
    HarnessToolSpec::try_new(
        HarnessToolName::try_new("cargo".to_owned())?,
        command,
        HarnessToolRequirement::Required,
        HarnessExecutionLimits::try_new(max_wall_time_ms, max_output_bytes, 100)?,
        None,
    )
    .map_err(Into::into)
}

fn child_command(test_name: &str) -> Result<Vec<HarnessCommandArgument>> {
    let executable = std::env::current_exe()
        .map_err(|error| Error::InvalidConfig(format!("current test executable: {error}")))?;
    Ok(vec![
        HarnessCommandArgument::try_new(executable.to_string_lossy().into_owned())?,
        HarnessCommandArgument::try_new("--exact".to_owned())?,
        HarnessCommandArgument::try_new(test_name.to_owned())?,
        HarnessCommandArgument::try_new("--nocapture".to_owned())?,
        HarnessCommandArgument::try_new("--".to_owned())?,
        HarnessCommandArgument::try_new("--ul07-child".to_owned())?,
        HarnessCommandArgument::try_new("&|$()".to_owned())?,
    ])
}

fn request(
    repo_root: RepoRoot,
    command: Vec<HarnessCommandArgument>,
    cwd: Option<String>,
) -> Result<ExecuteRequest> {
    Ok(ExecuteRequest {
        repo_root,
        cwd,
        run_id: enforcer_domain::harness_types::HarnessRunId::try_new("contract-run".to_owned())?,
        tool: HarnessToolName::try_new("cargo".to_owned())?,
        language: None,
        command,
        crate_name: None,
        package_name: None,
        domain: None,
        tags: vec![],
    })
}

fn assert_decode_error<T>(
    result: std::result::Result<T, DecodeError>,
    path: &str,
) -> std::result::Result<(), DecodeError> {
    match result {
        Ok(_) => Err(DecodeError::new(path, "expected rejection")),
        Err(error) if error.path == path => Ok(()),
        Err(error) => Err(error),
    }
}

fn assert_rejection<T>(result: Result<T>, expected: &str) -> Result<()> {
    let error = match result {
        Ok(_) => {
            return Err(Error::InvalidConfig(format!(
                "expected rejection containing {expected}"
            )))
        }
        Err(error) => error,
    };
    if !error.to_string().contains(expected) {
        return Err(Error::InvalidConfig(format!(
            "rejection did not contain {expected}: {error}"
        )));
    }
    Ok(())
}

fn assert_termination(
    outcome: &HarnessBoundedExecution,
    expected: HarnessExecutionTermination,
    expected_reaped: bool,
) -> Result<()> {
    if outcome.termination() != expected {
        return Err(Error::InvalidConfig(format!(
            "expected termination {}, got {}",
            expected.as_str(),
            outcome.termination().as_str()
        )));
    }
    if outcome.child_reaped() != expected_reaped {
        return Err(Error::InvalidConfig(format!(
            "bounded child reaped state was {}, expected {}",
            outcome.child_reaped(),
            expected_reaped
        )));
    }
    Ok(())
}

#[test]
fn required_non_available_states_block_without_collapsing_the_state() -> Result<()> {
    for availability in [
        HarnessToolAvailability::Missing,
        HarnessToolAvailability::VersionMismatch,
        HarnessToolAvailability::Misconfigured,
        HarnessToolAvailability::TimedOut,
        HarnessToolAvailability::Failed,
        HarnessToolAvailability::MalformedOutput,
    ] {
        assert_eq!(
            availability.decision(HarnessToolRequirement::Required),
            HarnessToolDecision::Block,
            "{} must block a required tool",
            availability.as_str()
        );
        assert_eq!(
            availability.decision(HarnessToolRequirement::Optional),
            HarnessToolDecision::Warn,
            "{} must warn for an optional tool",
            availability.as_str()
        );
        assert_eq!(
            availability.decision(HarnessToolRequirement::Advisory),
            HarnessToolDecision::NotApplicable,
            "{} must remain not-applicable for an advisory tool",
            availability.as_str()
        );
    }
    assert_eq!(
        HarnessToolAvailability::Available.decision(HarnessToolRequirement::Required),
        HarnessToolDecision::Run
    );
    Ok(())
}

#[test]
fn execution_limits_and_command_templates_are_closed_and_non_zero() -> Result<()> {
    let limits = HarnessExecutionLimits::try_new(10, 20, 30)?;
    assert_eq!(limits.max_wall_time_ms(), 10);
    assert_eq!(limits.max_output_bytes(), 20);
    assert_eq!(limits.max_files(), 30);
    assert_decode_error(HarnessExecutionLimits::try_new(0, 20, 30), "maxWallTimeMs")?;
    assert_decode_error(HarnessExecutionLimits::try_new(10, 0, 30), "maxOutputBytes")?;
    assert_decode_error(HarnessExecutionLimits::try_new(10, 20, 0), "maxFiles")?;
    assert_decode_error(
        HarnessToolSpec::try_new(
            HarnessToolName::try_new("cargo".to_owned())?,
            vec![],
            HarnessToolRequirement::Required,
            limits,
            None,
        ),
        "command",
    )?;
    Ok(())
}

#[test]
fn allowlisted_validation_requires_exact_command_and_repository_relative_cwd() -> Result<()> {
    let temp = tempfile::TempDir::new()?;
    let root = RepoRoot::try_from(temp.path())?;
    let reviewed = spec()?;
    let command = reviewed.command().to_vec();
    let accepted = request(root.clone(), command.clone(), Some("crates".to_owned()))?;
    validate_allowlisted_request(&accepted, &reviewed)?;

    let wrong_command = request(
        root.clone(),
        vec![
            HarnessCommandArgument::try_new("cargo".to_owned())?,
            HarnessCommandArgument::try_new("test".to_owned())?,
        ],
        None,
    )?;
    assert_rejection(
        validate_allowlisted_request(&wrong_command, &reviewed),
        "reviewed template",
    )?;

    let traversal = request(root.clone(), command.clone(), Some("../outside".to_owned()))?;
    assert_rejection(
        validate_allowlisted_request(&traversal, &reviewed),
        "repository root",
    )?;

    let absolute = request(
        root.clone(),
        command.clone(),
        Some(temp.path().to_string_lossy().into_owned()),
    )?;
    assert_rejection(
        validate_allowlisted_request(&absolute, &reviewed),
        "repository root",
    )?;

    #[cfg(windows)]
    {
        let rooted_without_prefix = request(
            root.clone(),
            reviewed.command().to_vec(),
            Some(r"\outside".to_owned()),
        )?;
        assert_rejection(
            validate_allowlisted_request(&rooted_without_prefix, &reviewed),
            "repository root",
        )?;

        let drive_relative = request(
            root,
            reviewed.command().to_vec(),
            Some(r"C:outside".to_owned()),
        )?;
        assert_rejection(
            validate_allowlisted_request(&drive_relative, &reviewed),
            "repository root",
        )?;
    }
    Ok(())
}

#[test]
fn bounded_runner_captures_both_streams_and_zero_exit() -> Result<()> {
    let temp = tempfile::TempDir::new()?;
    let root = RepoRoot::try_from(temp.path())?;
    let command = child_command("child_entry_outputs")?;
    let reviewed = spec_with_command(command.clone(), 2_000, 1_024)?;
    let outcome = execute_allowlisted_bounded(&request(root, command, None)?, &reviewed)?;
    assert_termination(&outcome, HarnessExecutionTermination::Completed, true)?;
    if !outcome.stdout().as_str().contains("bounded-stdout") {
        return Err(Error::InvalidConfig(
            "bounded stdout did not preserve the child marker".to_owned(),
        ));
    }
    if !outcome.stderr().as_str().contains("bounded-stderr") {
        return Err(Error::InvalidConfig(
            "bounded stderr did not preserve the child marker".to_owned(),
        ));
    }
    if outcome.exit_code().map(|code| code.get()) != Some(0) {
        return Err(Error::InvalidConfig(
            "bounded zero-exit result did not preserve exit code".to_owned(),
        ));
    }
    Ok(())
}

#[test]
fn bounded_runner_distinguishes_nonzero_exit_and_missing_executable() -> Result<()> {
    let temp = tempfile::TempDir::new()?;
    let root = RepoRoot::try_from(temp.path())?;
    let command = child_command("child_entry_exits_nonzero")?;
    let reviewed = spec_with_command(command.clone(), 2_000, 1_024)?;
    let nonzero = execute_allowlisted_bounded(&request(root.clone(), command, None)?, &reviewed)?;
    assert_termination(&nonzero, HarnessExecutionTermination::NonZeroExit, true)?;
    if nonzero.exit_code().map(|code| code.get()) != Some(7) {
        return Err(Error::InvalidConfig(
            "non-zero child result did not preserve exit code 7".to_owned(),
        ));
    }

    let missing = vec![HarnessCommandArgument::try_new(
        "ocentra-definitely-missing-ul07-tool".to_owned(),
    )?];
    let missing_spec = spec_with_command(missing.clone(), 2_000, 1_024)?;
    let missing_outcome =
        execute_allowlisted_bounded(&request(root, missing, None)?, &missing_spec)?;
    assert_termination(
        &missing_outcome,
        HarnessExecutionTermination::MissingExecutable,
        false,
    )
}

#[test]
fn bounded_runner_terminates_timeout_child_before_sentinel() -> Result<()> {
    let temp = tempfile::TempDir::new()?;
    let root = RepoRoot::try_from(temp.path())?;
    let sentinel = temp.path().join("ul07-timeout-sentinel");
    let command = child_command("child_entry_writes_timeout_sentinel")?;
    let reviewed = spec_with_command(command.clone(), 40, 1_024)?;
    let outcome = execute_allowlisted_bounded(&request(root, command, None)?, &reviewed)?;
    assert_termination(&outcome, HarnessExecutionTermination::TimedOut, true)?;
    std::thread::park_timeout(std::time::Duration::from_millis(400));
    if sentinel.exists() {
        return Err(Error::InvalidConfig(
            "timed-out child wrote its post-boundary sentinel".to_owned(),
        ));
    }
    Ok(())
}

#[test]
fn bounded_runner_terminates_overflow_child_before_sentinel() -> Result<()> {
    let temp = tempfile::TempDir::new()?;
    let root = RepoRoot::try_from(temp.path())?;
    let sentinel = temp.path().join("ul07-overflow-sentinel");
    let command = child_command("child_entry_overflows_then_writes_sentinel")?;
    let reviewed = spec_with_command(command.clone(), 2_000, 32)?;
    let outcome = execute_allowlisted_bounded(&request(root, command, None)?, &reviewed)?;
    assert_termination(
        &outcome,
        HarnessExecutionTermination::OutputLimitExceeded,
        true,
    )?;
    if outcome.stdout().as_str().len() + outcome.stderr().as_str().len() > 32 {
        return Err(Error::InvalidConfig(
            "bounded output retained more than the combined byte limit".to_owned(),
        ));
    }
    std::thread::park_timeout(std::time::Duration::from_millis(400));
    if sentinel.exists() {
        return Err(Error::InvalidConfig(
            "overflow child wrote its post-boundary sentinel".to_owned(),
        ));
    }
    Ok(())
}

#[test]
fn bounded_runner_preserves_literal_shell_metacharacters_and_calls_allowlist() -> Result<()> {
    let temp = tempfile::TempDir::new()?;
    let root = RepoRoot::try_from(temp.path())?;
    let command = child_command("child_entry_reports_literal_argument")?;
    let reviewed = spec_with_command(command.clone(), 2_000, 1_024)?;
    let outcome =
        execute_allowlisted_bounded(&request(root.clone(), command.clone(), None)?, &reviewed)?;
    assert_termination(&outcome, HarnessExecutionTermination::Completed, true)?;
    if !outcome.stdout().as_str().contains("literal=true") {
        return Err(Error::InvalidConfig(
            "shell metacharacter was not passed as one literal argument".to_owned(),
        ));
    }

    let mismatch = request(
        root,
        vec![HarnessCommandArgument::try_new(
            "different-tool".to_owned(),
        )?],
        Some("../outside".to_owned()),
    )?;
    assert_rejection(
        execute_allowlisted_bounded(&mismatch, &reviewed),
        "allowlisted command",
    )
}

#[test]
fn bounded_runner_caps_lossy_utf8_without_expansion() -> Result<()> {
    let temp = tempfile::TempDir::new()?;
    let root = RepoRoot::try_from(temp.path())?;
    let command = child_command("child_entry_writes_invalid_utf8")?;
    let reviewed = spec_with_command(command.clone(), 2_000, 32)?;
    let outcome = execute_allowlisted_bounded(&request(root, command, None)?, &reviewed)?;
    assert_termination(
        &outcome,
        HarnessExecutionTermination::OutputLimitExceeded,
        true,
    )?;
    if outcome.stdout().as_str().len() + outcome.stderr().as_str().len() > 32 {
        return Err(Error::InvalidConfig(
            "lossy UTF-8 output exceeded the combined byte limit".to_owned(),
        ));
    }
    Ok(())
}

#[test]
fn child_entry_outputs() {
    if child_mode() {
        print!("bounded-stdout");
        eprint!("bounded-stderr");
    }
}

#[test]
fn child_entry_exits_nonzero() {
    if child_mode() {
        std::process::exit(7);
    }
}

#[test]
fn child_entry_writes_timeout_sentinel() {
    if child_mode() {
        std::thread::park_timeout(std::time::Duration::from_millis(250));
        let _ = std::fs::write("ul07-timeout-sentinel", "late");
    }
}

#[test]
fn child_entry_overflows_then_writes_sentinel() {
    if child_mode() {
        use std::io::Write;

        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(&vec![b'x'; 2_048]);
        let _ = stdout.flush();
        std::thread::park_timeout(std::time::Duration::from_millis(250));
        let _ = std::fs::write("ul07-overflow-sentinel", "late");
    }
}

#[test]
fn child_entry_reports_literal_argument() {
    if child_mode() {
        let literal = std::env::args().any(|value| value == "&|$()");
        print!("literal={literal}");
    }
}

#[test]
fn child_entry_writes_invalid_utf8() {
    if child_mode() {
        use std::io::Write;

        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(&[0xff; 64]);
        let _ = stdout.flush();
    }
}

fn child_mode() -> bool {
    std::env::args().any(|value| value == "--ul07-child")
}

#[test]
fn arbitrary_execute_remains_a_distinct_user_invoked_surface() {
    let _ = HarnessConfig::default();
    let function_name = enforcer_harness::execution::execute
        as fn(
            &ExecuteRequest,
            &HarnessConfig,
        ) -> enforcer_core::error::Result<enforcer_harness::storage::RunOutcome>;
    let _ = function_name;
}
