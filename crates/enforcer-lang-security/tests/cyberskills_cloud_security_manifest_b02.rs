//! BOUNDARY-INVARIANT: CP09 cloud-security B02 tests supplied JSON only.
//! NEGATIVE-TEST: missing, malformed, duplicate, and unknown manifest fields are rejected.
//! ROUNDTRIP-TEST: the positive fixtures decode through the production validator.

use std::collections::BTreeSet;
use std::path::PathBuf;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_lang_security::rules::cyberskills::cloud_security_b02::CloudSecurityManifestB02Validator;
use enforcer_validator::validator::{ValidationInput, Validator};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture(
    path: &str,
) -> Result<Vec<enforcer_domain::findings::Finding>, Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(manifest_dir().join(path))?;
    let file: RelPath = "cloud-security-manifest-b02.json".parse()?;
    let validator = CloudSecurityManifestB02Validator::new()?;
    Ok(validator.validate(ValidationInput {
        file: &file,
        source: enforcer_domain::boundary::validation::ValidationSource::from_text(&source),
        scope: ScanScope::Files,
    }))
}

#[test]
fn cloud_security_manifest_b02_pass_and_boundary_are_clean(
) -> Result<(), Box<dyn std::error::Error>> {
    let pass = fixture("tests/fixtures/cyberskills/cloud-security-manifest-b02/pass.json")?;
    let boundary = fixture("tests/fixtures/cyberskills/cloud-security-manifest-b02/boundary.json")?;
    assert!(pass.is_empty(), "pass findings: {pass:?}");
    assert!(boundary.is_empty(), "boundary findings: {boundary:?}");
    Ok(())
}

#[test]
fn cloud_security_manifest_b02_fail_and_malformed_are_rejected(
) -> Result<(), Box<dyn std::error::Error>> {
    let failures = fixture("tests/fixtures/cyberskills/cloud-security-manifest-b02/fail.json")?;
    let malformed =
        fixture("tests/fixtures/cyberskills/cloud-security-manifest-b02/malformed.json")?;
    assert_eq!(failures.len(), 1);
    assert_eq!(malformed.len(), 1);
    assert!(failures
        .iter()
        .chain(malformed.iter())
        .all(|finding| finding.rule_id.as_str() == "CYBER-CLOUD-MANIFEST.2"));
    Ok(())
}

#[test]
fn cloud_security_manifest_b02_pass_covers_all_five_skills(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(
        manifest_dir().join("tests/fixtures/cyberskills/cloud-security-manifest-b02/pass.json"),
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
            "auditing-gcp-iam-permissions",
            "auditing-kubernetes-cluster-rbac",
            "auditing-terraform-infrastructure-for-security",
            "building-cloud-siem-with-sentinel",
            "conducting-cloud-penetration-testing",
        ])
    );
    Ok(())
}

#[test]
fn cloud_security_manifest_b02_preserves_static_only_boundary(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(
        manifest_dir().join("tests/fixtures/cyberskills/cloud-security-manifest-b02/boundary.json"),
    )?;
    let document: serde_json::Value = serde_json::from_str(&source)?;
    assert_eq!(document["scope"], "scope:offline-evidence");
    let evidence = document["evidence"]
        .as_array()
        .ok_or("boundary evidence must be an array")?;
    assert!(evidence.iter().any(|entry| {
        entry["kind"] == "authorization"
            && entry["reference"] == "written-authorization:review-only"
    }));
    let records = document["records"]
        .as_array()
        .ok_or("boundary records must be an array")?;
    let assessment = records
        .iter()
        .find(|record| record["skillId"] == "conducting-cloud-penetration-testing")
        .ok_or("boundary assessment record is required")?;
    assert_eq!(assessment["authorizationRef"], "authorization:written");
    assert_eq!(assessment["stopConditionRef"], "stop-condition:offline");
    assert!(records
        .iter()
        .all(|record| record.get("credentialRef").is_none() && record.get("liveApi").is_none()));
    Ok(())
}
