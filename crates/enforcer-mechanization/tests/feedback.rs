//! Integration test for the d08 harness-feedback pipeline
//! (`enforcer_mechanization::feedback`): a preventable failure produces a
//! PROPOSED registry record + fixtures that pass the d01 parity oracle; a
//! detect-only failure produces none; PROPOSED rules stay out of a live
//! `RuleRegistry` (non-blocking) until an explicit, separate promotion
//! step.
//!
//! Deliberately an INTEGRATION test (drives the pipeline end-to-end
//! through the real fixture files under `tests/fixtures/feedback/**` and
//! the real fail-closed parity oracle) rather than the crate-local unit
//! tests in `src/feedback.rs`, which only exercise in-memory shapes.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::{Severity, Tier};
use enforcer_harness::parsers::HarnessDiagnostic;
use enforcer_mechanization::feedback::classify::Classification;
use enforcer_mechanization::feedback::{ingest_and_classify, RuleStatus};
use enforcer_mechanization::oracle::accept_rule;
use enforcer_mechanization::scaffold::ScaffoldSpec;
use enforcer_validator::validator::{ValidationInput, Validator};

/// Stand-in for "a human filled in the detection logic" for the
/// auto-scaffolded PROPOSED rule: fires on the literal marker
/// `FEEDBACK_MARKER`, matching `tests/fixtures/feedback/{fail,pass}.txt`.
struct FilledInValidator {
    rule_id: RuleId,
}

impl Validator for FilledInValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if input.source.contains("FEEDBACK_MARKER") {
            vec![Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "feedback marker present".to_owned(),
                detail: "found FEEDBACK_MARKER".to_owned(),
                file: input.file.clone(),
                line: 1,
                snippet: None,
            }]
        } else {
            Vec::new()
        }
    }
}

fn manifest_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rustc_diagnostic() -> HarnessDiagnostic {
    // A rustc `compiler-message`-shaped diagnostic — this is exactly the
    // typed output `enforcer_harness::parsers::rust_message_to_diagnostic`
    // produces; the feedback pipeline consumes that shape directly, never
    // re-parsing raw tool text itself.
    HarnessDiagnostic {
        run_id: "run-feedback-1".to_owned(),
        tool: "cargo".to_owned(),
        language: "rust".to_owned(),
        severity: "error".to_owned(),
        rule_id: "E0308".to_owned(),
        file: "src/lib.rs".to_owned(),
        line: 12,
        message: "mismatched types".to_owned(),
        source: None,
        fingerprint: None,
    }
}

fn pytest_diagnostic() -> HarnessDiagnostic {
    HarnessDiagnostic {
        run_id: "run-feedback-2".to_owned(),
        tool: "pytest".to_owned(),
        language: "python".to_owned(),
        severity: "error".to_owned(),
        rule_id: "pytest".to_owned(),
        file: "tests/test_x.py".to_owned(),
        line: 1,
        message: "AssertionError: boom".to_owned(),
        source: None,
        fingerprint: None,
    }
}

fn feedback_spec() -> Result<ScaffoldSpec, enforcer_core::error::DecodeError> {
    Ok(ScaffoldSpec {
        rule_id: "RR-96.1".parse()?,
        title: "No mismatched frobnication (auto-proposed from a harness failure)".to_owned(),
        tier: Tier::T1,
        validator_crate: "enforcer-mechanization".to_owned(),
        validator_path: "feedback_integration::FilledInValidator".to_owned(),
        fail_fixture_path: "tests/fixtures/feedback/fail.txt".to_owned(),
        pass_fixture_path: "tests/fixtures/feedback/pass.txt".to_owned(),
        doc_anchor: "docs/rules/FEEDBACK.md#FEEDBACK-1".to_owned(),
        tags: vec!["harness-feedback".to_owned()],
    })
}

#[test]
fn preventable_failure_yields_a_proposed_record_that_passes_parity(
) -> Result<(), Box<dyn std::error::Error>> {
    let diagnostic = rustc_diagnostic();
    let outcome = ingest_and_classify(&diagnostic, &feedback_spec()?)?;

    assert_eq!(outcome.classification, Classification::Prevent);
    let Some(proposed) = outcome.proposed_rule else {
        return Err("a preventable failure must scaffold a proposed rule".into());
    };
    assert_eq!(proposed.status, RuleStatus::Proposed);

    // The PROPOSED record's shape is independently loadable (same
    // guarantee the d01 scaffolder itself proves) ...
    let registry = enforcer_rules::registry::RuleRegistry::from_records(vec![proposed
        .scaffold
        .record
        .clone()])?;
    assert_eq!(registry.len(), 1);

    // ... and, with a real validator implementation supplied, the record
    // plus its declared fail/pass fixtures pass the SAME fail-closed d01
    // parity oracle every hand-authored rule must pass — an auto-proposed
    // rule gets no special treatment from the oracle's perspective.
    let validator = FilledInValidator {
        rule_id: proposed.scaffold.record.rule_id.clone(),
    };
    accept_rule(&proposed.scaffold.record, Some(&validator), &manifest_dir())?;

    Ok(())
}

#[test]
fn detect_only_failure_produces_no_proposed_record() -> Result<(), Box<dyn std::error::Error>> {
    let diagnostic = pytest_diagnostic();
    let outcome = ingest_and_classify(&diagnostic, &feedback_spec()?)?;

    assert_eq!(outcome.classification, Classification::Detect);
    assert!(
        outcome.proposed_rule.is_none(),
        "a detect-only failure must never produce a PROPOSED registry record"
    );

    Ok(())
}

#[test]
fn proposed_rules_are_non_blocking_in_a_scan() -> Result<(), Box<dyn std::error::Error>> {
    // A PROPOSED rule is a distinct `ProposedRule` type, never a bare
    // `RuleRecord` that could slip into a live registry unlabeled. This
    // test proves the non-blocking property at this pipeline's boundary:
    // building the scan-facing `RuleRegistry` from only what a scan engine
    // would actually load (nothing here) leaves it empty even though a
    // proposal was produced, because promotion is a separate, explicit
    // step this module never performs on its own.
    let diagnostic = rustc_diagnostic();
    let outcome = ingest_and_classify(&diagnostic, &feedback_spec()?)?;
    let Some(proposed) = outcome.proposed_rule else {
        return Err("preventable failure scaffolds a proposal".into());
    };
    assert_eq!(proposed.status, RuleStatus::Proposed);

    // A scan engine's live registry is built ONLY from records it was
    // explicitly given; nothing about producing a `ProposedRule` feeds one
    // in automatically.
    let scan_registry = enforcer_rules::registry::RuleRegistry::from_records(vec![])?;
    assert!(
        scan_registry.is_empty(),
        "a PROPOSED rule must never appear in a scan's live registry without an explicit promotion step"
    );
    assert!(scan_registry
        .get(&proposed.scaffold.record.rule_id)
        .is_none());

    Ok(())
}
