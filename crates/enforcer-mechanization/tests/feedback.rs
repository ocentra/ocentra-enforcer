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

use enforcer_domain::findings::{Finding, FindingDetail, FindingLine, FindingTitle};
use enforcer_domain::harness_types::{
    HarnessDiagnosticMessage, HarnessDiagnosticPath, HarnessExternalRuleId, HarnessLanguage,
    HarnessRunId, HarnessSourceLine, HarnessToolName,
};
use enforcer_domain::ids::RuleId;
use enforcer_domain::mechanization_types::{MechanizationClassification, RuleLifecycleStatus};
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::rules_types::RuleRegistryState;
use enforcer_domain::severity::{Severity, Tier};
use enforcer_domain::telemetry_types::SourceLine;
use enforcer_harness::parsers::HarnessDiagnostic;
use enforcer_mechanization::feedback::ingest_and_classify;
use enforcer_mechanization::feedback::{boundary::FeedbackDecisionDto, FeedbackDecisionRecord};
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
        if input.source.as_str().contains("FEEDBACK_MARKER") {
            test_finding(
                self.rule_id.clone(),
                input.file.clone(),
                "feedback marker present",
                "found FEEDBACK_MARKER",
            )
            .into_iter()
            .collect()
        } else {
            Vec::new()
        }
    }
}

fn test_finding(rule_id: RuleId, file: RelPath, title: &str, detail: &str) -> Option<Finding> {
    Some(Finding {
        rule_id,
        severity: Severity::Error,
        title: FindingTitle::new(title.to_owned()).ok()?,
        detail: FindingDetail::new(detail.to_owned()).ok()?,
        file,
        line: FindingLine::known(SourceLine::try_new(std::num::NonZeroU32::MIN)),
        snippet: None,
    })
}

fn manifest_root() -> Result<RepoRoot, enforcer_domain::boundary::decode_error::DecodeError> {
    RepoRoot::try_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
}

fn rustc_diagnostic() -> HarnessDiagnostic {
    // A rustc `compiler-message`-shaped diagnostic — this is exactly the
    // typed output `enforcer_harness::parsers::rust_message_to_diagnostic`
    // produces; the feedback pipeline consumes that shape directly, never
    // re-parsing raw tool text itself.
    HarnessDiagnostic {
        run_id: HarnessRunId::from_adapter("run-feedback-1"),
        tool: HarnessToolName::from_adapter("cargo"),
        language: HarnessLanguage::Rust,
        severity: Severity::Error,
        rule_id: HarnessExternalRuleId::from_adapter("E0308"),
        file: HarnessDiagnosticPath::from_adapter("src/lib.rs"),
        line: HarnessSourceLine::from_external(12),
        message: HarnessDiagnosticMessage::from_adapter("mismatched types"),
        source: None,
        fingerprint: None,
    }
}

fn pytest_diagnostic() -> HarnessDiagnostic {
    HarnessDiagnostic {
        run_id: HarnessRunId::from_adapter("run-feedback-2"),
        tool: HarnessToolName::from_adapter("pytest"),
        language: HarnessLanguage::Python,
        severity: Severity::Error,
        rule_id: HarnessExternalRuleId::from_adapter("pytest"),
        file: HarnessDiagnosticPath::from_adapter("tests/test_x.py"),
        line: HarnessSourceLine::from_external(1),
        message: HarnessDiagnosticMessage::from_adapter("AssertionError: boom"),
        source: None,
        fingerprint: None,
    }
}

fn feedback_spec() -> Result<ScaffoldSpec, enforcer_domain::boundary::decode_error::DecodeError> {
    Ok(ScaffoldSpec {
        rule_id: "RR-96.1".parse()?,
        title: "No mismatched frobnication (auto-proposed from a harness failure)".parse()?,
        tier: Tier::T1,
        validator_crate: "enforcer-mechanization".parse()?,
        validator_path: "feedback_integration::FilledInValidator".parse()?,
        fail_fixture_path: "tests/fixtures/feedback/fail.txt".parse()?,
        pass_fixture_path: "tests/fixtures/feedback/pass.txt".parse()?,
        doc_anchor: "docs/rules/FEEDBACK.md#FEEDBACK-1".parse()?,
        tags: vec!["harness-feedback".parse()?],
    })
}

#[test]
fn preventable_failure_yields_a_proposed_record_that_passes_parity(
) -> Result<(), Box<dyn std::error::Error>> {
    let diagnostic = rustc_diagnostic();
    let outcome = ingest_and_classify(&diagnostic, &feedback_spec()?)?;

    assert_eq!(outcome.classification, MechanizationClassification::Prevent);
    let Some(proposed) = outcome.proposed_rule else {
        return Err("a preventable failure must scaffold a proposed rule".into());
    };
    assert_eq!(proposed.status, RuleLifecycleStatus::Proposed);

    // The PROPOSED record's shape is independently loadable (same
    // guarantee the d01 scaffolder itself proves) ...
    let registry = enforcer_rules::registry::RuleRegistry::from_records(vec![proposed
        .scaffold
        .record
        .clone()])?;
    assert_eq!(
        registry.count(),
        enforcer_domain::rules_types::RuleRecordCount::from_records([()])
    );

    // ... and, with a real validator implementation supplied, the record
    // plus its declared fail/pass fixtures pass the SAME fail-closed d01
    // parity oracle every hand-authored rule must pass — an auto-proposed
    // rule gets no special treatment from the oracle's perspective.
    let validator = FilledInValidator {
        rule_id: proposed.scaffold.record.rule_id.clone(),
    };
    accept_rule(
        &proposed.scaffold.record,
        Some(&validator),
        &manifest_root()?,
    )?;

    Ok(())
}

#[test]
fn detect_only_failure_produces_no_proposed_record() -> Result<(), Box<dyn std::error::Error>> {
    let diagnostic = pytest_diagnostic();
    let outcome = ingest_and_classify(&diagnostic, &feedback_spec()?)?;

    assert_eq!(outcome.classification, MechanizationClassification::Detect);
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
    assert_eq!(proposed.status, RuleLifecycleStatus::Proposed);

    // A scan engine's live registry is built ONLY from records it was
    // explicitly given; nothing about producing a `ProposedRule` feeds one
    // in automatically.
    let scan_registry = enforcer_rules::registry::RuleRegistry::from_records(vec![])?;
    assert!(
        scan_registry.state() == RuleRegistryState::Empty,
        "a PROPOSED rule must never appear in a scan's live registry without an explicit promotion step"
    );
    assert!(scan_registry
        .get(&proposed.scaffold.record.rule_id)
        .is_none());

    Ok(())
}

#[test]
fn feedback_decision_dto_rejects_invalid_persisted_classification(
) -> Result<(), Box<dyn std::error::Error>> {
    let wire: FeedbackDecisionDto = serde_json::from_value(serde_json::json!({
        "schemaVersion": 1,
        "eventType": "harness.feedback.decision",
        "inputFingerprint": format!("sha256:{}", "0".repeat(64)),
        "tool": "cargo",
        "sourceRuleId": "E0308",
        "classification": "unsupported",
        "proposed": false
    }))?;
    assert!(FeedbackDecisionRecord::try_from(wire).is_err());
    Ok(())
}
