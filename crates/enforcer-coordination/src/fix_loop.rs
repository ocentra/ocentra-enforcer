//! d07 — the bounded self-correct fix loop.
//!
//! ADBP describes a "self-correcting" agent loop as aspiration; today the
//! enforcer only reports. This module is the first bounded step: given a
//! [`Finding`](enforcer_domain::findings::Finding) set for one file, dispatch
//! a pluggable [`dispatch::FixGenerator`], re-run the SAME
//! [`Validator`](enforcer_validator::validator::Validator) the findings came
//! from, and keep the edit only if it strictly improves (fewer findings, no
//! new [`RuleId`](enforcer_domain::ids::RuleId) introduced) — otherwise
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
//! The loop snapshots the single target file's bytes (not the whole tree —
//! `owns:` for this workpack is scoped to `fix_loop.rs`/`dispatch.rs`, and a
//! single-file snapshot is sufficient because [`dispatch::FixGenerator`]
//! implementations in this pass only ever edit the one file under
//! validation; a future multi-file generator would need a directory-level
//! snapshot, which is a natural extension point on
//! [`Snapshot`] rather than a redesign). Restore is a plain byte-for-byte
//! rewrite — deterministic, no reliance on version control being present.

pub mod dispatch;

use std::fs;
use std::path::{Path, PathBuf};

use enforcer_domain::findings::{Finding, ScanScope};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_events::event::DomainEvent;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::error::{CoordinationError, Result};
use dispatch::FixGenerator;

/// Hard bound on fix-loop iterations. Chosen generously enough to let a few
/// genuinely improving fixes land in sequence, small enough that a
/// degenerate generator (one that keeps making no-op-equivalent edits that
/// each still count as "changed") cannot spin unboundedly.
pub const MAX_ITERATIONS: u32 = 8;

/// Byte-for-byte snapshot of one file, taken before a fix attempt so it can
/// be restored verbatim if the attempt does not improve things.
struct Snapshot {
    path: PathBuf,
    bytes: Vec<u8>,
}

impl Snapshot {
    fn capture(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            bytes,
        })
    }

    fn restore(&self) -> Result<()> {
        fs::write(&self.path, &self.bytes)?;
        Ok(())
    }
}

/// One iteration's outcome, exposed on [`FixLoopReport`] so callers/tests can
/// inspect the full trajectory, not just the final state.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IterationOutcome {
    /// 1-based iteration number.
    pub iteration: u32,
    /// Finding count immediately before this iteration's attempt.
    pub findings_before: usize,
    /// Finding count after the attempt (before any revert).
    pub findings_after: usize,
    /// Whether the edit was kept (`true`) or reverted (`false`).
    pub accepted: bool,
    /// Why this outcome happened, for observability.
    pub reason: IterationReason,
}

/// Why an iteration ended the way it did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IterationReason {
    /// Findings strictly decreased and no new `RuleId` appeared.
    Improved,
    /// The generator declined to act (`attempt_fix` returned `false`).
    GeneratorDeclined,
    /// The edit did not strictly reduce findings (neutral or regressing).
    NotImproved,
    /// The edit strictly reduced count but introduced a `RuleId` that was
    /// not present before — still rejected: "strictly decrease AND no new
    /// `RuleId`" is a conjunction, not an either/or.
    NewRuleIdIntroduced,
}

/// Full result of running the fix loop to completion (either an iteration
/// stopped improving, the generator declined, or the cap was hit).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixLoopReport {
    /// Finding count before the very first iteration.
    pub findings_start: usize,
    /// Finding count in the final, kept state.
    pub findings_final: usize,
    /// Per-iteration trajectory, in order.
    pub iterations: Vec<IterationOutcome>,
    /// True if the loop stopped because it hit [`MAX_ITERATIONS`] while
    /// still improving on every prior iteration (not because it plateaued).
    pub hit_iteration_cap: bool,
}

/// A typed coordination event for one fix-loop accept/revert decision,
/// carried through [`enforcer_events`] and mirrored to the d04 telemetry
/// NDJSON sink by callers that want a durable record (this module emits the
/// event; wiring it to a live sink is the caller's job — see
/// `enforcer-cli`'s `fix` command, d06).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixLoopDecisionEvent {
    pub generator_name: String,
    pub iteration: u32,
    pub findings_before: usize,
    pub findings_after: usize,
    pub accepted: bool,
    pub reason: IterationReason,
}

impl DomainEvent for FixLoopDecisionEvent {
    fn event_kind(&self) -> &'static str {
        "coordination.fix_loop.decision"
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
///   reached — whichever comes first. Every accept/revert decision is
///   reported via `on_decision` as a typed [`FixLoopDecisionEvent`] so
///   callers can log/telemeter without this module taking a direct
///   dependency on any particular sink.
#[allow(clippy::too_many_arguments)]
pub fn run_fix_loop(
    file_path: &Path,
    rel_path: &RelPath,
    initial_findings: Vec<Finding>,
    validator: &dyn Validator,
    generator: &dyn FixGenerator,
    mut on_decision: impl FnMut(&FixLoopDecisionEvent),
) -> Result<FixLoopReport> {
    let findings_start = initial_findings.len();
    let mut current_findings = initial_findings;
    let mut iterations = Vec::new();
    let mut hit_iteration_cap = false;

    for iteration in 1..=MAX_ITERATIONS {
        let findings_before = current_findings.len();
        let rule_ids_before: std::collections::BTreeSet<RuleId> = current_findings
            .iter()
            .map(|finding| finding.rule_id.clone())
            .collect();

        let snapshot = Snapshot::capture(file_path)?;

        let changed =
            generator.attempt_fix(file_path.parent().unwrap_or(file_path), &current_findings)?;
        if !changed {
            iterations.push(IterationOutcome {
                iteration,
                findings_before,
                findings_after: findings_before,
                accepted: false,
                reason: IterationReason::GeneratorDeclined,
            });
            emit(
                &mut on_decision,
                generator,
                iteration,
                findings_before,
                findings_before,
                false,
                IterationReason::GeneratorDeclined,
            );
            break;
        }

        let source_after = fs::read_to_string(file_path)?;
        let rescanned = validator.validate(ValidationInput {
            file: rel_path,
            source: &source_after,
            scope: ScanScope::Files,
        });
        let findings_after = rescanned.len();
        let rule_ids_after: std::collections::BTreeSet<RuleId> = rescanned
            .iter()
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
        let accepted = matches!(reason, IterationReason::Improved);

        if !accepted {
            snapshot.restore()?;
        }

        iterations.push(IterationOutcome {
            iteration,
            findings_before,
            findings_after,
            accepted,
            reason,
        });
        emit(
            &mut on_decision,
            generator,
            iteration,
            findings_before,
            findings_after,
            accepted,
            reason,
        );

        if !accepted {
            break;
        }

        current_findings = rescanned;

        if iteration == MAX_ITERATIONS {
            hit_iteration_cap = true;
        }
    }

    let findings_final = current_findings.len();
    if findings_final > findings_start {
        return Err(CoordinationError::rejected(format!(
            "fix loop invariant violated: final findings ({findings_final}) exceed start ({findings_start})"
        )));
    }

    Ok(FixLoopReport {
        findings_start,
        findings_final,
        iterations,
        hit_iteration_cap,
    })
}

#[allow(clippy::too_many_arguments)]
fn emit(
    on_decision: &mut impl FnMut(&FixLoopDecisionEvent),
    generator: &dyn FixGenerator,
    iteration: u32,
    findings_before: usize,
    findings_after: usize,
    accepted: bool,
    reason: IterationReason,
) {
    let event = FixLoopDecisionEvent {
        generator_name: generator.name().to_owned(),
        iteration,
        findings_before,
        findings_after,
        accepted,
        reason,
    };
    on_decision(&event);
}

#[cfg(test)]
mod tests {
    use enforcer_domain::severity::Severity;

    use super::*;

    /// Validator that counts occurrences of the literal marker `BAD` in the
    /// source, one finding per occurrence, all under the same `RuleId`.
    struct MarkerValidator {
        rule_id: RuleId,
    }

    impl Validator for MarkerValidator {
        fn rule_id(&self) -> &RuleId {
            &self.rule_id
        }

        fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
            input
                .source
                .match_indices("BAD")
                .enumerate()
                .map(|(idx, _)| Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "bad marker".to_owned(),
                    detail: "found BAD".to_owned(),
                    file: input.file.clone(),
                    line: (idx as u32) + 1,
                    snippet: None,
                })
                .collect()
        }
    }

    fn rel_path() -> Result<RelPath> {
        Ok("a.txt".parse()?)
    }

    fn findings_for(rule_id: &RuleId, count: usize) -> Result<Vec<Finding>> {
        let file = rel_path()?;
        Ok((0..count)
            .map(|idx| Finding {
                rule_id: rule_id.clone(),
                severity: Severity::Error,
                title: "bad marker".to_owned(),
                detail: "found BAD".to_owned(),
                file: file.clone(),
                line: (idx as u32) + 1,
                snippet: None,
            })
            .collect())
    }

    /// Removes exactly one `BAD` occurrence per attempt — an improving fix.
    struct OneAtATimeRemover;
    impl FixGenerator for OneAtATimeRemover {
        fn attempt_fix(&self, root: &Path, findings: &[Finding]) -> Result<bool> {
            if findings.is_empty() {
                return Ok(false);
            }
            let path = root.join("a.txt");
            let content = fs::read_to_string(&path)?;
            if let Some(pos) = content.find("BAD") {
                let mut new_content = content.clone();
                new_content.replace_range(pos..pos + 3, "OK_");
                fs::write(&path, new_content)?;
                Ok(true)
            } else {
                Ok(false)
            }
        }

        fn name(&self) -> &str {
            "one-at-a-time-remover"
        }
    }

    /// Rewrites the file to something unrelated that still has the same
    /// finding count — a neutral (non-improving) fix.
    struct NeutralRewriter;
    impl FixGenerator for NeutralRewriter {
        fn attempt_fix(&self, root: &Path, findings: &[Finding]) -> Result<bool> {
            if findings.is_empty() {
                return Ok(false);
            }
            fs::write(root.join("a.txt"), "BAD BAD totally rewritten")?;
            Ok(true)
        }

        fn name(&self) -> &str {
            "neutral-rewriter"
        }
    }

    /// Adds MORE `BAD` occurrences — a regressing fix.
    struct RegressingWriter;
    impl FixGenerator for RegressingWriter {
        fn attempt_fix(&self, root: &Path, findings: &[Finding]) -> Result<bool> {
            if findings.is_empty() {
                return Ok(false);
            }
            let path = root.join("a.txt");
            let content = fs::read_to_string(&path)?;
            fs::write(&path, format!("{content} BAD BAD"))?;
            Ok(true)
        }

        fn name(&self) -> &str {
            "regressing-writer"
        }
    }

    /// A generator that always claims success but never actually edits the
    /// file — proves the loop's re-scan gate, not the generator's return
    /// value, decides acceptance.
    struct LyingGenerator;
    impl FixGenerator for LyingGenerator {
        fn attempt_fix(&self, _root: &Path, findings: &[Finding]) -> Result<bool> {
            Ok(!findings.is_empty())
        }

        fn name(&self) -> &str {
            "lying-generator"
        }
    }

    fn setup(dir: &Path, content: &str) -> Result<PathBuf> {
        let path = dir.join("a.txt");
        fs::write(&path, content)?;
        Ok(path)
    }

    fn marker_rule_id() -> Result<RuleId> {
        Ok("RR-7.1".parse()?)
    }

    #[test]
    fn improving_fix_is_kept_across_iterations() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let file = setup(dir.path(), "BAD BAD BAD")?;
        let rule_id = marker_rule_id()?;
        let validator = MarkerValidator {
            rule_id: rule_id.clone(),
        };
        let generator = OneAtATimeRemover;
        let mut events = Vec::new();

        let report = run_fix_loop(
            &file,
            &rel_path()?,
            findings_for(&rule_id, 3)?,
            &validator,
            &generator,
            |event| events.push(event.clone()),
        )?;

        assert_eq!(report.findings_start, 3);
        assert_eq!(report.findings_final, 0);
        assert!(!report.hit_iteration_cap);
        assert!(report
            .iterations
            .iter()
            .all(|it| it.accepted || it.reason != IterationReason::Improved));
        assert_eq!(fs::read_to_string(&file)?, "OK_ OK_ OK_");
        assert!(events
            .iter()
            .all(|event| event.generator_name == "one-at-a-time-remover"));
        Ok(())
    }

    #[test]
    fn neutral_fix_is_reverted_and_not_kept() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let original = "BAD BAD";
        let file = setup(dir.path(), original)?;
        let rule_id = marker_rule_id()?;
        let validator = MarkerValidator {
            rule_id: rule_id.clone(),
        };
        let generator = NeutralRewriter;

        let report = run_fix_loop(
            &file,
            &rel_path()?,
            findings_for(&rule_id, 2)?,
            &validator,
            &generator,
            |_| {},
        )?;

        assert_eq!(report.findings_start, 2);
        assert_eq!(report.findings_final, 2);
        assert_eq!(report.iterations.len(), 1);
        assert!(!report.iterations[0].accepted);
        assert_eq!(report.iterations[0].reason, IterationReason::NotImproved);
        // File must be restored to the exact original bytes.
        assert_eq!(fs::read_to_string(&file)?, original);
        Ok(())
    }

    #[test]
    fn regressing_fix_is_reverted_and_not_kept() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let original = "BAD";
        let file = setup(dir.path(), original)?;
        let rule_id = marker_rule_id()?;
        let validator = MarkerValidator {
            rule_id: rule_id.clone(),
        };
        let generator = RegressingWriter;

        let report = run_fix_loop(
            &file,
            &rel_path()?,
            findings_for(&rule_id, 1)?,
            &validator,
            &generator,
            |_| {},
        )?;

        assert_eq!(report.findings_final, report.findings_start);
        assert_eq!(report.iterations.len(), 1);
        assert!(!report.iterations[0].accepted);
        assert_eq!(report.iterations[0].reason, IterationReason::NotImproved);
        assert_eq!(fs::read_to_string(&file)?, original);
        Ok(())
    }

    #[test]
    fn loop_halts_at_iteration_cap_when_always_improving() -> Result<()> {
        let dir = tempfile::tempdir()?;
        // More BAD markers than MAX_ITERATIONS can clear one-at-a-time.
        let many = "BAD ".repeat((MAX_ITERATIONS as usize) + 5);
        let file = setup(dir.path(), many.trim())?;
        let validator = MarkerValidator {
            rule_id: marker_rule_id()?,
        };
        let generator = OneAtATimeRemover;
        let rel = rel_path()?;
        let start_count = validator.validate(ValidationInput {
            file: &rel,
            source: many.trim(),
            scope: ScanScope::Files,
        });

        let report = run_fix_loop(&file, &rel, start_count, &validator, &generator, |_| {})?;

        assert!(report.hit_iteration_cap);
        assert_eq!(report.iterations.len(), MAX_ITERATIONS as usize);
        assert!(report.iterations.iter().all(|it| it.accepted));
        assert!(report.findings_final < report.findings_start);
        Ok(())
    }

    #[test]
    fn generator_decline_halts_the_loop_immediately() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let file = setup(dir.path(), "clean file, no marker")?;
        let validator = MarkerValidator {
            rule_id: marker_rule_id()?,
        };
        let generator = OneAtATimeRemover;

        let report = run_fix_loop(
            &file,
            &rel_path()?,
            Vec::new(),
            &validator,
            &generator,
            |_| {},
        )?;

        assert_eq!(report.iterations.len(), 1);
        assert_eq!(
            report.iterations[0].reason,
            IterationReason::GeneratorDeclined
        );
        assert_eq!(report.findings_final, 0);
        Ok(())
    }

    #[test]
    fn final_state_never_has_more_findings_than_the_start() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let original = "BAD";
        let file = setup(dir.path(), original)?;
        let rule_id = marker_rule_id()?;
        let validator = MarkerValidator {
            rule_id: rule_id.clone(),
        };
        let generator = RegressingWriter;

        let report = run_fix_loop(
            &file,
            &rel_path()?,
            findings_for(&rule_id, 1)?,
            &validator,
            &generator,
            |_| {},
        )?;

        assert!(report.findings_final <= report.findings_start);
        Ok(())
    }

    #[test]
    fn a_generator_that_lies_about_changing_anything_is_still_gated_by_rescan() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let original = "BAD";
        let file = setup(dir.path(), original)?;
        let rule_id = marker_rule_id()?;
        let validator = MarkerValidator {
            rule_id: rule_id.clone(),
        };
        let generator = LyingGenerator;

        let report = run_fix_loop(
            &file,
            &rel_path()?,
            findings_for(&rule_id, 1)?,
            &validator,
            &generator,
            |_| {},
        )?;

        // The generator claimed `true` (a change happened) but never wrote
        // anything; the re-scan sees an unchanged file (findings_after ==
        // findings_before), which is NOT a strict improvement, so it must be
        // treated as not-accepted and the file content is untouched.
        assert_eq!(report.iterations.len(), 1);
        assert!(!report.iterations[0].accepted);
        assert_eq!(fs::read_to_string(&file)?, original);
        Ok(())
    }
}
