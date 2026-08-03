// source owner: crates/enforcer-domain/src/harness_types.rs
// generator: cargo test -p enforcer-harness --test tool_adapter_contract
// contractHash: 2ccae7474073653d7be42bbd3903bac8c1c818c0b3ddf70c7350381e2837299a

use enforcer_core::error::{Error, Result};
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::config_types::HarnessConfig;
use enforcer_domain::harness_types::{
    HarnessCommandArgument, HarnessExecutionLimits, HarnessToolAvailability, HarnessToolDecision,
    HarnessToolName, HarnessToolRequirement, HarnessToolSpec,
};
use enforcer_domain::paths::RepoRoot;
use enforcer_harness::execution::{validate_allowlisted_request, ExecuteRequest};

fn spec() -> Result<HarnessToolSpec> {
    HarnessToolSpec::try_new(
        HarnessToolName::try_new("cargo".to_owned())?,
        vec![
            HarnessCommandArgument::try_new("cargo".to_owned())?,
            HarnessCommandArgument::try_new("check".to_owned())?,
        ],
        HarnessToolRequirement::Required,
        HarnessExecutionLimits::try_new(10_000, 1_048_576, 100)?,
        None,
    )
    .map_err(Into::into)
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

fn assert_rejection(result: Result<()>, expected: &str) -> Result<()> {
    let error = match result {
        Ok(()) => {
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
fn arbitrary_execute_remains_a_distinct_user_invoked_surface() {
    let _ = HarnessConfig::default();
    let function_name = enforcer_harness::execution::execute
        as fn(
            &ExecuteRequest,
            &HarnessConfig,
        ) -> enforcer_core::error::Result<enforcer_harness::storage::RunOutcome>;
    let _ = function_name;
}
