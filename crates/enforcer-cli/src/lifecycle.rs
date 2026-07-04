//! d06 lifecycle commands: `plan | implement | check | fix | review`, a
//! clap subcommand family where every phase's pass/fail is decided by a
//! Rust oracle ([`oracle`]), never by prose or model self-report.
//!
//! # Dispatch table, not a match-per-caller
//! [`Phase`] enumerates the five phases; [`run_phase`] is the ONE dispatch
//! function mapping a `Phase` to its oracle and folding the resulting
//! [`oracle::PhaseVerdict`] into an [`enforcer_core::exit_codes::ExitCode`].
//! A phase failure always yields a non-zero exit — there is no phase that
//! can report [`enforcer_core::exit_codes::ExitCode::Success`] while its
//! oracle returned [`oracle::PhaseVerdict::Fail`].
//!
//! # Telemetry
//! Every phase transition is recorded as a d04 [`enforcer_domain::run_record::RunRecord`]
//! (versioned serde struct), appended through the `enforcer-core` NDJSON
//! sink ([`enforcer_core::telemetry::RunTelemetrySink`]) — the same sink
//! and record shape d04 wires for `check`/`scan`, reused here rather than
//! a parallel logging path.
//!
//! # Integration seam
//! This module is a self-contained sibling of [`crate::commands`]; wiring
//! `Phase` into [`crate::cli::Command`]/`main.rs`'s dispatch is arc-22's
//! clap-grammar surface (owned by that workpack's files, not this one's
//! `owns:` set) — see the workpack's Parallel Ownership Notes. The public
//! [`run_phase`] entry point here is what that wiring calls.

use std::path::Path;

use enforcer_core::exit_codes::ExitCode;
use enforcer_core::telemetry::{default_run_telemetry_path, RunTelemetrySink};
use enforcer_domain::run_record::{ExitStatus, FindingCounts, RunRecord, RunRecordParams};
use enforcer_scan::scope::{resolve, ScopeRequest};

use crate::lifecycle::oracle::{
    check_oracle, current_repo_root, fix_oracle, implement_oracle, plan_oracle, resolve_files,
    review_oracle, PhaseVerdict, ReviewArgs,
};

pub mod oracle;

/// The five lifecycle phases, in their natural plan -> implement -> check
/// -> fix -> review order. `Copy`/closed enum: there is no sixth phase
/// and no way to spell one that parses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Plan,
    Implement,
    Check,
    Fix,
    Review,
}

impl Phase {
    /// The command-name string recorded on this phase's [`RunRecord`] and
    /// used in the CLI verb (`enforcer <verb>`).
    pub fn command_name(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Implement => "implement",
            Self::Check => "check",
            Self::Fix => "fix",
            Self::Review => "review",
        }
    }
}

/// Explicit paths a `check` phase run is scoped to. Empty means "whole
/// workspace" (mirrors [`crate::cli::ScopeArgs`]'s `--all`/bare-paths
/// duality one level up, without depending on that clap type here).
#[derive(Debug, Clone, Default)]
pub struct CheckScope {
    pub paths: Vec<std::path::PathBuf>,
}

/// Arguments a `review` phase run needs — the proof ids to gate and the
/// resolvers [`oracle::review_oracle`] consults. See
/// [`oracle::ReviewArgs`] for the field contract; this wrapper only adds
/// the `claim_id` default so callers do not have to invent one.
pub struct ReviewRequest<'a> {
    pub proof_ids: Vec<String>,
    pub current_git: enforcer_proof::envelope::GitState,
    pub latest_run: &'a dyn Fn(&str) -> Option<enforcer_proof::envelope::ProofRun>,
    pub definition: &'a dyn Fn(&str) -> Option<enforcer_proof::harness::ProofDefinition>,
    pub artifact_exists: &'a dyn Fn(&str) -> bool,
    pub required_path_exists: &'a dyn Fn(&str) -> bool,
}

/// One phase run's outcome: the verdict plus the exit code it maps to.
/// Kept as a pair (not just the `ExitCode`) so callers/tests can inspect
/// WHY a phase failed without re-deriving it from the numeric exit code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseOutcome {
    pub verdict: PhaseVerdict,
    pub exit_code: ExitCodeShim,
}

/// A `PartialEq`/`Eq`/`Debug`-able mirror of [`ExitCode`] (which is not
/// itself all of those) so [`PhaseOutcome`] can derive them for tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCodeShim {
    Success,
    Violations,
    InternalError,
}

impl From<ExitCodeShim> for ExitCode {
    fn from(shim: ExitCodeShim) -> Self {
        match shim {
            ExitCodeShim::Success => ExitCode::Success,
            ExitCodeShim::Violations => ExitCode::Violations,
            ExitCodeShim::InternalError => ExitCode::InternalError,
        }
    }
}

/// Fold an [`oracle::PhaseVerdict`] into the exit-code taxonomy: a pass is
/// always [`ExitCodeShim::Success`]; a fail is
/// [`ExitCodeShim::InternalError`] for [`oracle::FailReason::NotYetWired`]/
/// [`oracle::FailReason::Internal`] (the enforcer's own gap or bug, not the
/// scanned project's fault) and [`ExitCodeShim::Violations`] for
/// [`oracle::FailReason::OracleFindings`] (the oracle found something real
/// in the target). No branch here can produce `Success` from a `Fail`.
fn verdict_to_outcome(verdict: PhaseVerdict) -> PhaseOutcome {
    let exit_code = match &verdict {
        PhaseVerdict::Pass => ExitCodeShim::Success,
        PhaseVerdict::Fail(oracle::FailReason::OracleFindings(_)) => ExitCodeShim::Violations,
        PhaseVerdict::Fail(oracle::FailReason::NotYetWired(_))
        | PhaseVerdict::Fail(oracle::FailReason::Internal(_)) => ExitCodeShim::InternalError,
    };
    PhaseOutcome { verdict, exit_code }
}

/// Record one phase transition as a d04 [`RunRecord`], appended through
/// [`RunTelemetrySink`] at the default `proof/telemetry/runs.ndjson`
/// location. Telemetry emission is an OBSERVER (per
/// `enforcer-core::telemetry`'s contract): a failure to append telemetry
/// is swallowed here (best-effort) rather than turned into a phase
/// failure — a disk-full telemetry sink must never make an otherwise-clean
/// `check` phase report `InternalError`, matching the sink's own
/// documented observer posture.
fn record_transition(phase: Phase, outcome: &PhaseOutcome, duration_ms: u64) {
    let exit_status = match outcome.exit_code {
        ExitCodeShim::Success => ExitStatus::Clean,
        ExitCodeShim::Violations => ExitStatus::Violations,
        ExitCodeShim::InternalError => ExitStatus::Aborted,
    };
    let epoch_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0);
    let record = RunRecord::new(RunRecordParams {
        epoch_ms,
        command: phase.command_name().to_owned(),
        rule_ids_in_scope: &[],
        findings: FindingCounts::default(),
        duration_ms,
        exit_status,
    });
    let path = default_run_telemetry_path();
    if let Ok(mut sink) = RunTelemetrySink::<RunRecord>::open(Path::new(&path)) {
        let _ = sink.append(&record);
    }
}

/// `plan` phase: delegates to [`oracle::plan_oracle`].
pub fn run_plan() -> PhaseOutcome {
    let started = std::time::Instant::now();
    let outcome = verdict_to_outcome(plan_oracle());
    record_transition(
        Phase::Plan,
        &outcome,
        u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    );
    outcome
}

/// `implement` phase: delegates to [`oracle::implement_oracle`].
pub fn run_implement() -> PhaseOutcome {
    let started = std::time::Instant::now();
    let outcome = verdict_to_outcome(implement_oracle());
    record_transition(
        Phase::Implement,
        &outcome,
        u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    );
    outcome
}

/// `check` phase: resolves `scope` against the current repo root, walks
/// it, and delegates to [`oracle::check_oracle`] — the SAME validator
/// registry `enforcer check`/`scan` run, reused rather than reimplemented.
pub fn run_check(scope: &CheckScope) -> PhaseOutcome {
    let started = std::time::Instant::now();
    let outcome = (|| -> Result<PhaseOutcome, String> {
        let root = current_repo_root()?;
        let request = if scope.paths.is_empty() {
            ScopeRequest::All
        } else {
            ScopeRequest::Paths(scope.paths.clone())
        };
        let resolved = resolve(&request, &root).map_err(|e| e.to_string())?;
        let files = resolve_files(&root, &resolved).map_err(|e| e.to_string())?;
        Ok(verdict_to_outcome(check_oracle(&resolved, &files)))
    })()
    .unwrap_or_else(|message| {
        verdict_to_outcome(PhaseVerdict::Fail(oracle::FailReason::Internal(message)))
    });
    record_transition(
        Phase::Check,
        &outcome,
        u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    );
    outcome
}

/// `fix` phase: delegates to [`oracle::fix_oracle`] (see that function's
/// docs for the d07 seam this currently fails closed on).
pub fn run_fix() -> PhaseOutcome {
    let started = std::time::Instant::now();
    let outcome = verdict_to_outcome(fix_oracle());
    record_transition(
        Phase::Fix,
        &outcome,
        u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    );
    outcome
}

/// `review` phase: delegates to [`oracle::review_oracle`] — blocks on
/// missing/failed proof rows via the landed `enforcer-proof::claim` gate.
pub fn run_review(request: &ReviewRequest<'_>) -> PhaseOutcome {
    let started = std::time::Instant::now();
    let outcome = verdict_to_outcome(review_oracle(&ReviewArgs {
        claim_id: "lifecycle-review".to_owned(),
        proof_ids: request.proof_ids.clone(),
        current_git: request.current_git.clone(),
        latest_run: request.latest_run,
        definition: request.definition,
        artifact_exists: request.artifact_exists,
        required_path_exists: request.required_path_exists,
    }));
    record_transition(
        Phase::Review,
        &outcome,
        u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    );
    outcome
}

#[cfg(test)]
mod tests {
    use super::{
        run_check, run_fix, run_implement, run_plan, run_review, CheckScope, ExitCodeShim,
        ReviewRequest,
    };
    use enforcer_proof::envelope::GitState;

    #[test]
    fn plan_phase_is_not_a_prose_only_pass() {
        let outcome = run_plan();
        assert_eq!(outcome.exit_code, ExitCodeShim::InternalError);
        assert!(!outcome.verdict.is_pass());
    }

    #[test]
    fn implement_phase_is_not_a_prose_only_pass() {
        let outcome = run_implement();
        assert_eq!(outcome.exit_code, ExitCodeShim::InternalError);
        assert!(!outcome.verdict.is_pass());
    }

    #[test]
    fn fix_phase_is_not_a_prose_only_pass() {
        let outcome = run_fix();
        assert_eq!(outcome.exit_code, ExitCodeShim::InternalError);
        assert!(!outcome.verdict.is_pass());
    }

    #[test]
    fn check_phase_routes_to_the_real_validator_registry() {
        // A nonexistent path scope resolves to zero files -- an empty file
        // set is trivially clean, proving `check` actually calls the
        // engine (not a stub) rather than unconditionally failing closed
        // like plan/implement/fix above.
        let scope = CheckScope {
            paths: vec![std::path::PathBuf::from(
                "this-path-does-not-exist-anywhere",
            )],
        };
        let outcome = run_check(&scope);
        assert_eq!(outcome.exit_code, ExitCodeShim::Success);
    }

    #[test]
    fn review_phase_blocks_on_missing_proof_rows() {
        let outcome = run_review(&ReviewRequest {
            proof_ids: vec!["nonexistent-proof".to_owned()],
            current_git: GitState::default(),
            latest_run: &|_| None,
            definition: &|_| None,
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        });
        assert_eq!(outcome.exit_code, ExitCodeShim::Violations);
        assert!(!outcome.verdict.is_pass());
    }

    #[test]
    fn review_phase_requires_at_least_one_proof_id() {
        let outcome = run_review(&ReviewRequest {
            proof_ids: vec![],
            current_git: GitState::default(),
            latest_run: &|_| None,
            definition: &|_| None,
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        });
        assert!(!outcome.verdict.is_pass());
    }
}
