//! BOUNDARY-INVARIANT: CP09 API-security B02 validates supplied JSON records only;
//! no endpoint, token, gateway, key store, scanner, or production system is used.

use std::collections::BTreeSet;
use std::path::PathBuf;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_lang_security::rules::cyberskills::api_security_manifest::ApiSecurityManifestValidator;
use enforcer_validator::validator::{ValidationInput, Validator};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture(
    path: &str,
) -> Result<Vec<enforcer_domain::findings::Finding>, Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(manifest_dir().join(path))?;
    let file: RelPath = "api-security-manifest-b02.json".parse()?;
    let validator = ApiSecurityManifestValidator::new()?;
    Ok(validator.validate(ValidationInput {
        file: &file,
        source: enforcer_domain::boundary::validation::ValidationSource::from_text(&source),
        scope: ScanScope::Files,
    }))
}

#[test]
fn api_security_manifest_b02_pass_and_boundary_are_clean() -> Result<(), Box<dyn std::error::Error>>
{
    let pass = fixture("tests/fixtures/cyberskills/api-security-manifest-b02/pass.json")?;
    let boundary = fixture("tests/fixtures/cyberskills/api-security-manifest-b02/boundary.json")?;
    assert!(pass.is_empty(), "pass findings: {pass:?}");
    assert!(boundary.is_empty(), "boundary findings: {boundary:?}");
    Ok(())
}

#[test]
fn api_security_manifest_b02_fail_and_malformed_are_rejected(
) -> Result<(), Box<dyn std::error::Error>> {
    let failures = fixture("tests/fixtures/cyberskills/api-security-manifest-b02/fail.json")?;
    let malformed = fixture("tests/fixtures/cyberskills/api-security-manifest-b02/malformed.json")?;
    assert_eq!(failures.len(), 1);
    assert_eq!(malformed.len(), 1);
    assert!(failures
        .iter()
        .chain(malformed.iter())
        .all(|finding| finding.rule_id.as_str() == "CYBER-API-MANIFEST.1"));
    Ok(())
}

#[test]
fn api_security_manifest_b02_pass_covers_the_five_selected_skills(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(
        manifest_dir().join("tests/fixtures/cyberskills/api-security-manifest-b02/pass.json"),
    )?;
    let document: serde_json::Value = serde_json::from_str(&source)?;
    let ids: BTreeSet<&str> = document["records"]
        .as_array()
        .ok_or("pass fixture records must be an array")?
        .iter()
        .filter_map(|record| record["skillId"].as_str())
        .collect();
    assert_eq!(ids.len(), 5);
    assert_eq!(
        ids,
        BTreeSet::from([
            "exploiting-excessive-data-exposure-in-api",
            "exploiting-jwt-algorithm-confusion-attack",
            "implementing-api-abuse-detection-with-rate-limiting",
            "implementing-api-gateway-security-controls",
            "implementing-api-key-security-controls",
        ])
    );
    Ok(())
}
