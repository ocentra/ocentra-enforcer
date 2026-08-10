//! BOUNDARY-INVARIANT: CP09 AI-security packets validate supplied JSON only; no live AI or
//! external-engine execution is part of this proof.

use std::collections::BTreeSet;
use std::path::PathBuf;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_lang_security::rules::cyberskills::ai_security_manifest::AiSecurityManifestValidator;
use enforcer_lang_security::rules::cyberskills::registry;
use enforcer_validator::validator::{ValidationInput, Validator};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture(
    path: &str,
) -> Result<Vec<enforcer_domain::findings::Finding>, Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(manifest_dir().join(path))?;
    let file: RelPath = "ai-security-manifest.json".parse()?;
    let validator = AiSecurityManifestValidator::new()?;
    Ok(validator.validate(ValidationInput {
        file: &file,
        source: enforcer_domain::boundary::validation::ValidationSource::from_text(&source),
        scope: ScanScope::Files,
    }))
}

#[test]
fn ai_security_manifest_pass_and_boundary_fixtures_are_clean(
) -> Result<(), Box<dyn std::error::Error>> {
    let pass = fixture("tests/fixtures/cyberskills/ai-security-manifest-b01/pass.json")?;
    let boundary = fixture("tests/fixtures/cyberskills/ai-security-manifest-b01/boundary.json")?;
    assert!(pass.is_empty(), "pass findings: {pass:?}");
    assert!(boundary.is_empty(), "boundary findings: {boundary:?}");
    Ok(())
}

#[test]
fn ai_security_manifest_fail_and_malformed_fixtures_are_rejected(
) -> Result<(), Box<dyn std::error::Error>> {
    let failures = fixture("tests/fixtures/cyberskills/ai-security-manifest-b01/fail.json")?;
    let malformed = fixture("tests/fixtures/cyberskills/ai-security-manifest-b01/malformed.json")?;
    assert_eq!(failures.len(), 1);
    assert_eq!(malformed.len(), 1);
    assert!(failures
        .iter()
        .chain(malformed.iter())
        .all(|finding| finding.rule_id.as_str() == "CYBER-AI-MANIFEST.1"));
    Ok(())
}

#[test]
fn ai_security_manifest_pass_fixture_covers_the_five_selected_skills(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(
        manifest_dir().join("tests/fixtures/cyberskills/ai-security-manifest-b01/pass.json"),
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
            "assessing-vector-and-embedding-weaknesses",
            "auditing-mcp-servers-for-tool-poisoning",
            "continuous-llm-red-teaming-with-promptfoo",
            "defending-llms-with-guardrails",
            "detecting-ai-model-prompt-injection-attacks",
        ])
    );
    Ok(())
}

#[test]
fn ai_security_manifest_is_registered_once() -> Result<(), Box<dyn std::error::Error>> {
    let rows = registry::build_all()?;
    assert_eq!(rows.len(), 41);
    assert_eq!(
        rows.iter()
            .filter(|row| row.rule_id().as_str() == "CYBER-AI-MANIFEST.1")
            .count(),
        1
    );
    Ok(())
}

#[test]
fn ai_security_manifest_b02_pass_covers_five_remaining_intents(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(
        manifest_dir().join("tests/fixtures/cyberskills/ai-security-manifest-b02/pass.json"),
    )?;
    let document: serde_json::Value = serde_json::from_str(&source)?;
    let ids: BTreeSet<&str> = document["records"]
        .as_array()
        .ok_or("B02 pass fixture records must be an array")?
        .iter()
        .filter_map(|record| record["skillId"].as_str())
        .collect();
    assert_eq!(ids.len(), 5);
    assert_eq!(
        ids,
        BTreeSet::from([
            "detecting-data-and-model-poisoning",
            "detecting-indirect-prompt-injection",
            "detecting-model-extraction-attacks",
            "implementing-llm-guardrails-for-security",
            "orchestrating-llm-attacks-with-pyrit",
        ])
    );
    Ok(())
}

#[test]
fn ai_security_manifest_b02_negative_classes_are_rejected() -> Result<(), Box<dyn std::error::Error>>
{
    let failures = fixture("tests/fixtures/cyberskills/ai-security-manifest-b02/fail.json")?;
    let malformed = fixture("tests/fixtures/cyberskills/ai-security-manifest-b02/malformed.json")?;
    assert_eq!(failures.len(), 1);
    assert_eq!(malformed.len(), 1);
    assert!(failures
        .iter()
        .chain(malformed.iter())
        .all(|finding| finding.rule_id.as_str() == "CYBER-AI-MANIFEST.1"));
    Ok(())
}

#[test]
fn ai_security_manifest_b03_pass_covers_four_remaining_intents(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(
        manifest_dir().join("tests/fixtures/cyberskills/ai-security-manifest-b03/pass.json"),
    )?;
    let document: serde_json::Value = serde_json::from_str(&source)?;
    let ids: BTreeSet<&str> = document["records"]
        .as_array()
        .ok_or("B03 pass fixture records must be an array")?
        .iter()
        .filter_map(|record| record["skillId"].as_str())
        .collect();
    assert_eq!(ids.len(), 4);
    assert_eq!(
        ids,
        BTreeSet::from([
            "red-teaming-llms-with-garak",
            "securing-agentic-ai-tool-invocation",
            "testing-for-system-prompt-leakage",
            "testing-prompt-injection-in-rag-pipelines",
        ])
    );
    Ok(())
}

#[test]
fn ai_security_manifest_b03_negative_classes_are_rejected() -> Result<(), Box<dyn std::error::Error>>
{
    let failures = fixture("tests/fixtures/cyberskills/ai-security-manifest-b03/fail.json")?;
    let malformed = fixture("tests/fixtures/cyberskills/ai-security-manifest-b03/malformed.json")?;
    assert_eq!(failures.len(), 1);
    assert_eq!(malformed.len(), 1);
    assert!(failures
        .iter()
        .chain(malformed.iter())
        .all(|finding| finding.rule_id.as_str() == "CYBER-AI-MANIFEST.1"));
    Ok(())
}
