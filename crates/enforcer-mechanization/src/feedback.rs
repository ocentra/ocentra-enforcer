//! Harness feedback pipeline (d08): turn an escaped harness failure into a
//! candidate rule instead of letting the lesson die in a chat log.
//!
//! `enforcer-harness` (arc-18) already parses native-tool output into
//! [`enforcer_harness::parsers::HarnessDiagnostic`] records; this module is
//! the missing link ADBP's "close the loop" prose pointed at:
//!
//! 1. Ingest a diagnostic (typed, already produced by `enforcer-harness` —
//!    this module does not re-parse raw tool output).
//! 2. Classify it [`MechanizationClassification::Prevent`] vs
//!    [`MechanizationClassification::Detect`] via [`classify::classify`] —
//!    mechanical field matching, never an LLM judgment call.
//! 3. For `Prevent`, call [`crate::scaffold::scaffold_rule`] (the d01
//!    scaffolder already in this crate) to emit a validator skeleton + doc
//!    anchor + fixture slots, wrapped as a [`ProposedRule`] carrying a
//!    typed, non-blocking [`RuleLifecycleStatus::Proposed`] tag.
//! 4. Log the classification decision as a [`FeedbackDecisionRecord`]
//!    (versioned serde struct, carrying the input's [`Sha256`] fingerprint)
//!    appendable via the `enforcer-core` NDJSON sink.
//!
//! ## `RuleLifecycleStatus` seam (documented, not silently absorbed)
//! [`enforcer_rules::registry::RuleRecord`] has no `status` field yet — that
//! field is out of this workpack's `owns:` (it lives in `enforcer-rules`,
//! owned by d01/arc-14). [`ProposedRule`] carries the typed status
//! ALONGSIDE the record rather than mutating the shared registry DTO, so a
//! future pass can fold `RuleLifecycleStatus` directly onto `RuleRecord` without this
//! module needing to change: [`ProposedRule::record`] is exactly the
//! `RuleRecord` the scaffolder would have produced standalone. "PROPOSED
//! rules are non-blocking in a scan" is proven at THIS module's boundary —
//! a `ProposedRule` is a distinct type from a registry-ready `RuleRecord`,
//! so nothing in this crate's own output can reach a scan engine's
//! `RuleRegistry` without an explicit, separate promotion step that is
//! itself out of scope here.

pub mod boundary;
pub mod classify;

use enforcer_domain::events_types::EventType;
use enforcer_domain::hashes::Sha256;
use enforcer_domain::mechanization_types::{
    ExternalDiagnosticCode, FeedbackDecisionSchemaVersion, FeedbackScaffoldState, FeedbackToolName,
    MechanizationClassification, RuleLifecycleStatus,
};
use enforcer_harness::parsers::HarnessDiagnostic;

use crate::error::MechanizationResult;
use crate::scaffold::{scaffold_rule, ScaffoldOutput, ScaffoldSpec};

/// Typed, machine-readable rule-record status. Closed set: only
/// [`RuleLifecycleStatus::Proposed`] is produced by this pipeline today, but the
/// enum exists (rather than a bare bool) so a later `Promoted`/`Rejected`
/// variant is a non-breaking addition, matching the workpack's "a
/// `Tier`/status field ... not prose" requirement.
/// A scaffolded rule still awaiting human review/promotion. Distinct from
/// a registry-ready [`enforcer_rules::registry::RuleRecord`] so it can
/// never be mistaken for (or accidentally loaded as) a live rule — see the
/// module-level "`RuleLifecycleStatus` seam" note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedRule {
    /// The typed status tag. Always [`RuleLifecycleStatus::Proposed`] for output of
    /// this pipeline.
    pub status: RuleLifecycleStatus,
    /// The scaffolder's full output (record + validator skeleton + fixture
    /// slots) for the candidate rule.
    pub scaffold: ScaffoldOutput,
}

/// Outcome of feeding one [`HarnessDiagnostic`] through the pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackOutcome {
    /// The mechanical classification the diagnostic received.
    pub classification: MechanizationClassification,
    /// `Some` only when `classification` was
    /// [`MechanizationClassification::Prevent`] AND scaffolding succeeded; a
    /// [`MechanizationClassification::Detect`] diagnostic always yields `None` — no
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackDecisionRecord {
    /// Record schema version.
    schema_version: FeedbackDecisionSchemaVersion,
    /// Fixed tag identifying this record shape.
    event_type: EventType,
    /// `Sha256` fingerprint of the input diagnostic's canonical form (see
    /// [`diagnostic_fingerprint`]).
    input_fingerprint: Sha256,
    /// The tool that produced the original diagnostic (e.g. `cargo`).
    tool: FeedbackToolName,
    /// The diagnostic's own rule id (e.g. `E0308`, `pytest`), independent
    /// of any rule id later minted for a scaffolded PROPOSED rule.
    source_rule_id: ExternalDiagnosticCode,
    /// `"prevent"` or `"detect"` — the wire form of [`MechanizationClassification`].
    classification: MechanizationClassification,
    /// `true` iff a `ProposedRule` was scaffolded for this decision.
    proposed: FeedbackScaffoldState,
}

impl FeedbackDecisionRecord {
    /// Return the feedback decision schema version.
    pub const fn schema_version(&self) -> FeedbackDecisionSchemaVersion {
        self.schema_version
    }

    /// Return the canonical event type for this decision record.
    pub fn event_type(&self) -> &EventType {
        &self.event_type
    }

    /// Return the fingerprint of the source diagnostic.
    pub fn input_fingerprint(&self) -> &Sha256 {
        &self.input_fingerprint
    }

    /// Return the tool that emitted the source diagnostic.
    pub fn tool(&self) -> &FeedbackToolName {
        &self.tool
    }

    /// Return the external diagnostic code that informed the decision.
    pub fn source_rule_id(&self) -> &ExternalDiagnosticCode {
        &self.source_rule_id
    }

    /// Return the mechanical prevent-or-detect classification.
    pub const fn classification(&self) -> MechanizationClassification {
        self.classification
    }

    /// Return whether the decision produced a scaffold candidate.
    pub const fn scaffold_state(&self) -> FeedbackScaffoldState {
        self.proposed
    }
}

/// Current schema version stamped on new [`FeedbackDecisionRecord`]s.
pub const FEEDBACK_DECISION_SCHEMA_VERSION: FeedbackDecisionSchemaVersion =
    FeedbackDecisionSchemaVersion::initial();

/// Compute the canonical fingerprint of a diagnostic: a `Sha256` digest
/// over the same dedupe key `enforcer_harness::parsers::dedupe_diagnostics`
/// uses (`tool|ruleId|file|line|message`), so a decision record's
/// fingerprint is stable across process runs and independent of transient
/// fields like `run_id`. Parsing remains explicit and fallible at this hash
/// boundary even though `link_digest` currently emits canonical SHA-256 text.
pub fn diagnostic_fingerprint(
    diagnostic: &HarnessDiagnostic,
) -> Result<Sha256, enforcer_domain::boundary::decode_error::DecodeError> {
    let key = format!(
        "{}|{}|{}|{}|{}",
        diagnostic.tool, diagnostic.rule_id, diagnostic.file, diagnostic.line, diagnostic.message
    );
    let digest = enforcer_core::hash_chain::link_digest(None, key.as_bytes());
    Ok(digest)
}

/// Build the [`FeedbackDecisionRecord`] for one classification decision.
/// Pure — callers append it to wherever their `enforcer-core` NDJSON sink
/// lives (this crate does not own a sink instance or a file path).
pub fn decision_record(
    diagnostic: &HarnessDiagnostic,
    classification: MechanizationClassification,
) -> Result<FeedbackDecisionRecord, enforcer_domain::boundary::decode_error::DecodeError> {
    let proposed = match classification {
        MechanizationClassification::Prevent => FeedbackScaffoldState::Proposed,
        MechanizationClassification::Detect => FeedbackScaffoldState::NotProposed,
    };
    Ok(FeedbackDecisionRecord {
        schema_version: FEEDBACK_DECISION_SCHEMA_VERSION,
        event_type: boundary::feedback_decision_event_type()?,
        input_fingerprint: diagnostic_fingerprint(diagnostic)?,
        // ALLOC-JUSTIFICATION: the decision record owns the tool identity after the diagnostic borrow ends.
        tool: FeedbackToolName::try_new(diagnostic.tool.as_str().to_owned())?,
        // ALLOC-JUSTIFICATION: the decision record owns the diagnostic code after the diagnostic borrow ends.
        source_rule_id: ExternalDiagnosticCode::try_new(diagnostic.rule_id.as_str().to_owned())?,
        classification,
        proposed,
    })
}

/// Ingest+classify one [`HarnessDiagnostic`], and for a [`MechanizationClassification::Prevent`]
/// verdict, scaffold a PROPOSED rule from `spec` via [`scaffold_rule`].
///
/// `spec` is caller-supplied (rather than derived automatically from the
/// diagnostic) because turning "rustc emitted `E0308` at `src/lib.rs:12`"
/// into a well-formed [`ScaffoldSpec`] — a rule id, a validator crate/path,
/// fixture file paths, a doc anchor — requires choices (which crate should
/// own the new validator? what should the rule be called?) this module
/// does not have enough context to make mechanically; the CLASSIFICATION
/// is mechanical, the scaffold SPEC authoring is the caller's job. A
/// [`MechanizationClassification::Detect`] diagnostic never reaches [`scaffold_rule`] at
/// all — `spec` is simply ignored for that branch, matching "a detect-only
/// failure produces none" from the acceptance criteria.
pub fn ingest_and_classify(
    diagnostic: &HarnessDiagnostic,
    spec: &ScaffoldSpec,
) -> MechanizationResult<FeedbackOutcome> {
    let classification = classify::classify(diagnostic);
    match classification {
        MechanizationClassification::Detect => Ok(FeedbackOutcome {
            classification,
            proposed_rule: None,
        }),
        MechanizationClassification::Prevent => {
            let scaffold = scaffold_rule(spec)?;
            Ok(FeedbackOutcome {
                classification,
                proposed_rule: Some(ProposedRule {
                    status: RuleLifecycleStatus::Proposed,
                    scaffold,
                }),
            })
        }
    }
}
