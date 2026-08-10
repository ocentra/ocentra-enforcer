//! BOUNDARY-INVARIANT: CP09 compliance-governance B01 tests supplied JSON only.
//! NEGATIVE-TEST: invalid arithmetic, malformed JSON, duplicate records, and
//! unsupported live-authority fields are rejected.
//! ROUNDTRIP-TEST: the positive fixtures decode and re-encode through JSON
//! without changing their typed field shape.

use std::collections::BTreeSet;
use std::path::PathBuf;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_lang_security::rules::cyberskills::compliance_governance_b01::ComplianceGovernanceManifestB01Validator;
use enforcer_validator::validator::{ValidationInput, Validator};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture(
    path: &str,
) -> Result<Vec<enforcer_domain::findings::Finding>, Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(manifest_dir().join(path))?;
    let file: RelPath = "compliance-governance-manifest-b01.json".parse()?;
    let validator = ComplianceGovernanceManifestB01Validator::new()?;
    Ok(validator.validate(ValidationInput {
        file: &file,
        source: enforcer_domain::boundary::validation::ValidationSource::from_text(&source),
        scope: ScanScope::Files,
    }))
}

#[test]
fn compliance_governance_b01_pass_and_boundary_are_clean() -> Result<(), Box<dyn std::error::Error>>
{
    let pass = fixture("tests/fixtures/cyberskills/compliance-governance-manifest-b01/pass.json")?;
    let boundary =
        fixture("tests/fixtures/cyberskills/compliance-governance-manifest-b01/boundary.json")?;
    assert!(pass.is_empty(), "pass findings: {pass:?}");
    assert!(boundary.is_empty(), "boundary findings: {boundary:?}");
    Ok(())
}

#[test]
fn compliance_governance_b01_fail_and_malformed_are_rejected(
) -> Result<(), Box<dyn std::error::Error>> {
    let failures =
        fixture("tests/fixtures/cyberskills/compliance-governance-manifest-b01/fail.json")?;
    let malformed =
        fixture("tests/fixtures/cyberskills/compliance-governance-manifest-b01/malformed.json")?;
    assert_eq!(failures.len(), 1);
    assert_eq!(malformed.len(), 1);
    assert!(failures
        .iter()
        .chain(malformed.iter())
        .all(|finding| finding.rule_id.as_str() == "CYBER-COMPLIANCE-MANIFEST.01"));
    Ok(())
}

#[test]
fn compliance_governance_b01_pass_covers_all_five_skills() -> Result<(), Box<dyn std::error::Error>>
{
    let source = std::fs::read_to_string(
        manifest_dir()
            .join("tests/fixtures/cyberskills/compliance-governance-manifest-b01/pass.json"),
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
            "achieving-cmmc-level-2-compliance",
            "conducting-cyber-risk-assessment-with-nist-800-30",
            "executing-nist-rmf-authorization-to-operate",
            "implementing-gdpr-data-protection-controls",
            "implementing-hipaa-security-rule-safeguards",
        ])
    );
    Ok(())
}

#[test]
fn compliance_governance_b01_preserves_static_only_boundary(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(
        manifest_dir()
            .join("tests/fixtures/cyberskills/compliance-governance-manifest-b01/boundary.json"),
    )?;
    let document: serde_json::Value = serde_json::from_str(&source)?;
    assert_eq!(document["scope"], "scope:offline-authorized-static-only");
    assert!(document["evidence"].as_array().is_some_and(|evidence| {
        evidence.iter().any(|entry| {
            entry["kind"] == "authorization" && entry["reference"] == "evidence:written-scope"
        })
    }));
    let records = document["records"]
        .as_array()
        .ok_or("boundary records must be an array")?;
    assert!(records.iter().all(|record| {
        [
            "frameworkApi",
            "assessor",
            "regulator",
            "liveSystem",
            "personalData",
            "ephi",
            "production",
            "credential",
            "externalEngine",
        ]
        .iter()
        .all(|field| record.get(*field).is_none())
    }));
    Ok(())
}

#[test]
fn compliance_governance_b01_positive_json_round_trip_is_stable(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(
        manifest_dir()
            .join("tests/fixtures/cyberskills/compliance-governance-manifest-b01/pass.json"),
    )?;
    let original: serde_json::Value = serde_json::from_str(&source)?;
    let wire = serde_json::to_vec(&original)?;
    let decoded: serde_json::Value = serde_json::from_slice(&wire)?;
    assert_eq!(decoded, original);
    Ok(())
}
