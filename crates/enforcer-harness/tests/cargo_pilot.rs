use std::fs;
use std::path::Path;

use enforcer_core::error::{Error, Result};
use enforcer_domain::boundary::hash::validate;
use enforcer_domain::harness_types::{
    HarnessCommandArgument, HarnessExecutionLimits, HarnessExecutionTermination,
    HarnessProbeOutput, HarnessRunId, HarnessRunStatus, HarnessStepVersion,
    HarnessToolAvailability, HarnessToolName, HarnessToolProbe, HarnessToolRequirement,
    HarnessToolSpec,
};
use enforcer_domain::paths::RepoRoot;
use enforcer_harness::adapters::cargo::{run_cargo_pilot, CargoPilotInput};
use enforcer_harness::execution::ExecuteRequest;

const EXPECTED_CARGO_VERSION: &str = "cargo 1.95.0 (f2d3ce0bd 2026-03-21)";
const FIXTURE_TOML: &[u8] = include_bytes!("fixtures/tool_adapters/cargo_pilot/Cargo.toml.fixture");
const FIXTURE_LOCK: &[u8] = include_bytes!("fixtures/tool_adapters/cargo_pilot/Cargo.lock");
const FIXTURE_LIB: &[u8] = include_bytes!("fixtures/tool_adapters/cargo_pilot/src/lib.rs");

fn arg(value: &str) -> Result<HarnessCommandArgument> {
    HarnessCommandArgument::try_new(value.to_owned()).map_err(Into::into)
}

fn command(target_dir: &str) -> Result<Vec<HarnessCommandArgument>> {
    [
        "cargo",
        "+1.95.0",
        "check",
        "--offline",
        "--locked",
        "--message-format=json",
        "--target-dir",
        target_dir,
    ]
    .into_iter()
    .map(arg)
    .collect()
}

fn probe_command() -> Result<Vec<HarnessCommandArgument>> {
    ["cargo", "+1.95.0", "--version"]
        .into_iter()
        .map(arg)
        .collect()
}

fn spec(target_dir: &str, expected: &str) -> Result<HarnessToolSpec> {
    let probe = HarnessToolProbe::try_new(probe_command()?, HarnessProbeOutput::Stdout)?;
    Ok(HarnessToolSpec::try_new(
        HarnessToolName::try_new("cargo".to_owned())?,
        command(target_dir)?,
        HarnessToolRequirement::Required,
        HarnessExecutionLimits::try_new(30_000, 1_048_576, 100)?,
        Some(HarnessStepVersion::try_new(expected.to_owned())?),
    )?
    .with_probe(probe))
}

fn request(root: RepoRoot, command: Vec<HarnessCommandArgument>) -> Result<ExecuteRequest> {
    Ok(ExecuteRequest {
        repo_root: root,
        cwd: Some("fixture".to_owned()),
        run_id: HarnessRunId::try_new("ul07-cargo-pilot".to_owned())?,
        tool: HarnessToolName::try_new("cargo".to_owned())?,
        language: None,
        command,
        crate_name: None,
        package_name: None,
        domain: None,
        tags: Vec::new(),
    })
}

fn fixture() -> Result<(tempfile::TempDir, RepoRoot)> {
    let temp = tempfile::TempDir::new()
        .map_err(|error| Error::InvalidConfig(format!("temporary fixture: {error}")))?;
    let root = temp.path().join("fixture");
    fs::create_dir_all(root.join("src"))
        .map_err(|error| Error::InvalidConfig(format!("fixture directories: {error}")))?;
    fs::write(root.join("Cargo.toml"), FIXTURE_TOML)
        .map_err(|error| Error::InvalidConfig(format!("fixture manifest: {error}")))?;
    fs::write(root.join("Cargo.lock"), FIXTURE_LOCK)
        .map_err(|error| Error::InvalidConfig(format!("fixture lock: {error}")))?;
    fs::write(root.join("src/lib.rs"), FIXTURE_LIB)
        .map_err(|error| Error::InvalidConfig(format!("fixture source: {error}")))?;
    let repo_root = RepoRoot::try_from(temp.path())?;
    Ok((temp, repo_root))
}

fn input<'a>(
    request: &'a ExecuteRequest,
    spec: &'a HarnessToolSpec,
) -> Result<CargoPilotInput<'a>> {
    CargoPilotInput::try_new(request, spec, validate(b"declared-tree")).map_err(Into::into)
}

fn assert_invalid_config<T>(result: Result<T>, expected: &str) -> Result<()> {
    let error = match result {
        Ok(_) => {
            return Err(Error::InvalidConfig(format!(
                "expected InvalidConfig containing {expected}"
            )))
        }
        Err(error) => error,
    };
    if !error.to_string().contains(expected) {
        return Err(Error::InvalidConfig(format!(
            "wrong rejection for {expected}: {error}"
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn make_directory_symlink(source: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(source, link)
}

#[cfg(not(windows))]
fn make_directory_symlink(source: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, link)
}

#[test]
fn clean_cargo_pilot_is_available_passed_and_in_memory() -> Result<()> {
    let (temp, root) = fixture()?;
    let spec = spec("target", EXPECTED_CARGO_VERSION)?;
    let request = request(root, spec.command().to_vec())?;
    let evidence = run_cargo_pilot(input(&request, &spec)?)?;

    assert_eq!(evidence.availability(), HarnessToolAvailability::Available);
    assert_eq!(
        evidence.probe_termination(),
        Some(HarnessExecutionTermination::Completed)
    );
    assert_eq!(
        evidence.execution_termination(),
        Some(HarnessExecutionTermination::Completed)
    );
    assert_eq!(evidence.status(), HarnessRunStatus::Passed);
    assert!(evidence
        .diagnostics()
        .iter()
        .all(|diagnostic| { diagnostic.severity != enforcer_domain::severity::Severity::Error }));
    assert!(!temp.path().join(".enforce").exists());
    Ok(())
}

#[test]
fn invalid_cargo_source_is_nonzero_with_rust_diagnostics() -> Result<()> {
    let (_temp, root) = fixture()?;
    let source = Path::new(root.as_str()).join("fixture/src/lib.rs");
    fs::write(source, b"pub fn broken( {")
        .map_err(|error| Error::InvalidConfig(format!("invalid fixture source: {error}")))?;
    let spec = spec("target", EXPECTED_CARGO_VERSION)?;
    let request = request(root, spec.command().to_vec())?;
    let evidence = run_cargo_pilot(input(&request, &spec)?)?;

    assert_eq!(evidence.availability(), HarnessToolAvailability::Available);
    assert_eq!(
        evidence.execution_termination(),
        Some(HarnessExecutionTermination::NonZeroExit)
    );
    assert_eq!(evidence.status(), HarnessRunStatus::Failed);
    assert!(evidence
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.severity == enforcer_domain::severity::Severity::Error));
    Ok(())
}

#[test]
fn exact_command_probe_and_target_contract_rejects_unsafe_variants() -> Result<()> {
    let (_temp, root) = fixture()?;
    let expected = EXPECTED_CARGO_VERSION;
    let exact = spec("target", expected)?;
    let exact_request = request(root.clone(), exact.command().to_vec())?;

    for rejected in [
        vec!["cargo", "+1.95.0", "publish"],
        vec!["cargo", "+1.95.0", "check", "--locked"],
        vec![
            "cargo",
            "+1.95.0",
            "check",
            "--locked",
            "--offline",
            "--message-format=json",
            "--target-dir",
            "target",
        ],
        vec![
            "cargo",
            "+1.95.0",
            "check",
            "--offline",
            "--locked",
            "--message-format=json",
            "--target-dir",
            "target",
            "--features",
            "unsafe-extra",
        ],
    ] {
        let rejected = rejected.into_iter().map(arg).collect::<Result<Vec<_>>>()?;
        let rejected_spec = HarnessToolSpec::try_new(
            HarnessToolName::try_new("cargo".to_owned())?,
            rejected.clone(),
            HarnessToolRequirement::Required,
            HarnessExecutionLimits::try_new(30_000, 1_048_576, 100)?,
            Some(HarnessStepVersion::try_new(expected.to_owned())?),
        )?;
        assert_invalid_config(
            CargoPilotInput::try_new(
                &request(root.clone(), rejected)?,
                &rejected_spec,
                validate(b"tree"),
            ),
            "exact offline locked check command",
        )?;
    }

    for target in if cfg!(windows) {
        vec![r"\outside", r"C:outside", r"C:\outside", r"..\outside"]
    } else {
        vec!["/outside", "../outside"]
    } {
        let invalid = spec(target, expected)?;
        let invalid_request = request(root.clone(), invalid.command().to_vec())?;
        assert_invalid_config(
            CargoPilotInput::try_new(&invalid_request, &invalid, validate(b"tree")),
            "repository-relative",
        )?;
    }

    let metachar_spec = spec("target-&$", expected)?;
    let metachar_request = request(root, metachar_spec.command().to_vec())?;
    let metachar = run_cargo_pilot(input(&metachar_request, &metachar_spec)?)?;
    assert_eq!(metachar.status(), HarnessRunStatus::Passed);
    let _ = exact_request;
    Ok(())
}

#[test]
fn target_symlink_or_reparse_escape_is_rejected_before_child() -> Result<()> {
    let (temp, root) = fixture()?;
    let outside = tempfile::TempDir::new()
        .map_err(|error| Error::InvalidConfig(format!("outside target fixture: {error}")))?;
    fs::create_dir_all(outside.path().join("build"))
        .map_err(|error| Error::InvalidConfig(format!("outside target directory: {error}")))?;
    let link = temp.path().join("fixture/target-link");
    make_directory_symlink(outside.path(), &link)
        .map_err(|error| Error::InvalidConfig(format!("target symlink fixture: {error}")))?;

    let reviewed_spec = spec("target-link/build", EXPECTED_CARGO_VERSION)?;
    let reviewed_request = request(root, reviewed_spec.command().to_vec())?;
    assert_invalid_config(
        CargoPilotInput::try_new(&reviewed_request, &reviewed_spec, validate(b"tree")),
        "remain below",
    )?;
    assert!(!outside.path().join("build/debug").exists());
    Ok(())
}

#[test]
fn unrelated_probe_executable_is_rejected_without_child() -> Result<()> {
    let (_temp, root) = fixture()?;
    let main = command("target")?;
    let unrelated = HarnessToolProbe::try_new(
        ["git", "--version"]
            .into_iter()
            .map(arg)
            .collect::<Result<_>>()?,
        HarnessProbeOutput::Stdout,
    )?;
    let wrong_stream = HarnessToolProbe::try_new(probe_command()?, HarnessProbeOutput::Stderr)?;
    let wrong_args = HarnessToolProbe::try_new(
        ["cargo", "+1.95.0", "version"]
            .into_iter()
            .map(arg)
            .collect::<Result<_>>()?,
        HarnessProbeOutput::Stdout,
    )?;
    for probe in [unrelated, wrong_stream, wrong_args] {
        let candidate = HarnessToolSpec::try_new(
            HarnessToolName::try_new("cargo".to_owned())?,
            main.clone(),
            HarnessToolRequirement::Required,
            HarnessExecutionLimits::try_new(30_000, 1_048_576, 100)?,
            Some(HarnessStepVersion::try_new(
                EXPECTED_CARGO_VERSION.to_owned(),
            )?),
        )?
        .with_probe(probe);
        let request = request(root.clone(), main.clone())?;
        assert_invalid_config(
            CargoPilotInput::try_new(&request, &candidate, validate(b"tree")),
            "exact pinned cargo stdout probe contract",
        )?;
    }

    let missing_version = HarnessToolSpec::try_new(
        HarnessToolName::try_new("cargo".to_owned())?,
        main.clone(),
        HarnessToolRequirement::Required,
        HarnessExecutionLimits::try_new(30_000, 1_048_576, 100)?,
        None,
    )?
    .with_probe(HarnessToolProbe::try_new(
        probe_command()?,
        HarnessProbeOutput::Stdout,
    )?);
    let probe_request = request(root.clone(), main.clone())?;
    assert_invalid_config(
        CargoPilotInput::try_new(&probe_request, &missing_version, validate(b"tree")),
        "exact pinned cargo stdout probe contract",
    )?;

    let mismatched_version = spec("target", "cargo 1.95.0 (wrong-record)")?;
    let mismatch_request = request(root, mismatched_version.command().to_vec())?;
    assert_invalid_config(
        CargoPilotInput::try_new(&mismatch_request, &mismatched_version, validate(b"tree")),
        "exact pinned cargo stdout probe contract",
    )?;
    Ok(())
}

#[test]
fn observed_version_mismatch_is_rejected_before_child() -> Result<()> {
    let (_temp, root) = fixture()?;
    let mismatched = spec("target", "cargo 1.95.0 (wrong-record)")?;
    let request = request(root, mismatched.command().to_vec())?;
    assert_invalid_config(
        CargoPilotInput::try_new(&request, &mismatched, validate(b"tree")),
        "exact pinned cargo stdout probe contract",
    )?;
    Ok(())
}

#[test]
fn captured_digests_are_deterministic_and_config_tamper_is_visible() -> Result<()> {
    let (_temp, root) = fixture()?;
    let reviewed_spec = spec("target", EXPECTED_CARGO_VERSION)?;
    let reviewed_request = request(root, reviewed_spec.command().to_vec())?;
    let first = run_cargo_pilot(input(&reviewed_request, &reviewed_spec)?)?;
    let second = run_cargo_pilot(input(&reviewed_request, &reviewed_spec)?)?;
    assert_eq!(first.command_digest(), second.command_digest());
    assert_eq!(first.config_digest(), second.config_digest());
    assert_eq!(first.status(), second.status());
    assert!(!first.captured_text_digest().as_str().is_empty());

    assert_ne!(
        first.declared_input_tree_digest(),
        &validate(b"different-declared-tree")
    );
    assert_eq!(
        first.input_tree_provenance().as_str(),
        "declared-unverified"
    );

    let (drift_temp, drift_root) = fixture()?;
    let drift_spec = spec("target", EXPECTED_CARGO_VERSION)?;
    let drift_request = request(drift_root, drift_spec.command().to_vec())?;
    let drift_input = input(&drift_request, &drift_spec)?;
    fs::write(
        drift_temp.path().join("fixture/Cargo.lock"),
        b"tampered-lock",
    )
    .map_err(|error| Error::InvalidConfig(format!("tamper fixture lock: {error}")))?;
    assert_invalid_config(
        run_cargo_pilot(drift_input),
        "changed after review and before execution",
    )?;
    assert!(!drift_temp.path().join("fixture/target").exists());
    Ok(())
}
