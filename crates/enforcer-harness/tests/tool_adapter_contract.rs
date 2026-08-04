// source owner: crates/enforcer-domain/src/harness_types.rs
// generator: cargo test -p enforcer-harness --test tool_adapter_contract
// contractHash: 2ccae7474073653d7be42bbd3903bac8c1c818c0b3ddf70c7350381e2837299a

use enforcer_core::error::{Error, Result};
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::config_types::HarnessConfig;
use enforcer_domain::harness_types::{
    HarnessBoundedExecution, HarnessCommandArgument, HarnessExecutionLimits,
    HarnessExecutionTermination, HarnessInputLimits, HarnessProbeOutput, HarnessStepVersion,
    HarnessToolAvailability, HarnessToolDecision, HarnessToolName, HarnessToolProbe,
    HarnessToolRequirement, HarnessToolSpec,
};
use enforcer_domain::paths::RepoRoot;
use enforcer_harness::availability::probe_allowlisted_tool;
use enforcer_harness::execution::{
    execute_allowlisted_bounded, validate_allowlisted_request, ExecuteRequest,
};
use enforcer_harness::input_scope::compute_input_tree;

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

fn spec_with_probe(
    main_command: Vec<HarnessCommandArgument>,
    probe_command: Vec<HarnessCommandArgument>,
    output: HarnessProbeOutput,
    expected_version: &str,
) -> Result<HarnessToolSpec> {
    spec_with_probe_for_tool(ProbeSpec {
        tool: HarnessToolName::try_new("cargo".to_owned())?,
        main_command,
        probe_command,
        output,
        expected_version,
        max_wall_time_ms: 2_000,
        max_output_bytes: 1_024,
    })
}

struct ProbeSpec<'a> {
    tool: HarnessToolName,
    main_command: Vec<HarnessCommandArgument>,
    probe_command: Vec<HarnessCommandArgument>,
    output: HarnessProbeOutput,
    expected_version: &'a str,
    max_wall_time_ms: u64,
    max_output_bytes: u64,
}

fn spec_with_probe_for_tool(spec: ProbeSpec<'_>) -> Result<HarnessToolSpec> {
    let expected_version = HarnessStepVersion::try_new(spec.expected_version.to_owned())?;
    let probe = HarnessToolProbe::try_new(spec.probe_command, spec.output)?;
    Ok(HarnessToolSpec::try_new(
        spec.tool,
        spec.main_command,
        HarnessToolRequirement::Required,
        HarnessExecutionLimits::try_new(spec.max_wall_time_ms, spec.max_output_bytes, 100)?,
        Some(expected_version),
    )?
    .with_probe(probe))
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
    request_with_tool(
        repo_root,
        command,
        cwd,
        HarnessToolName::try_new("cargo".to_owned())?,
    )
}

fn request_with_tool(
    repo_root: RepoRoot,
    command: Vec<HarnessCommandArgument>,
    cwd: Option<String>,
    tool: HarnessToolName,
) -> Result<ExecuteRequest> {
    Ok(ExecuteRequest {
        repo_root,
        cwd,
        run_id: enforcer_domain::harness_types::HarnessRunId::try_new("contract-run".to_owned())?,
        tool,
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
fn input_scope_adapter_contract_is_exact_three_files_and_bounded() -> Result<()> {
    let temp = tempfile::TempDir::new()
        .map_err(|error| Error::InvalidConfig(format!("input scope temp: {error}")))?;
    let fixture = temp.path().join("fixture");
    std::fs::create_dir_all(fixture.join("src"))
        .map_err(|error| Error::InvalidConfig(format!("input scope directories: {error}")))?;
    std::fs::write(
        fixture.join("Cargo.toml"),
        b"[package]\nname='scope'\nversion='0.1.0'\n",
    )
    .map_err(|error| Error::InvalidConfig(format!("input scope manifest: {error}")))?;
    std::fs::write(fixture.join("Cargo.lock"), b"version = 4\n")
        .map_err(|error| Error::InvalidConfig(format!("input scope lock: {error}")))?;
    std::fs::write(fixture.join("src/lib.rs"), b"pub fn scope() {}\n")
        .map_err(|error| Error::InvalidConfig(format!("input scope source: {error}")))?;
    let root = RepoRoot::try_from(temp.path())?;
    let command = [
        "cargo",
        "+1.95.0",
        "check",
        "--offline",
        "--locked",
        "--message-format=json",
        "--target-dir",
        "target",
    ]
    .into_iter()
    .map(|value| HarnessCommandArgument::try_new(value.to_owned()).map_err(Into::into))
    .collect::<Result<Vec<_>>>()?;
    let request = request(root, command.clone(), Some("fixture".to_owned()))?;
    let target = command
        .last()
        .ok_or_else(|| Error::InvalidConfig("input scope target command is empty".to_owned()))?;
    let limits = HarnessInputLimits::try_new(3, 4, 1_024, 4_096)?;
    let evidence = compute_input_tree(&request, target, limits)?;
    assert_eq!(evidence.file_count(), 3);
    assert_eq!(evidence.excluded_target(), "target");
    assert!(evidence.total_bytes() > 0);
    Ok(())
}

#[test]
fn availability_requires_one_complete_reviewed_probe_contract() -> Result<()> {
    let temp = tempfile::TempDir::new()?;
    let root = RepoRoot::try_from(temp.path())?;
    let main = child_command("child_entry_outputs")?;
    let probe_command = child_command("child_entry_probe_version")?;
    let missing_version = spec_with_command(main.clone(), 2_000, 1_024)?.with_probe(
        HarnessToolProbe::try_new(probe_command, HarnessProbeOutput::Stdout)?,
    );
    let result = probe_allowlisted_tool(&request(root, main, None)?, &missing_version)?;
    assert_eq!(
        result.availability(),
        HarnessToolAvailability::Misconfigured
    );
    assert_eq!(result.termination(), None);
    assert_eq!(result.observed_version(), None);
    assert_decode_error(
        HarnessToolProbe::try_new(vec![], HarnessProbeOutput::Stdout),
        "probe.command",
    )?;
    assert_decode_error(
        HarnessStepVersion::try_new("".to_owned()),
        "harnessStepVersion",
    )?;
    Ok(())
}

#[test]
fn availability_uses_only_the_reviewed_probe_command_and_exact_version() -> Result<()> {
    let temp = tempfile::TempDir::new()?;
    let root = RepoRoot::try_from(temp.path())?;
    let main = child_command("child_entry_outputs")?;
    let probe = child_command("child_entry_probe_version")?;
    let reviewed = spec_with_probe(main.clone(), probe, HarnessProbeOutput::Stderr, "1.2.3")?;
    let result = probe_allowlisted_tool(&request(root, main, None)?, &reviewed)?;
    assert_eq!(
        result.availability(),
        HarnessToolAvailability::Available,
        "observed={:?} termination={:?}",
        result.observed_version().map(HarnessStepVersion::as_str),
        result.termination()
    );
    assert_eq!(
        result.termination(),
        Some(HarnessExecutionTermination::Completed)
    );
    assert_eq!(
        result.observed_version().map(HarnessStepVersion::as_str),
        Some("1.2.3")
    );
    Ok(())
}

#[test]
fn availability_preserves_missing_spawn_and_nonzero_causality() -> Result<()> {
    let temp = tempfile::TempDir::new()?;
    let root = RepoRoot::try_from(temp.path())?;
    let main = child_command("child_entry_outputs")?;
    let missing_main = vec![HarnessCommandArgument::try_new(
        "ocentra-definitely-missing-ul07-probe".to_owned(),
    )?];
    let missing_spec = spec_with_probe(
        missing_main.clone(),
        missing_main,
        HarnessProbeOutput::Stdout,
        "1.2.3",
    )?;
    let missing = probe_allowlisted_tool(
        &request(root.clone(), child_command("child_entry_outputs")?, None)?,
        &missing_spec,
    )?;
    assert_eq!(missing.availability(), HarnessToolAvailability::Missing);
    assert_eq!(
        missing.termination(),
        Some(HarnessExecutionTermination::MissingExecutable)
    );
    assert_eq!(missing.observed_version(), None);

    let not_a_directory = temp.path().join("not-a-directory");
    std::fs::write(&not_a_directory, "file")?;
    let spawn_probe = child_command("child_entry_probe_version")?;
    let spawn_spec = spec_with_probe(
        main.clone(),
        spawn_probe,
        HarnessProbeOutput::Stdout,
        "1.2.3",
    )?;
    let spawn_failed = probe_allowlisted_tool(
        &request(
            root.clone(),
            main.clone(),
            Some("not-a-directory".to_owned()),
        )?,
        &spawn_spec,
    )?;
    assert_eq!(spawn_failed.availability(), HarnessToolAvailability::Failed);
    assert_eq!(
        spawn_failed.termination(),
        Some(HarnessExecutionTermination::SpawnFailed)
    );

    let nonzero_probe = child_command("child_entry_exits_nonzero")?;
    let nonzero_spec = spec_with_probe(main, nonzero_probe, HarnessProbeOutput::Stdout, "1.2.3")?;
    let nonzero = probe_allowlisted_tool(
        &request(root, child_command("child_entry_outputs")?, None)?,
        &nonzero_spec,
    )?;
    assert_eq!(nonzero.availability(), HarnessToolAvailability::Failed);
    assert_eq!(
        nonzero.termination(),
        Some(HarnessExecutionTermination::NonZeroExit)
    );
    Ok(())
}

#[test]
fn availability_maps_timeout_and_output_overflow_to_closed_states() -> Result<()> {
    let temp = tempfile::TempDir::new()?;
    let root = RepoRoot::try_from(temp.path())?;
    let main = child_command("child_entry_outputs")?;

    let timeout_spec = spec_with_probe_for_tool(ProbeSpec {
        tool: HarnessToolName::try_new("cargo".to_owned())?,
        main_command: main.clone(),
        probe_command: child_command("child_entry_writes_timeout_sentinel")?,
        output: HarnessProbeOutput::Stdout,
        expected_version: "1.2.3",
        max_wall_time_ms: 40,
        max_output_bytes: 1_024,
    })?;
    let timeout =
        probe_allowlisted_tool(&request(root.clone(), main.clone(), None)?, &timeout_spec)?;
    assert_eq!(timeout.availability(), HarnessToolAvailability::TimedOut);
    assert_eq!(
        timeout.termination(),
        Some(HarnessExecutionTermination::TimedOut)
    );
    assert_eq!(timeout.observed_version(), None);

    let overflow_spec = spec_with_probe_for_tool(ProbeSpec {
        tool: HarnessToolName::try_new("cargo".to_owned())?,
        main_command: main.clone(),
        probe_command: child_command("child_entry_overflows_then_writes_sentinel")?,
        output: HarnessProbeOutput::Stdout,
        expected_version: "1.2.3",
        max_wall_time_ms: 2_000,
        max_output_bytes: 32,
    })?;
    let overflow = probe_allowlisted_tool(&request(root, main, None)?, &overflow_spec)?;
    assert_eq!(
        overflow.availability(),
        HarnessToolAvailability::MalformedOutput
    );
    assert_eq!(
        overflow.termination(),
        Some(HarnessExecutionTermination::OutputLimitExceeded)
    );
    assert_eq!(overflow.observed_version(), None);
    Ok(())
}

#[test]
fn availability_uses_exact_normalized_version_records() -> Result<()> {
    let temp = tempfile::TempDir::new()?;
    let root = RepoRoot::try_from(temp.path())?;
    let main = child_command("child_entry_outputs")?;

    let exact = spec_with_probe(
        main.clone(),
        child_command("child_entry_probe_version")?,
        HarnessProbeOutput::Stderr,
        "1.2.3",
    )?;
    let exact_result = probe_allowlisted_tool(&request(root.clone(), main.clone(), None)?, &exact)?;
    assert_eq!(
        exact_result.availability(),
        HarnessToolAvailability::Available,
        "observed={:?} termination={:?}",
        exact_result
            .observed_version()
            .map(HarnessStepVersion::as_str),
        exact_result.termination()
    );
    assert_eq!(
        exact_result
            .observed_version()
            .map(HarnessStepVersion::as_str),
        Some("1.2.3")
    );

    let mismatch = spec_with_probe(
        main.clone(),
        child_command("child_entry_probe_version")?,
        HarnessProbeOutput::Stderr,
        "1.2.4",
    )?;
    let mismatch_result =
        probe_allowlisted_tool(&request(root.clone(), main.clone(), None)?, &mismatch)?;
    assert_eq!(
        mismatch_result.availability(),
        HarnessToolAvailability::VersionMismatch
    );
    assert_eq!(
        mismatch_result
            .observed_version()
            .map(HarnessStepVersion::as_str),
        Some("1.2.3")
    );

    let normalized = spec_with_probe(
        main,
        child_command("child_entry_probe_version_crlf")?,
        HarnessProbeOutput::Stderr,
        "1.2.3",
    )?;
    let normalized_result = probe_allowlisted_tool(
        &request(root, child_command("child_entry_outputs")?, None)?,
        &normalized,
    )?;
    assert_eq!(
        normalized_result.availability(),
        HarnessToolAvailability::Available
    );
    Ok(())
}

#[test]
fn availability_rejects_empty_control_and_multi_record_output() -> Result<()> {
    let temp = tempfile::TempDir::new()?;
    let root = RepoRoot::try_from(temp.path())?;
    let main = child_command("child_entry_outputs")?;
    for child_name in [
        "child_entry_probe_empty",
        "child_entry_probe_multiline",
        "child_entry_probe_control",
    ] {
        let reviewed = spec_with_probe(
            main.clone(),
            child_command(child_name)?,
            HarnessProbeOutput::Stderr,
            "1.2.3",
        )?;
        let result =
            probe_allowlisted_tool(&request(root.clone(), main.clone(), None)?, &reviewed)?;
        assert_eq!(
            result.availability(),
            HarnessToolAvailability::MalformedOutput,
            "{child_name} must be rejected as malformed output"
        );
        assert_eq!(
            result.termination(),
            Some(HarnessExecutionTermination::Completed)
        );
        assert_eq!(result.observed_version(), None);
    }
    Ok(())
}

#[test]
fn availability_selects_one_typed_stream_without_fallback() -> Result<()> {
    let temp = tempfile::TempDir::new()?;
    let root = RepoRoot::try_from(temp.path())?;
    let main = child_command("child_entry_outputs")?;
    let stderr = spec_with_probe(
        main.clone(),
        child_command("child_entry_probe_both_streams")?,
        HarnessProbeOutput::Stderr,
        "9.9.9",
    )?;
    let stderr_result =
        probe_allowlisted_tool(&request(root.clone(), main.clone(), None)?, &stderr)?;
    assert_eq!(
        stderr_result.availability(),
        HarnessToolAvailability::Available
    );

    let no_fallback = spec_with_probe(
        main,
        child_command("child_entry_probe_both_streams")?,
        HarnessProbeOutput::Stdout,
        "9.9.9",
    )?;
    let no_fallback_result = probe_allowlisted_tool(
        &request(root, child_command("child_entry_outputs")?, None)?,
        &no_fallback,
    )?;
    assert_eq!(
        no_fallback_result.availability(),
        HarnessToolAvailability::MalformedOutput
    );
    assert_eq!(no_fallback_result.observed_version(), None);
    Ok(())
}

#[test]
fn availability_reads_a_clean_reviewed_stdout_record() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let root = RepoRoot::try_from(current_dir.as_path())?;
    let git_command = vec![
        HarnessCommandArgument::try_new("git".to_owned())?,
        HarnessCommandArgument::try_new("config".to_owned())?,
        HarnessCommandArgument::try_new("--get".to_owned())?,
        HarnessCommandArgument::try_new("core.repositoryformatversion".to_owned())?,
    ];
    let reviewed = spec_with_probe_for_tool(ProbeSpec {
        tool: HarnessToolName::try_new("git".to_owned())?,
        main_command: git_command.clone(),
        probe_command: git_command.clone(),
        output: HarnessProbeOutput::Stdout,
        expected_version: "0",
        max_wall_time_ms: 2_000,
        max_output_bytes: 1_024,
    })?;
    let result = probe_allowlisted_tool(
        &request_with_tool(
            root.clone(),
            git_command,
            None,
            HarnessToolName::try_new("git".to_owned())?,
        )?,
        &reviewed,
    )?;
    assert_eq!(result.availability(), HarnessToolAvailability::Available);
    assert_eq!(
        result.observed_version().map(HarnessStepVersion::as_str),
        Some("0")
    );

    let unrelated_probe = vec![HarnessCommandArgument::try_new("git".to_owned())?];
    let unrelated = spec_with_probe_for_tool(ProbeSpec {
        tool: HarnessToolName::try_new("git".to_owned())?,
        main_command: child_command("child_entry_outputs")?,
        probe_command: unrelated_probe,
        output: HarnessProbeOutput::Stdout,
        expected_version: "unrelated",
        max_wall_time_ms: 2_000,
        max_output_bytes: 1_024,
    })?;
    let unrelated_result = probe_allowlisted_tool(
        &request_with_tool(
            root,
            child_command("child_entry_outputs")?,
            None,
            HarnessToolName::try_new("git".to_owned())?,
        )?,
        &unrelated,
    )?;
    assert_eq!(
        unrelated_result.availability(),
        HarnessToolAvailability::Misconfigured
    );
    assert_eq!(unrelated_result.termination(), None);
    assert_eq!(unrelated_result.observed_version(), None);
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
            root.clone(),
            reviewed.command().to_vec(),
            Some(r"C:outside".to_owned()),
        )?;
        assert_rejection(
            validate_allowlisted_request(&drive_relative, &reviewed),
            "repository root",
        )?;
    }

    let absolute = request(
        root,
        command,
        Some(temp.path().to_string_lossy().into_owned()),
    )?;
    assert_rejection(
        validate_allowlisted_request(&absolute, &reviewed),
        "repository root",
    )?;
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
    let outcome = execute_allowlisted_bounded(&request(root.clone(), command, None)?, &reviewed)?;
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
        use std::io::Write;
        let _ = std::io::stdout().write_all(b"bounded-stdout");
        let _ = std::io::stderr().write_all(b"bounded-stderr");
    }
}

#[test]
fn child_entry_probe_version() {
    if child_mode() {
        use std::io::Write;

        let mut stderr = std::io::stderr();
        let _ = stderr.write_all(b"1.2.3");
        let _ = stderr.flush();
        std::process::exit(0);
    }
}

#[test]
fn child_entry_probe_version_crlf() {
    if child_mode() {
        use std::io::Write;

        let mut stderr = std::io::stderr();
        let _ = stderr.write_all(b"  1.2.3\r\n");
        let _ = stderr.flush();
        std::process::exit(0);
    }
}

#[test]
fn child_entry_probe_empty() {
    if child_mode() {
        std::process::exit(0);
    }
}

#[test]
fn child_entry_probe_multiline() {
    if child_mode() {
        use std::io::Write;

        let mut stderr = std::io::stderr();
        let _ = stderr.write_all(b"1.2.3\nextra");
        let _ = stderr.flush();
        std::process::exit(0);
    }
}

#[test]
fn child_entry_probe_control() {
    if child_mode() {
        use std::io::Write;

        let mut stderr = std::io::stderr();
        let _ = stderr.write_all(b"1.2.3\t");
        let _ = stderr.flush();
        std::process::exit(0);
    }
}

#[test]
fn child_entry_probe_both_streams() {
    if child_mode() {
        use std::io::Write;

        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(b"1.2.3");
        let _ = stdout.flush();
        let mut stderr = std::io::stderr();
        let _ = stderr.write_all(b"9.9.9");
        let _ = stderr.flush();
        std::process::exit(0);
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
        use std::io::Write;
        let literal = std::env::args().any(|value| value == "&|$()");
        let _ = std::io::stdout().write_all(format!("literal={literal}").as_bytes());
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
