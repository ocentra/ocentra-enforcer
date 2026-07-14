use enforcer_install::error::InstallError;
use enforcer_install::migrate_legacy_name::{migrate, ConfigFormat, ConfigTarget, LEGACY_SERVER_NAME};
use enforcer_mcp::name::SERVER_NAME;

#[test]
fn migration_refuses_a_dual_server_registration_without_writing() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join(".claude.json");
    let original = serde_json::to_string_pretty(&serde_json::json!({
        "mcpServers": {
            LEGACY_SERVER_NAME: { "command": "/legacy/enforcer" },
            SERVER_NAME: { "command": "/current/enforcer" }
        }
    }))?;
    std::fs::write(&path, &original)?;
    let target = ConfigTarget::new("claude", &path, ConfigFormat::JsonMcpServers);

    let result = migrate(&[target], None);
    assert!(matches!(result, Err(InstallError::MalformedConfig { reason, .. }) if reason.contains("refuses to overwrite")));
    assert_eq!(std::fs::read_to_string(&path)?, original);
    assert!(std::fs::read_dir(dir.path())?.all(|entry| {
        entry.map(|entry| !entry.file_name().to_string_lossy().contains(".enforcer-bak.")).unwrap_or(false)
    }));
    Ok(())
}
