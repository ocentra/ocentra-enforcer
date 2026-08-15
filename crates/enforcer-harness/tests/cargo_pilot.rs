use std::fs;
use std::path::Path;

use enforcer_core::error::{Error, Result};
use enforcer_domain::boundary::hash::validate;
use enforcer_domain::harness_types::{
    HarnessCommandArgument, HarnessExecutionLimits, HarnessExecutionTermination,
    HarnessInputLimits, HarnessProbeOutput, HarnessRunId, HarnessRunStatus, HarnessStepVersion,
    HarnessToolAvailability, HarnessToolName, HarnessToolProbe, HarnessToolRequirement,
    HarnessToolSpec,
};
use enforcer_domain::paths::RepoRoot;
use enforcer_harness::adapters::cargo::{
    reviewed_input_tree_digest, run_cargo_pilot, CargoPilotInput,
};
use enforcer_harness::execution::ExecuteRequest;
use enforcer_harness::input_scope::compute_input_tree;

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
    request_with_cwd(root, command, Some("fixture".to_owned()))
}

fn request_with_cwd(
    root: RepoRoot,
    command: Vec<HarnessCommandArgument>,
    cwd: Option<String>,
) -> Result<ExecuteRequest> {
    Ok(ExecuteRequest {
        repo_root: root,
        cwd,
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
    let computed = reviewed_input_tree_digest(request, spec)?;
    CargoPilotInput::try_new(request, spec, computed)
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

#[cfg(windows)]
fn make_file_symlink(source: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(source, link)
}

#[cfg(not(windows))]
fn make_file_symlink(source: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, link)
}

#[test]
fn clean_cargo_pilot_is_available_passed_and_in_memory() -> Result<()> {
    let (temp, root) = fixture()?;
    let spec = spec("target", EXPECTED_CARGO_VERSION)?;
    let request = request(root, spec.command().to_vec())?;
    let evidence = run_cargo_pilot(&input(&request, &spec)?)?;

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
    assert_eq!(evidence.computed_input_file_count(), 3);
    assert_eq!(
        evidence.computed_input_tree_provenance().as_str(),
        "computed"
    );
    assert!(evidence.declared_input_tree_matches());
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
    let evidence = run_cargo_pilot(&input(&request, &spec)?)?;

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
        vec![
            r"\outside",
            r"C:outside",
            r"C:\outside",
            r"..\outside",
            "SRC",
            "Cargo.TOML",
        ]
    } else {
        vec!["/outside", "../outside"]
    } {
        let invalid = spec(target, expected)?;
        let invalid_request = request(root.clone(), invalid.command().to_vec())?;
        let expected_error = if cfg!(windows) && (target == "SRC" || target == "Cargo.TOML") {
            "overlaps"
        } else {
            "repository-relative"
        };
        assert_invalid_config(
            CargoPilotInput::try_new(&invalid_request, &invalid, validate(b"tree")),
            expected_error,
        )?;
    }

    let current_directory = spec(".", expected)?;
    let current_directory_request = request(root.clone(), current_directory.command().to_vec())?;
    assert_invalid_config(
        CargoPilotInput::try_new(
            &current_directory_request,
            &current_directory,
            validate(b"tree"),
        ),
        "current directory",
    )?;

    let metachar_spec = spec("target-&$", expected)?;
    let metachar_request = request(root, metachar_spec.command().to_vec())?;
    let metachar = run_cargo_pilot(&input(&metachar_request, &metachar_spec)?)?;
    assert_eq!(metachar.status(), HarnessRunStatus::Passed);
    let _ = exact_request;
    Ok(())
}

#[test]
fn existing_target_file_is_rejected_before_child() -> Result<()> {
    let (temp, root) = fixture()?;
    fs::write(
        temp.path().join("fixture/not-a-directory"),
        b"not a directory",
    )
    .map_err(|error| Error::InvalidConfig(format!("target file fixture: {error}")))?;
    let reviewed_spec = spec("not-a-directory", EXPECTED_CARGO_VERSION)?;
    let reviewed_request = request(root, reviewed_spec.command().to_vec())?;
    assert_invalid_config(
        CargoPilotInput::try_new(&reviewed_request, &reviewed_spec, validate(b"tree")),
        "must be a directory",
    )?;
    Ok(())
}

#[test]
fn repository_root_cwd_accepts_nonexistent_contained_target() -> Result<()> {
    let (_temp, root) = fixture()?;
    let root_path = Path::new(root.as_str());
    fs::create_dir_all(root_path.join("src"))
        .map_err(|error| Error::InvalidConfig(format!("root cwd source directory: {error}")))?;
    fs::write(root_path.join("Cargo.toml"), FIXTURE_TOML)
        .map_err(|error| Error::InvalidConfig(format!("root cwd manifest: {error}")))?;
    fs::write(root_path.join("Cargo.lock"), FIXTURE_LOCK)
        .map_err(|error| Error::InvalidConfig(format!("root cwd lock: {error}")))?;
    fs::write(root_path.join("src/lib.rs"), FIXTURE_LIB)
        .map_err(|error| Error::InvalidConfig(format!("root cwd source: {error}")))?;
    let reviewed_spec = spec("target", EXPECTED_CARGO_VERSION)?;
    let reviewed_request = request_with_cwd(root, reviewed_spec.command().to_vec(), None)?;
    let computed = reviewed_input_tree_digest(&reviewed_request, &reviewed_spec)?;
    let _input = CargoPilotInput::try_new(&reviewed_request, &reviewed_spec, computed.clone())?;
    assert_eq!(
        computed,
        reviewed_input_tree_digest(&reviewed_request, &reviewed_spec)?
    );
    Ok(())
}

#[test]
fn input_scope_digest_includes_the_selected_package_build_script() -> Result<()> {
    let (temp, root) = fixture()?;
    let reviewed_spec = spec("target", EXPECTED_CARGO_VERSION)?;
    let reviewed_request = request(root, reviewed_spec.command().to_vec())?;
    let target = arg("target")?;
    let limits = HarnessInputLimits::try_new(100, 20, 1_048_576, 8_388_608)?;

    let without_build_script = compute_input_tree(&reviewed_request, &target, limits)?;
    std::fs::write(
        temp.path().join("fixture/build.rs"),
        "fn main() { println!(\"cargo:rerun-if-changed=build.rs\"); }\n",
    )?;
    let with_build_script = compute_input_tree(&reviewed_request, &target, limits)?;

    assert_eq!(
        with_build_script.file_count(),
        without_build_script.file_count() + 1
    );
    assert_ne!(with_build_script.digest(), without_build_script.digest());
    Ok(())
}

#[cfg(windows)]
#[test]
fn non_ascii_reviewed_input_path_is_rejected_on_windows() -> Result<()> {
    let (_temp, root) = fixture()?;
    let reviewed_spec = spec("target", EXPECTED_CARGO_VERSION)?;
    let reviewed_request = request(root.clone(), reviewed_spec.command().to_vec())?;
    fs::write(
        Path::new(root.as_str()).join("fixture/src/é.rs"),
        b"pub fn non_ascii() {}",
    )
    .map_err(|error| Error::InvalidConfig(format!("non-ASCII input fixture: {error}")))?;
    assert_invalid_config(
        CargoPilotInput::try_new(&reviewed_request, &reviewed_spec, validate(b"tree")),
        "non-ASCII Windows paths",
    )?;
    Ok(())
}

#[test]
fn input_scope_bounded_read_rejects_file_grown_past_limit_before_read() -> Result<()> {
    let (temp, root) = fixture()?;
    let reviewed_spec = spec("target", EXPECTED_CARGO_VERSION)?;
    let reviewed_request = request(root, reviewed_spec.command().to_vec())?;
    fs::write(temp.path().join("fixture/src/lib.rs"), b"0123456789")
        .map_err(|error| Error::InvalidConfig(format!("grown input fixture: {error}")))?;
    let limits = HarnessInputLimits::try_new(3, 4, 8, 32)?;
    let target = arg("target")?;
    assert_invalid_config(
        compute_input_tree(&reviewed_request, &target, limits),
        "per-file byte bound exceeded",
    )?;
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
fn input_scope_rejects_file_symlink_or_reparse_entry() -> Result<()> {
    let (temp, root) = fixture()?;
    let outside = tempfile::TempDir::new()
        .map_err(|error| Error::InvalidConfig(format!("outside input fixture: {error}")))?;
    let outside_file = outside.path().join("outside.rs");
    fs::write(&outside_file, b"pub fn outside() {}")
        .map_err(|error| Error::InvalidConfig(format!("outside input file: {error}")))?;
    let link = temp.path().join("fixture/src/outside.rs");
    make_file_symlink(&outside_file, &link)
        .map_err(|error| Error::InvalidConfig(format!("input file symlink fixture: {error}")))?;
    let reviewed_spec = spec("target", EXPECTED_CARGO_VERSION)?;
    let reviewed_request = request(root, reviewed_spec.command().to_vec())?;
    assert_invalid_config(
        CargoPilotInput::try_new(&reviewed_request, &reviewed_spec, validate(b"tree")),
        "symlink or reparse",
    )?;
    Ok(())
}

#[test]
fn input_scope_rejects_directory_symlink_or_reparse_entry() -> Result<()> {
    let (temp, root) = fixture()?;
    let outside = tempfile::TempDir::new()
        .map_err(|error| Error::InvalidConfig(format!("outside input fixture: {error}")))?;
    fs::create_dir_all(outside.path().join("nested"))
        .map_err(|error| Error::InvalidConfig(format!("outside input directory: {error}")))?;
    let link = temp.path().join("fixture/src/nested");
    make_directory_symlink(outside.path(), &link).map_err(|error| {
        Error::InvalidConfig(format!("input directory symlink fixture: {error}"))
    })?;
    let reviewed_spec = spec("target", EXPECTED_CARGO_VERSION)?;
    let reviewed_request = request(root, reviewed_spec.command().to_vec())?;
    assert_invalid_config(
        CargoPilotInput::try_new(&reviewed_request, &reviewed_spec, validate(b"tree")),
        "symlink or reparse",
    )?;
    Ok(())
}

#[test]
fn input_scope_excludes_target_contents_and_rejects_target_overlap() -> Result<()> {
    let (temp, root) = fixture()?;
    fs::create_dir_all(temp.path().join("fixture/target/debug"))
        .map_err(|error| Error::InvalidConfig(format!("target directory: {error}")))?;
    fs::write(
        temp.path().join("fixture/target/debug/generated"),
        b"generated",
    )
    .map_err(|error| Error::InvalidConfig(format!("target output: {error}")))?;
    let reviewed_spec = spec("target", EXPECTED_CARGO_VERSION)?;
    let reviewed_request = request(root.clone(), reviewed_spec.command().to_vec())?;
    let first = reviewed_input_tree_digest(&reviewed_request, &reviewed_spec)?;
    fs::write(
        temp.path().join("fixture/target/debug/generated"),
        b"changed",
    )
    .map_err(|error| Error::InvalidConfig(format!("target output mutation: {error}")))?;
    let second = reviewed_input_tree_digest(&reviewed_request, &reviewed_spec)?;
    assert_eq!(first, second);

    let overlap = spec("src", EXPECTED_CARGO_VERSION)?;
    let overlap_request = request(root, overlap.command().to_vec())?;
    assert_invalid_config(
        CargoPilotInput::try_new(&overlap_request, &overlap, validate(b"tree")),
        "overlaps",
    )?;
    Ok(())
}

#[test]
fn declared_and_computed_input_digests_must_match_before_child() -> Result<()> {
    let (temp, root) = fixture()?;
    let reviewed_spec = spec("target", EXPECTED_CARGO_VERSION)?;
    let reviewed_request = request(root, reviewed_spec.command().to_vec())?;
    let mismatched = CargoPilotInput::try_new(
        &reviewed_request,
        &reviewed_spec,
        validate(b"caller-declared-different"),
    )?;
    let evidence = run_cargo_pilot(&mismatched)?;
    assert_eq!(
        evidence.availability(),
        HarnessToolAvailability::Misconfigured
    );
    assert_eq!(evidence.status(), HarnessRunStatus::Failed);
    assert!(!evidence.declared_input_tree_matches());
    assert_eq!(evidence.execution_termination(), None);
    assert!(!temp.path().join("fixture/target").exists());
    Ok(())
}

#[test]
fn input_scope_rejects_file_count_overflow() -> Result<()> {
    let (_temp, root) = fixture()?;
    for (index, _) in std::iter::repeat_n((), 101).enumerate() {
        let path = Path::new(root.as_str()).join(format!("fixture/src/generated-{index}.rs"));
        fs::write(path, b"pub fn generated() {}")
            .map_err(|error| Error::InvalidConfig(format!("file-count fixture: {error}")))?;
    }
    let reviewed_spec = spec("target", EXPECTED_CARGO_VERSION)?;
    let reviewed_request = request(root, reviewed_spec.command().to_vec())?;
    assert_invalid_config(
        CargoPilotInput::try_new(&reviewed_request, &reviewed_spec, validate(b"tree")),
        "file-count bound",
    )?;
    Ok(())
}

#[test]
fn input_scope_rejects_depth_overflow() -> Result<()> {
    let (_temp, root) = fixture()?;
    let mut relative = String::from("fixture/src");
    for (index, _) in std::iter::repeat_n((), 17).enumerate() {
        relative.push_str(&format!("/nested-{index}"));
        fs::create_dir_all(Path::new(root.as_str()).join(&relative))
            .map_err(|error| Error::InvalidConfig(format!("depth fixture: {error}")))?;
    }
    fs::write(
        Path::new(root.as_str()).join(&relative).join("deep.rs"),
        b"deep",
    )
    .map_err(|error| Error::InvalidConfig(format!("depth file fixture: {error}")))?;
    let reviewed_spec = spec("target", EXPECTED_CARGO_VERSION)?;
    let reviewed_request = request(root, reviewed_spec.command().to_vec())?;
    assert_invalid_config(
        CargoPilotInput::try_new(&reviewed_request, &reviewed_spec, validate(b"tree")),
        "recursion depth",
    )?;
    Ok(())
}

#[test]
fn input_scope_rejects_per_file_and_total_byte_overflow() -> Result<()> {
    let (_temp, root) = fixture()?;
    let large = vec![b'x'; 1_048_577];
    fs::write(
        Path::new(root.as_str()).join("fixture/src/too-large.rs"),
        &large,
    )
    .map_err(|error| Error::InvalidConfig(format!("per-file fixture: {error}")))?;
    let reviewed_spec = spec("target", EXPECTED_CARGO_VERSION)?;
    let reviewed_request = request(root, reviewed_spec.command().to_vec())?;
    assert_invalid_config(
        CargoPilotInput::try_new(&reviewed_request, &reviewed_spec, validate(b"tree")),
        "per-file byte bound",
    )?;

    let (_total_temp, total_root) = fixture()?;
    let chunk = vec![b'y'; 1_048_576];
    for (index, _) in std::iter::repeat_n((), 8).enumerate() {
        fs::write(
            Path::new(total_root.as_str()).join(format!("fixture/src/chunk-{index}.rs")),
            &chunk,
        )
        .map_err(|error| Error::InvalidConfig(format!("total-byte fixture: {error}")))?;
    }
    let total_spec = spec("target", EXPECTED_CARGO_VERSION)?;
    let total_request = request(total_root, total_spec.command().to_vec())?;
    assert_invalid_config(
        CargoPilotInput::try_new(&total_request, &total_spec, validate(b"tree")),
        "total-byte bound",
    )?;
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
    let probe_request = request(root.clone(), main)?;
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
    let first = run_cargo_pilot(&input(&reviewed_request, &reviewed_spec)?)?;
    let second = run_cargo_pilot(&input(&reviewed_request, &reviewed_spec)?)?;
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
        run_cargo_pilot(&drift_input),
        "scope changed before availability probe",
    )?;
    assert!(!drift_temp.path().join("fixture/target").exists());
    Ok(())
}
