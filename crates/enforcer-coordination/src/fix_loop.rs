//! d07 â€” the bounded self-correct fix loop.
//!
//! ADBP describes a "self-correcting" agent loop as aspiration; today the
//! enforcer only reports. This module is the first bounded step: given a
//! [`Finding`](enforcer_domain::findings::Finding) set for one file, dispatch
//! a pluggable [`dispatch::FixGenerator`], re-run the SAME
//! [`Validator`](enforcer_validator::validator::Validator) the findings came
//! from, and keep the edit only if it strictly improves (fewer findings, no
//! new [`RuleId`](enforcer_domain::ids::RuleId) introduced) â€” otherwise
//! revert the working tree to the snapshot taken before the attempt. A hard
//! iteration cap guarantees termination regardless of generator behavior.
//!
//! # Why re-scan rather than trust the generator
//! A fix generator's own claim that it "fixed" something is exactly the
//! kind of unverified self-report the enforcer doctrine forbids elsewhere
//! (rules are proven by re-running the validator, not by asking the model).
//! The same discipline applies here: every accept/revert decision is made by
//! diffing two REAL `Validator::validate` calls (before vs. after), never by
//! trusting [`dispatch::FixGenerator::attempt_fix`]'s return value beyond
//! "did you touch anything".
//!
//! # Snapshot/restore
//! The loop snapshots the single target file's bytes (not the whole tree â€”
//! `owns:` for this workpack is scoped to `fix_loop.rs`/`dispatch.rs`, and a
//! single-file snapshot is sufficient because [`dispatch::FixGenerator`]
//! implementations in this pass only ever edit the one file under
//! validation; a future multi-file generator would need a directory-level
//! snapshot, which is a natural extension point on
//! [`Snapshot`] rather than a redesign). Restore is a plain byte-for-byte
//! rewrite â€” deterministic, no reliance on version control being present.

pub mod boundary;
pub mod dispatch;

use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::coordination_types::{
    CoordinationRejection, FindingCount, FixAcceptance, FixAttemptOutcome, FixGeneratorName,
    FixIteration, FixTargetPath, IterationCapStatus, IterationReason,
};
use enforcer_domain::events_types::EventType;
use enforcer_domain::findings::{Finding, ScanScope};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_events::event::DomainEvent;
use enforcer_validator::validator::{ValidationInput, Validator};
use std::fs;

use crate::error::{CoordinationError, Result};
use dispatch::FixGenerator;

/// Hard bound on fix-loop iterations. Chosen generously enough to let a few
/// genuinely improving edits land in sequence, small enough that a
/// degenerate generator (one that keeps making no-op-equivalent edits that
/// each still count as "changed") cannot spin unboundedly.
pub const MAX_ITERATIONS: u32 = 8;

/// Byte-for-byte snapshot of one file, taken before a fix attempt so it can
/// be restored verbatim if the attempt does not improve things.
struct Snapshot<'a> {
    path: &'a FixTargetPath,
    bytes: SnapshotBytes,
}

struct SnapshotBytes(Box<[u8]>);

impl Snapshot<'_> {
    fn capture(path: &FixTargetPath) -> Result<Snapshot<'_>> {
        let bytes = SnapshotBytes(fs::read(path.as_path())?.into_boxed_slice());
        Ok(Snapshot { path, bytes })
    }

    fn restore(&self) -> Result<()> {
        fs::write(self.path.as_path(), &self.bytes.0)?;
        Ok(())
    }
}

/// One iteration's outcome, exposed on [`FixLoopReport`] so callers/tests can
/// inspect the full trajectory, not just the final state.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Validated outcome of one bounded fix-loop iteration."]
pub struct IterationOutcome {
    /// 1-based iteration number.
    pub iteration: FixIteration,
    /// Finding count immediately before this iteration's attempt.
    pub findings_before: FindingCount,
    /// Finding count after the attempt (before any revert).
    pub findings_after: FindingCount,
    /// Whether the edit was kept (`true`) or reverted (`false`).
    pub accepted: FixAcceptance,
    /// Why this outcome happened, for observability.
    pub reason: IterationReason,
}

/// not present before â€” still rejected: "strictly decrease AND no new
/// `RuleId`" is a conjunction, not an either/or.
/// Full result of running the fix loop to completion (either an iteration
/// stopped improving, the generator declined, or the cap was hit).
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Validated final report produced by the bounded fix loop."]
pub struct FixLoopReport {
    /// Finding count before the very first iteration.
    pub findings_start: FindingCount,
    /// Finding count in the final, kept state.
    pub findings_final: FindingCount,
    /// Per-iteration trajectory, in order.
    pub iterations: Vec<IterationOutcome>,
    /// True if the loop stopped because it hit [`MAX_ITERATIONS`] while
    /// still improving on every prior iteration (not because it plateaued).
    pub hit_iteration_cap: IterationCapStatus,
}

/// A typed coordination event for one fix-loop accept/revert decision,
/// carried through [`enforcer_events`] and mirrored to the d04 telemetry
/// NDJSON sink by callers that want a durable record (this module emits the
/// event; wiring it to a live sink is the caller's job â€” see
/// `enforcer-cli`'s `fix` command, d06).
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Typed domain event describing one validated fix-loop decision."]
pub struct FixLoopDecisionEvent {
    pub generator_name: FixGeneratorName,
    pub iteration: FixIteration,
    pub findings_before: FindingCount,
    pub findings_after: FindingCount,
    pub accepted: FixAcceptance,
    pub reason: IterationReason,
}

impl serde::Serialize for FixLoopDecisionEvent {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        crate::fix_loop::boundary::FixLoopDecisionEventResponse::from(self).serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for FixLoopDecisionEvent {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let response =
            crate::fix_loop::boundary::FixLoopDecisionEventResponse::deserialize(deserializer)?;
        Self::try_from(response).map_err(serde::de::Error::custom)
    }
}

impl DomainEvent for FixLoopDecisionEvent {
    fn event_kind(
        &self,
    ) -> std::result::Result<EventType, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(EventType::coordination_fix_loop_decision())
    }
}

/// Run the bounded self-correct loop for one file's findings.
///
/// - `validator` re-checks `file` after every attempt; it MUST be the same
///   validator (same `RuleId`) that produced `initial_findings`, or the
///   "no new `RuleId`" comparison is meaningless.
/// - `generator` is invoked once per iteration; its edit is snapshotted
///   before and compared after via a fresh `validator.validate` call, never
///   trusted directly.
/// - Terminates when: the generator declines to act, an attempt does not
///   strictly improve (see [`IterationReason`]), or [`MAX_ITERATIONS`] is
///   reached â€” whichever comes first. Every accept/revert decision is
///   reported via `on_decision` as a typed [`FixLoopDecisionEvent`] so
///   callers can log/telemeter without this module taking a direct
///   dependency on any particular sink.
pub fn run_fix_loop<'a>(
    file_path: &FixTargetPath,
    rel_path: &RelPath,
    initial_findings: Vec<Finding>,
    engines: FixEngines<'a>,
    mut on_decision: impl FnMut(&FixLoopDecisionEvent),
) -> Result<FixLoopReport> {
    let validator = engines.validator;
    let generator = engines.generator;
    let findings_start = FindingCount::from_collection(&initial_findings);
    let mut current_findings = initial_findings;
    let mut iterations = Vec::new();
    let mut cap_status = IterationCapStatus::NotReached;

    for iteration_number in 1..=MAX_ITERATIONS {
        let iteration =
            FixIteration::new(std::num::NonZeroU32::new(iteration_number).ok_or_else(|| {
                enforcer_domain::boundary::decode_error::DecodeError::new(
                    "fixIteration",
                    "expected a positive iteration",
                )
            })?);
        let findings_before = FindingCount::from_collection(&current_findings);
        let rule_ids_before: std::collections::BTreeSet<RuleId> = current_findings
            .iter()
            // CLONE-JUSTIFICATION: the before-set owns rule identifiers for
            // comparison after validation releases the current findings.
            .map(|finding| finding.rule_id.clone())
            .collect();

        let snapshot = Snapshot::capture(file_path)?;

        let root = file_path.parent_root()?;
        let changed = generator.attempt_fix(&root, &current_findings)?;
        if matches!(changed, FixAttemptOutcome::Declined) {
            let outcome = IterationOutcome {
                iteration,
                findings_before,
                findings_after: findings_before,
                accepted: FixAcceptance::Reverted,
                reason: IterationReason::GeneratorDeclined,
            };
            emit(&mut on_decision, generator, &outcome)?;
            iterations.push(outcome);
            break;
        }

        let source_after = match fs::read_to_string(file_path.as_path()) {
            Ok(source) => source,
            Err(error) => {
                // An unreadable candidate cannot be validated, so it is
                // rejected exactly like any other non-improving attempt.
                snapshot.restore()?;
                return Err(error.into());
            }
        };
        let rescanned = validator.validate(ValidationInput {
            file: rel_path,
            source: ValidationSource::from_text(&source_after),
            scope: ScanScope::Files,
        });
        let findings_after = FindingCount::from_collection(&rescanned);
        let rule_ids_after: std::collections::BTreeSet<RuleId> = rescanned
            .iter()
            // CLONE-JUSTIFICATION: the after-set survives independently for
            // the new-rule comparison after the validator result is consumed.
            .map(|finding| finding.rule_id.clone())
            .collect();
        let introduced_new_rule = rule_ids_after.difference(&rule_ids_before).next().is_some();

        let reason = if introduced_new_rule {
            IterationReason::NewRuleIdIntroduced
        } else if findings_after < findings_before {
            IterationReason::Improved
        } else {
            IterationReason::NotImproved
        };
        let accepted = if matches!(reason, IterationReason::Improved) {
            FixAcceptance::Accepted
        } else {
            FixAcceptance::Reverted
        };

        if matches!(accepted, FixAcceptance::Reverted) {
            snapshot.restore()?;
        }

        let outcome = IterationOutcome {
            iteration,
            findings_before,
            findings_after,
            accepted,
            reason,
        };
        emit(&mut on_decision, generator, &outcome)?;
        iterations.push(outcome);

        if matches!(accepted, FixAcceptance::Reverted) {
            break;
        }

        current_findings = rescanned;

        if iteration_number == MAX_ITERATIONS {
            cap_status = IterationCapStatus::Reached;
        }
    }

    let findings_final = FindingCount::from_collection(&current_findings);
    if findings_final > findings_start {
        let reason = CoordinationRejection::from_display(&format_args!(
            "fix loop invariant violated: final findings ({}) exceed start ({})",
            findings_final, findings_start
        ))?;
        return Err(CoordinationError::rejected(reason));
    }

    Ok(FixLoopReport {
        findings_start,
        findings_final,
        iterations,
        hit_iteration_cap: cap_status,
    })
}

/// Validator and generator pair used by one bounded fix-loop execution.
#[derive(Clone, Copy)]
pub struct FixEngines<'a> {
    pub validator: &'a dyn Validator,
    pub generator: &'a dyn FixGenerator,
}

impl<'a> FixEngines<'a> {
    /// Pair one validator with one fix generator for a bounded execution.
    pub fn new(validator: &'a dyn Validator, generator: &'a dyn FixGenerator) -> Self {
        Self {
            validator,
            generator,
        }
    }
}

impl std::fmt::Debug for FixEngines<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FixEngines")
            .field("validator", &"dyn Validator")
            .field("generator", &"dyn FixGenerator")
            .finish()
    }
}

fn emit(
    on_decision: &mut impl FnMut(&FixLoopDecisionEvent),
    generator: &dyn FixGenerator,
    outcome: &IterationOutcome,
) -> Result<()> {
    let event = FixLoopDecisionEvent {
        // ALLOC-JUSTIFICATION: decision events are owned telemetry records,
        // so they retain the generator identity after the generator borrow.
        generator_name: generator.name()?,
        iteration: outcome.iteration,
        findings_before: outcome.findings_before,
        findings_after: outcome.findings_after,
        accepted: outcome.accepted,
        reason: outcome.reason,
    };
    on_decision(&event);
    Ok(())
}
