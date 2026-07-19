use enforcer_domain::install_types::{
    CheckStatus, InstallReportText, InstallRootPath, OverwriteMode,
};
use enforcer_install::ci::project_template::{
    install, render, verify, ProjectCiCommand, ProjectCiCommands, ProjectKind,
};

fn root(path: &std::path::Path) -> Result<InstallRootPath, Box<dyn std::error::Error>> {
    Ok(InstallRootPath::try_from(path.to_path_buf())?)
}

#[test]
fn happy_path_is_deterministic_and_uses_only_the_supported_ci_contract(
) -> Result<(), Box<dyn std::error::Error>> {
    let commands = ProjectCiCommands::for_kind(ProjectKind::Hybrid)?;
    let first = render(&commands)?;
    assert_eq!(first, render(&commands)?);
    let workflow = first.content().as_str();
    for expected in [
        "enforcer memory cli --json index_repository --repo-path . --stores-dir \"$RUNNER_TEMP/enforcer-memory-store\" --mode fast",
        "enforcer scan --all",
        "enforcer verify --mode ci --all",
        "Install pinned Enforcer with checksum verification",
        "ENFORCER_VERSION: \"0.1.0\"",
        "install.sh\" -o \"$RUNNER_TEMP/install-enforcer.sh\"",
        "actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5",
        "actions/cache@0057852bfaa89a56745cba8c7296529d2fc39830",
        "BLOCKED: memory CLI lacks a typed zero-result assertion for graph exclusion proof",
    ] { assert!(workflow.contains(expected), "missing {expected}"); }
    for forbidden in [
        "codebase-memory-mcp",
        "--root",
        "enforcer scan --workspace",
        "enforcer verify --profile",
        "enforcer check",
        "/main/install.sh",
        "@v4",
    ] {
        assert!(
            !workflow.contains(forbidden),
            "obsolete or mutable form present: {forbidden}"
        );
    }
    Ok(())
}

#[test]
fn commands_reject_newlines_controls_and_shell_or_yaml_injection(
) -> Result<(), Box<dyn std::error::Error>> {
    for rejected_value in [
        "npm run ci\n  evil: true",
        "npm run ci\r\necho evil",
        "npm run ci\u{7f}",
        "npm run ci; rm -rf /",
        "npm run ci | tee leak",
        "npm run ci # yaml",
    ] {
        assert!(
            ProjectCiCommand::try_from(InstallReportText::try_from(rejected_value.to_owned())?)
                .is_err(),
            "accepted rejected command: {rejected_value:?}"
        );
    }
    Ok(())
}

#[test]
fn custom_commands_render_as_literal_block_content() -> Result<(), Box<dyn std::error::Error>> {
    let commands = ProjectCiCommands::new(
        ProjectCiCommand::try_from(InstallReportText::try_from("just ci-local".to_owned())?)?,
        Some(ProjectCiCommand::try_from(InstallReportText::try_from(
            "just release-rehearsal".to_owned(),
        )?)?),
    );
    let workflow = render(&commands)?;
    assert!(workflow
        .content()
        .as_str()
        .contains("run: |\n          just ci-local"));
    assert!(workflow
        .content()
        .as_str()
        .contains("run: |\n          just release-rehearsal"));
    Ok(())
}

#[test]
fn install_preserves_then_force_repairs_drift() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let commands = ProjectCiCommands::for_kind(ProjectKind::Rust)?;
    let install_root = root(directory.path())?;
    install(&install_root, &commands, OverwriteMode::PreserveExisting)?;
    let path = directory
        .path()
        .join(".github/workflows/ocentra-enforcer-ci.yml");
    let first = std::fs::read_to_string(&path)?;
    std::fs::write(&path, "name: drifted\n")?;
    install(&install_root, &commands, OverwriteMode::PreserveExisting)?;
    assert_eq!(std::fs::read_to_string(&path)?, "name: drifted\n");
    assert!(matches!(
        verify(&install_root, &commands)?.status,
        CheckStatus::Failed
    ));
    install(&install_root, &commands, OverwriteMode::Force)?;
    assert_eq!(std::fs::read_to_string(&path)?, first);
    Ok(())
}
