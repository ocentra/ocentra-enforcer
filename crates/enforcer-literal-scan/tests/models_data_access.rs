use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use enforcer_domain::scan_types::LiteralRiskCategory as RiskCategory;
use enforcer_literal_scan::{run_scan, CliOptions};

fn test_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root = std::env::temp_dir().join(format!("literal_scan_models_{nanos}"));
    fs::create_dir_all(&root)?;
    Ok(root)
}

fn json_string_field<'a>(
    line: &'a str,
    field: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    let prefix = format!("\"{field}\":\"");
    let after_field = line
        .split_once(&prefix)
        .map(|(_, value)| value)
        .ok_or_else(|| format!("expected JSON field {field:?} in {line}"))?;
    after_field
        .split_once('"')
        .map(|(value, _)| value)
        .ok_or_else(|| format!("expected a closing quote for {field:?}"))
        .map_err(Into::into)
}

#[test]
fn model_labels_remain_stable_for_a_secret_finding() -> Result<(), Box<dyn std::error::Error>> {
    let root = test_root()?;
    let token_prefix = "sk-proj-";
    let token = format!("{token_prefix}abcdefghijklmnopqrstuvwxyz123456");
    fs::write(
        root.join("secret.ts"),
        format!("export const token = \"{token}\";\n"),
    )?;
    let opts = CliOptions {
        root: root.clone().into(),
        include_low: true.into(),
        min_score: enforcer_domain::scan_types::LiteralRiskScore::ZERO,
        ..CliOptions::default()
    };

    let report = run_scan(&opts)?;
    let finding = report
        .hard_findings
        .iter()
        .find(|finding| finding.category == RiskCategory::SecretLike)
        .ok_or("expected the secret fixture to produce a hard finding")?;
    assert_eq!(finding.rule_id.as_str(), "SEC-2.10");

    let json_line = report
        .to_json_lines()
        .into_iter()
        .find(|line| line.as_str().contains("\"ruleId\":\"SEC-2.10\""))
        .ok_or("expected the secret finding in JSON-lines output")?;
    assert_eq!(
        json_string_field(json_line.as_str(), "fileRole")?,
        "unknown"
    );
    assert_eq!(
        json_string_field(json_line.as_str(), "literalKind")?,
        "import-specifier"
    );
    assert_eq!(
        json_string_field(json_line.as_str(), "category")?,
        "secret-like"
    );

    fs::remove_dir_all(root)?;
    Ok(())
}
