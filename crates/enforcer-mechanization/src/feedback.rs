//! Harness feedback pipeline (d08): turn an escaped harness failure into a
//! candidate rule instead of letting the lesson die in a chat log.
//!
//! `enforcer-harness` (arc-18) already parses native-tool output into
//! [`enforcer_harness::parsers::HarnessDiagnostic`] records; this module is
//! the missing link ADBP's "close the loop" prose pointed at:
//!
//! 1. Ingest a diagnostic (typed, already produced by `enforcer-harness` —
//!    this module does not re-parse raw tool output).
//! 2. Classify it [`classify::Classification::Prevent`] vs
//!    [`classify::Classification::Detect`] via [`classify::classify`] —
//!    mechanical field matching, never an LLM judgment call.
//! 3. For `Prevent`, call [`crate::scaffold::scaffold_rule`] (the d01
//!    scaffolder already in this crate) to emit a `Validator` stub + doc
//!    anchor + fixture slots, wrapped as a [`ProposedRule`] carrying a
//!    typed, non-blocking [`RuleStatus::Proposed`] tag.
//! 4. Log the classification decision as a [`FeedbackDecisionRecord`]
//!    (versioned serde struct, carrying the input's [`Sha256`] fingerprint)
//!    appendable via the `enforcer-core` NDJSON sink.
//!
//! ## `RuleStatus` seam (documented, not silently absorbed)
//! [`enforcer_rules::registry::RuleRecord`] has no `status` field yet — that
//! field is out of this workpack's `owns:` (it lives in `enforcer-rules`,
//! owned by d01/arc-14). [`ProposedRule`] carries the typed status
//! ALONGSIDE the record rather than mutating the shared registry DTO, so a
//! future pass can fold `RuleStatus` directly onto `RuleRecord` without this
//! module needing to change: [`ProposedRule::record`] is exactly the
//! `RuleRecord` the scaffolder would have produced standalone. "PROPOSED
//! rules are non-blocking in a scan" is proven at THIS module's boundary —
//! a `ProposedRule` is a distinct type from a registry-ready `RuleRecord`,
//! so nothing in this crate's own output can reach a scan engine's
//! `RuleRegistry` without an explicit, separate promotion step that is
//! itself out of scope here.

pub mod classify;

use enforcer_domain::hashes::Sha256;
use enforcer_harness::parsers::HarnessDiagnostic;

use crate::error::MechanizationResult;
use crate::scaffold::{scaffold_rule, ScaffoldOutput, ScaffoldSpec};
use classify::Classification;

/// Typed, machine-readable rule-record status. Closed set: only
/// [`RuleStatus::Proposed`] is produced by this pipeline today, but the
/// enum exists (rather than a bare bool) so a later `Promoted`/`Rejected`
/// variant is a non-breaking addition, matching the workpack's "a
/// `Tier`/status field ... not prose" requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuleStatus {
    /// Auto-scaffolded from a harness failure, not yet reviewed/promoted.
    /// A scan engine consuming rules MUST skip records carrying this
    /// status — never a live, build-gating rule.
    Proposed,
}

/// A scaffolded rule still awaiting human review/promotion. Distinct from
/// a registry-ready [`enforcer_rules::registry::RuleRecord`] so it can
/// never be mistaken for (or accidentally loaded as) a live rule — see the
/// module-level "`RuleStatus` seam" note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedRule {
    /// The typed status tag. Always [`RuleStatus::Proposed`] for output of
    /// this pipeline.
    pub status: RuleStatus,
    /// The scaffolder's full output (record + validator skeleton + fixture
    /// slots) for the candidate rule.
    pub scaffold: ScaffoldOutput,
}

/// Outcome of feeding one [`HarnessDiagnostic`] through the pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackOutcome {
    /// The mechanical classification the diagnostic received.
    pub classification: Classification,
    /// `Some` only when `classification` was
    /// [`Classification::Prevent`] AND scaffolding succeeded; a
    /// [`Classification::Detect`] diagnostic always yields `None` — no
    /// rule is ever scaffolded for a detect-only failure.
    pub proposed_rule: Option<ProposedRule>,
}

/// Versioned telemetry record for one classification decision — appended
/// via the generic `enforcer-core::ndjson_writer::NdjsonWriter` sink (this
/// crate takes no direct dependency on `enforcer-core` outside tests/dev;
/// callers own the sink and the append call, this module only shapes the
/// record). Carries the [`Sha256`] fingerprint of the input diagnostic so a
/// decision is traceable back to the exact failure that produced it
/// without re-embedding the (potentially sensitive) diagnostic text.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackDecisionRecord {
    /// Record schema version.
    pub schema_version: u32,
    /// Fixed tag identifying this record shape.
    pub event_type: String,
    /// `Sha256` fingerprint of the input diagnostic's canonical form (see
    /// [`diagnostic_fingerprint`]).
    pub input_fingerprint: Sha256,
    /// The tool that produced the original diagnostic (e.g. `cargo`).
    pub tool: String,
    /// The diagnostic's own rule id (e.g. `E0308`, `pytest`), independent
    /// of any rule id later minted for a scaffolded PROPOSED rule.
    pub source_rule_id: String,
    /// `"prevent"` or `"detect"` — the wire form of [`Classification`].
    pub classification: String,
    /// `true` iff a `ProposedRule` was scaffolded for this decision.
    pub proposed: bool,
}

/// Current schema version stamped on new [`FeedbackDecisionRecord`]s.
pub const FEEDBACK_DECISION_SCHEMA_VERSION: u32 = 1;

/// Fixed `eventType` tag for every [`FeedbackDecisionRecord`] line.
pub const FEEDBACK_DECISION_EVENT_TYPE: &str = "harnessFeedbackDecision";

/// Compute the canonical fingerprint of a diagnostic: a `Sha256` digest
/// over the same dedupe key `enforcer_harness::parsers::dedupe_diagnostics`
/// uses (`tool|ruleId|file|line|message`), so a decision record's
/// fingerprint is stable across process runs and independent of transient
/// fields like `run_id`.
// `link_digest` always emits the branded wire form (`sha256:<64 lowercase
// hex>` — see `enforcer_core::hash_chain::DIGEST_PREFIX` and its SHA-256
// hex-encoding loop), so the `expect` below can never actually fire; scoped
// `#[allow(clippy::expect_used)]` on a provably infallible parse of a
// known-well-formed value is the established pattern in this workspace
// (e.g. `enforcer-mechanization::parity::diagnostic_relpath`) for exactly
// this situation, rather than threading a fallible `Result` through every
// caller for a conversion that cannot fail given its own upstream contract.
#[allow(clippy::expect_used)]
pub fn diagnostic_fingerprint(diagnostic: &HarnessDiagnostic) -> Sha256 {
    let key = format!(
        "{}|{}|{}|{}|{}",
        diagnostic.tool, diagnostic.rule_id, diagnostic.file, diagnostic.line, diagnostic.message
    );
    let digest = enforcer_core::hash_chain::link_digest(None, key.as_bytes());
    digest
        .parse()
        .expect("enforcer_core::hash_chain::link_digest always yields a valid Sha256")
}

/// Build the [`FeedbackDecisionRecord`] for one classification decision.
/// Pure — callers append it to wherever their `enforcer-core` NDJSON sink
/// lives (this crate does not own a sink instance or a file path).
pub fn decision_record(
    diagnostic: &HarnessDiagnostic,
    classification: Classification,
    proposed: bool,
) -> FeedbackDecisionRecord {
    FeedbackDecisionRecord {
        schema_version: FEEDBACK_DECISION_SCHEMA_VERSION,
        event_type: FEEDBACK_DECISION_EVENT_TYPE.to_owned(),
        input_fingerprint: diagnostic_fingerprint(diagnostic),
        tool: diagnostic.tool.clone(),
        source_rule_id: diagnostic.rule_id.clone(),
        classification: match classification {
            Classification::Prevent => "prevent".to_owned(),
            Classification::Detect => "detect".to_owned(),
        },
        proposed,
    }
}

/// Ingest+classify one [`HarnessDiagnostic`], and for a [`Classification::Prevent`]
/// verdict, scaffold a PROPOSED rule from `spec` via [`scaffold_rule`].
///
/// `spec` is caller-supplied (rather than derived automatically from the
/// diagnostic) because turning "rustc emitted `E0308` at `src/lib.rs:12`"
/// into a well-formed [`ScaffoldSpec`] — a rule id, a validator crate/path,
/// fixture file paths, a doc anchor — requires choices (which crate should
/// own the new validator? what should the rule be called?) this module
/// does not have enough context to make mechanically; the CLASSIFICATION
/// is mechanical, the scaffold SPEC authoring is the caller's job. A
/// [`Classification::Detect`] diagnostic never reaches [`scaffold_rule`] at
/// all — `spec` is simply ignored for that branch, matching "a detect-only
/// failure produces none" from the acceptance criteria.
pub fn ingest_and_classify(
    diagnostic: &HarnessDiagnostic,
    spec: &ScaffoldSpec,
) -> MechanizationResult<FeedbackOutcome> {
    let classification = classify::classify(diagnostic);
    match classification {
        Classification::Detect => Ok(FeedbackOutcome {
            classification,
            proposed_rule: None,
        }),
        Classification::Prevent => {
            let scaffold = scaffold_rule(spec)?;
            Ok(FeedbackOutcome {
                classification,
                proposed_rule: Some(ProposedRule {
                    status: RuleStatus::Proposed,
                    scaffold,
                }),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::classify::Classification;
    use super::{
        decision_record, diagnostic_fingerprint, ingest_and_classify, RuleStatus,
        FEEDBACK_DECISION_EVENT_TYPE, FEEDBACK_DECISION_SCHEMA_VERSION,
    };
    use crate::scaffold::ScaffoldSpec;
    use enforcer_domain::severity::Tier;
    use enforcer_harness::parsers::HarnessDiagnostic;

    fn diagnostic(tool: &str, rule_id: &str) -> HarnessDiagnostic {
        HarnessDiagnostic {
            run_id: "run-1".to_owned(),
            tool: tool.to_owned(),
            language: "rust".to_owned(),
            severity: "error".to_owned(),
            rule_id: rule_id.to_owned(),
            file: "src/lib.rs".to_owned(),
            line: 12,
            message: "mismatched types".to_owned(),
            source: None,
            fingerprint: None,
        }
    }

    fn spec() -> Result<ScaffoldSpec, enforcer_core::error::DecodeError> {
        Ok(ScaffoldSpec {
            rule_id: "RR-95.1".parse()?,
            title: "No mismatched frobnication".to_owned(),
            tier: Tier::T1,
            validator_crate: "enforcer-lang-rust".to_owned(),
            validator_path: "no_frob::NoFrobValidator".to_owned(),
            fail_fixture_path: "crates/enforcer-lang-rust/fixtures/no_frob/fail.rs".to_owned(),
            pass_fixture_path: "crates/enforcer-lang-rust/fixtures/no_frob/pass.rs".to_owned(),
            doc_anchor: "docs/rules/FROB.md#FROB-2".to_owned(),
            tags: vec!["rust".to_owned()],
        })
    }

    #[test]
    fn preventable_failure_scaffolds_a_proposed_rule() -> Result<(), Box<dyn std::error::Error>> {
        let diag = diagnostic("cargo", "E0308");
        let outcome = ingest_and_classify(&diag, &spec()?)?;
        assert_eq!(outcome.classification, Classification::Prevent);
        let Some(proposed) = outcome.proposed_rule else {
            return Err("must scaffold a proposed rule".into());
        };
        assert_eq!(proposed.status, RuleStatus::Proposed);
        assert_eq!(proposed.scaffold.record.rule_id.as_str(), "RR-95.1");
        Ok(())
    }

    #[test]
    fn detect_only_failure_scaffolds_nothing() -> Result<(), Box<dyn std::error::Error>> {
        let diag = diagnostic("pytest", "pytest");
        let outcome = ingest_and_classify(&diag, &spec()?)?;
        assert_eq!(outcome.classification, Classification::Detect);
        assert!(
            outcome.proposed_rule.is_none(),
            "a detect-only failure must never produce a proposed rule"
        );
        Ok(())
    }

    #[test]
    fn fingerprint_is_stable_across_run_id_and_changes_with_content() {
        let mut a = diagnostic("cargo", "E0308");
        let mut b = a.clone();
        b.run_id = "a-totally-different-run".to_owned();
        assert_eq!(
            diagnostic_fingerprint(&a),
            diagnostic_fingerprint(&b),
            "fingerprint must not depend on the transient run id"
        );
        a.message = "a different message".to_owned();
        assert_ne!(
            diagnostic_fingerprint(&a),
            diagnostic_fingerprint(&b),
            "fingerprint must change when dedupe-key content changes"
        );
    }

    #[test]
    fn decision_record_carries_fingerprint_and_wire_shape() -> Result<(), Box<dyn std::error::Error>>
    {
        let diag = diagnostic("cargo", "E0308");
        let record = decision_record(&diag, Classification::Prevent, true);
        assert_eq!(record.schema_version, FEEDBACK_DECISION_SCHEMA_VERSION);
        assert_eq!(record.event_type, FEEDBACK_DECISION_EVENT_TYPE);
        assert_eq!(record.classification, "prevent");
        assert!(record.proposed);
        assert_eq!(record.input_fingerprint, diagnostic_fingerprint(&diag));

        // Fail-closed round trip: the record's own `Serialize`/
        // `Deserialize` impls must agree, matching every other telemetry
        // DTO in this workspace.
        let wire = serde_json::to_value(&record)?;
        assert_eq!(wire["eventType"], "harnessFeedbackDecision");
        assert_eq!(wire["classification"], "prevent");
        let back: super::FeedbackDecisionRecord = serde_json::from_value(wire)?;
        assert_eq!(back, record);
        Ok(())
    }

    #[test]
    fn detect_only_decision_record_marks_proposed_false() {
        let diag = diagnostic("pytest", "pytest");
        let record = decision_record(&diag, Classification::Detect, false);
        assert_eq!(record.classification, "detect");
        assert!(!record.proposed);
    }
}
