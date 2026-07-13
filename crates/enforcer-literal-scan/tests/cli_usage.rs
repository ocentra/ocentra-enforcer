use std::process::Command;

#[test]
fn cli_usage_reports_help_and_invalid_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let binary = env!("CARGO_BIN_EXE_enforcer-literal-scan");

    let help = Command::new(binary)
        .arg("--help")
        .output()?;
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("Usage:"));

    let invalid = Command::new(binary)
        .arg("unknown-command")
        .output()?;
    assert_eq!(invalid.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid.stdout).contains("Usage:"));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("unknown argument"));
    Ok(())
}
