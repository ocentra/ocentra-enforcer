use enforcer_domain::install_types::{ConfigFormat, FindingKind, MigrationOutcome};
use enforcer_install::error::InstallError;
use enforcer_install::migrate_legacy_name::{
    migrate, ConfigTarget, MigrationFindingDto, MigrationOutcomeDto, RewrittenFileDto,
    LEGACY_SERVER_NAME,
};
use enforcer_domain::mcp_types::SERVER_NAME;

#[test]
fn migration_refuses_a_dual_server_registration_without_writing(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join(".claude.json");
    let original = serde_json::to_string_pretty(&serde_json::json!({
        "mcpServers": {
            LEGACY_SERVER_NAME: { "command": "/legacy/enforcer" },
            SERVER_NAME: { "command": "/current/enforcer" }
        }
    }))?;
    std::fs::write(&path, &original)?;
    let target = ConfigTarget::try_new(
        "claude".to_owned(),
        path.clone(),
        ConfigFormat::JsonMcpServers,
    )?;

    let result = migrate(&[target], None);
    assert!(
        matches!(result, Err(InstallError::MalformedConfig { reason, .. }) if reason.as_str().contains("refuses to overwrite"))
    );
    assert_eq!(std::fs::read_to_string(&path)?, original);
    assert!(std::fs::read_dir(dir.path())?.all(|entry| {
        entry
            .map(|entry| {
                !entry
                    .file_name()
                    .to_string_lossy()
                    .contains(".enforcer-bak.")
            })
            .unwrap_or(false)
    }));
    Ok(())
}

#[test]
fn migration_outcome_json_crosses_the_public_domain_boundary_in_both_directions(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let config_path = dir.path().join("config.toml");
    let backup_path = dir.path().join("config.toml.enforcer-bak");
    let skill_path = dir.path().join("legacy-skill");
    let valid = MigrationOutcomeDto {
        findings: vec![MigrationFindingDto {
            harness: "codex".to_owned(),
            path: config_path.display().to_string(),
            kind: FindingKind::LegacyServerRegistration,
            detail: "legacy server registration".to_owned(),
        }],
        rewritten: vec![RewrittenFileDto {
            path: config_path.display().to_string(),
            backup_path: backup_path.display().to_string(),
        }],
        retired_skill_dir: Some(skill_path.display().to_string()),
        notice: Some("migration complete".to_owned()),
    };

    let valid_wire = serde_json::to_string(&valid)?;
    let decoded: MigrationOutcomeDto = serde_json::from_str(&valid_wire)?;
    let domain = MigrationOutcome::try_from(decoded)?;
    let reencoded = serde_json::to_string(&MigrationOutcomeDto::from(domain))?;
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&reencoded)?,
        serde_json::from_str::<serde_json::Value>(&valid_wire)?
    );

    let invalid = MigrationOutcomeDto {
        findings: vec![MigrationFindingDto {
            harness: "codex".to_owned(),
            path: "relative/config.toml".to_owned(),
            kind: FindingKind::LegacyServerRegistration,
            detail: "legacy server registration".to_owned(),
        }],
        rewritten: Vec::new(),
        retired_skill_dir: None,
        notice: None,
    };
    let invalid_wire = serde_json::to_string(&invalid)?;
    let decoded_invalid: MigrationOutcomeDto = serde_json::from_str(&invalid_wire)?;
    assert_eq!(serde_json::to_string(&decoded_invalid)?, invalid_wire);
    assert!(matches!(
        MigrationOutcome::try_from(decoded_invalid),
        Err(InstallError::InvalidDomain(_))
    ));
    Ok(())
}
