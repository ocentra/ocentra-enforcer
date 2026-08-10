//! BOUNDARY-INVARIANT: CP09 cloud-security B09 tests supplied JSON only.
//! NEGATIVE-TEST: missing, malformed, duplicate, and unknown manifest fields are rejected.
//! ROUNDTRIP-TEST: the positive fixtures decode through the production validator.

use std::collections::BTreeSet;
use std::path::PathBuf;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_lang_security::rules::cyberskills::cloud_security_b09::CloudSecurityManifestB09Validator;
use enforcer_validator::validator::{ValidationInput, Validator};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture(
    path: &str,
) -> Result<Vec<enforcer_domain::findings::Finding>, Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(manifest_dir().join(path))?;
    let file: RelPath = "cloud-security-manifest-b09.json".parse()?;
    let validator = CloudSecurityManifestB09Validator::new()?;
    Ok(validator.validate(ValidationInput {
        file: &file,
        source: enforcer_domain::boundary::validation::ValidationSource::from_text(&source),
        scope: ScanScope::Files,
    }))
}

#[test]
fn cloud_security_manifest_b09_pass_and_boundary_are_clean(
) -> Result<(), Box<dyn std::error::Error>> {
    let pass = fixture("tests/fixtures/cyberskills/cloud-security-manifest-b09/pass.json")?;
    let boundary = fixture("tests/fixtures/cyberskills/cloud-security-manifest-b09/boundary.json")?;
    assert!(pass.is_empty(), "pass findings: {pass:?}");
    assert!(boundary.is_empty(), "boundary findings: {boundary:?}");
    Ok(())
}

#[test]
fn cloud_security_manifest_b09_fail_and_malformed_are_rejected(
) -> Result<(), Box<dyn std::error::Error>> {
    let failures = fixture("tests/fixtures/cyberskills/cloud-security-manifest-b09/fail.json")?;
    let malformed =
        fixture("tests/fixtures/cyberskills/cloud-security-manifest-b09/malformed.json")?;
    assert_eq!(failures.len(), 1);
    assert_eq!(malformed.len(), 1);
    assert!(failures
        .iter()
        .chain(malformed.iter())
        .all(|finding| finding.rule_id.as_str() == "CYBER-CLOUD-MANIFEST.9"));
    Ok(())
}

#[test]
fn cloud_security_manifest_b09_pass_covers_all_five_skills(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(
        manifest_dir().join("tests/fixtures/cyberskills/cloud-security-manifest-b09/pass.json"),
    )?;
    let document: serde_json::Value = serde_json::from_str(&source)?;
    let ids: BTreeSet<&str> = document["records"]
        .as_array()
        .ok_or("pass fixture records must be an array")?
        .iter()
        .filter_map(|record| record["skillId"].as_str())
        .collect();
    assert_eq!(
        ids,
        BTreeSet::from([
            "implementing-gcp-binary-authorization",
            "implementing-gcp-organization-policy-constraints",
            "implementing-gcp-vpc-firewall-rules",
            "implementing-secrets-management-with-vault",
            "implementing-zero-trust-in-cloud",
        ])
    );
    Ok(())
}

#[test]
fn cloud_security_manifest_b09_preserves_static_only_boundary(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(
        manifest_dir().join("tests/fixtures/cyberskills/cloud-security-manifest-b09/boundary.json"),
    )?;
    let document: serde_json::Value = serde_json::from_str(&source)?;
    assert_eq!(document["scope"], "scope:offline-authorized-static-only");
    let evidence = document["evidence"]
        .as_array()
        .ok_or("boundary evidence must be an array")?;
    assert!(evidence.iter().any(|entry| {
        entry["kind"] == "authorization" && entry["reference"] == "evidence:written-scope"
    }));
    let records = document["records"]
        .as_array()
        .ok_or("boundary records must be an array")?;
    assert!(records.iter().all(|record| {
        record.get("providerApi").is_none()
            && record.get("liveEndpoint").is_none()
            && record.get("credential").is_none()
            && record.get("session").is_none()
            && record.get("networkCall").is_none()
    }));
    Ok(())
}
