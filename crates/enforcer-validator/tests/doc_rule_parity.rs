//! Integration test for the doc-rule parity oracle (d09): drives
//! `enforcer_validator::doc_rule_parity` over the three fixtures under
//! `tests/fixtures/doc_rule_parity/**`, proving the real-detection
//! contract the workpack requires:
//!
//! - `full-parity`: a bullet citing a real, registered id passes (no
//!   [`enforcer_domain::findings::Finding`]).
//! - `doc-with-no-validator`: an uncited/dangling-id bullet fails (emits a
//!   `Finding`) — prose pretending to be enforcement.
//! - `validator-with-no-doc`: a registered rule with no citing bullet
//!   anywhere in the doc corpus surfaces as a T2 advisory `Finding`
//!   (non-blocking).
//!
//! Also proves persona free-text (non-bullet prose that happens to contain
//! "must"/"never") is ignored by the T1 gate.

use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::severity::Severity;
use enforcer_rules::loader::load_registry_from_files;
use enforcer_validator::doc_rule_parity::{check_doc_against_registry, find_undocumented_rules};

fn manifest_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture_dir(name: &str) -> std::path::PathBuf {
    manifest_dir()
        .join("tests/fixtures/doc_rule_parity")
        .join(name)
}

fn read_doc(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(fixture_dir(name).join("doc.md"))?)
}

#[test]
fn full_parity_bullet_citing_real_rule_passes() -> Result<(), Box<dyn std::error::Error>> {
    let dir = fixture_dir("full-parity");
    let registry = load_registry_from_files(&[dir.join("registry.json").as_path()])?;
    let doc_path = "docs/agents/common.md".parse()?;
    let source = read_doc("full-parity")?;

    let findings =
        check_doc_against_registry(&doc_path, ValidationSource::from_text(&source), &registry)?;

    assert!(
        findings.is_empty(),
        "expected zero findings for a bullet citing a registered rule, got {findings:?}"
    );
    Ok(())
}

#[test]
fn doc_with_no_validator_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let dir = fixture_dir("doc-with-no-validator");
    let registry = load_registry_from_files(&[dir.join("registry.json").as_path()])?;
    let doc_path = "docs/agents/common.md".parse()?;
    let source = read_doc("doc-with-no-validator")?;

    let findings =
        check_doc_against_registry(&doc_path, ValidationSource::from_text(&source), &registry)?;

    assert_eq!(
        findings.len(),
        1,
        "expected exactly one finding for a bullet citing an unregistered ruleId, got {findings:?}"
    );
    assert_eq!(findings[0].severity, Severity::Error);
    assert!(findings[0].detail.as_str().contains("RR-100.2"));
    Ok(())
}

#[test]
fn validator_with_no_doc_surfaces_as_advisory() -> Result<(), Box<dyn std::error::Error>> {
    let dir = fixture_dir("validator-with-no-doc");
    let registry = load_registry_from_files(&[dir.join("registry.json").as_path()])?;
    let advisory_path = "docs/agents".parse()?;
    let source = read_doc("validator-with-no-doc")?;

    // No must/never bullet in this doc cites RR-100.3 at all, so the T1 gate
    // over this single doc produces zero findings...
    let doc_path = "docs/agents/common.md".parse()?;
    let gate_findings =
        check_doc_against_registry(&doc_path, ValidationSource::from_text(&source), &registry)?;
    assert!(
        gate_findings.is_empty(),
        "a doc with no must/never bullets should not trip the T1 gate, got {gate_findings:?}"
    );

    // ...but the reverse (T2 advisory) check flags the registered rule as
    // undocumented, since no doc in the corpus cites it.
    let advisory_findings = find_undocumented_rules(
        [ValidationSource::from_text(&source)],
        &registry,
        &advisory_path,
    )?;
    assert_eq!(
        advisory_findings.len(),
        1,
        "expected exactly one advisory finding for an undocumented registered rule, got {advisory_findings:?}"
    );
    assert_eq!(advisory_findings[0].severity, Severity::Warning);
    assert_eq!(advisory_findings[0].rule_id.as_str(), "RR-100.3");
    Ok(())
}

#[test]
fn full_parity_doc_leaves_no_undocumented_advisory() -> Result<(), Box<dyn std::error::Error>> {
    let dir = fixture_dir("full-parity");
    let registry = load_registry_from_files(&[dir.join("registry.json").as_path()])?;
    let advisory_path = "docs/agents".parse()?;
    let source = read_doc("full-parity")?;

    let advisory_findings = find_undocumented_rules(
        [ValidationSource::from_text(&source)],
        &registry,
        &advisory_path,
    )?;
    assert!(
        advisory_findings.is_empty(),
        "RR-100.1 is cited by the full-parity doc, so it must not surface as undocumented: {advisory_findings:?}"
    );
    Ok(())
}

#[test]
fn free_text_must_never_prose_is_not_gated() -> Result<(), Box<dyn std::error::Error>> {
    // The full-parity fixture's doc.md deliberately includes an explanatory
    // paragraph containing "must"/"never" that is NOT a markdown bullet;
    // proving that paragraph contributes no findings confirms persona
    // free-text is ignored by the T1 gate (only bullets are checked).
    let dir = fixture_dir("full-parity");
    let registry = load_registry_from_files(&[dir.join("registry.json").as_path()])?;
    let doc_path = "docs/agents/common.md".parse()?;
    let source = read_doc("full-parity")?;
    assert!(
        source.contains("Explanatory prose that happens to say must and never"),
        "fixture drifted: expected the free-text paragraph to still be present"
    );

    let findings =
        check_doc_against_registry(&doc_path, ValidationSource::from_text(&source), &registry)?;
    assert!(
        findings.is_empty(),
        "free-text must/never prose must not be gated, got {findings:?}"
    );
    Ok(())
}
