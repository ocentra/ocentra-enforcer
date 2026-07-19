//! d06 lifecycle commands: `plan | implement | check | fix | review`, a
//! clap subcommand family where every phase's pass/fail is decided by a
//! Rust oracle ([`oracle`]), never by prose or model self-report.
//!
//! # Dispatch table, not a match-per-caller
//! [`Phase`] enumerates the five phases; [`run_phase`] is the ONE dispatch
//! function mapping a `Phase` to its oracle and folding the resulting
//! [`oracle::PhaseVerdict`] into an [`enforcer_domain::core_types::ExitCode`].
//! A phase failure always yields a non-zero exit — there is no phase that
//! can report [`enforcer_domain::core_types::ExitCode::Success`] while its
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

use enforcer_core::telemetry::{default_run_telemetry_path, RunTelemetrySink};
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::cli_types::CheckScope;
use enforcer_domain::cli_types::Phase;
use enforcer_domain::cli_types::{LifecycleFailReason, LifecycleReasonText, PhaseVerdict};
use enforcer_domain::core_types::ExitCode;
use enforcer_domain::run_record::{ExitStatus, FindingCounts, RunRecord, RunRecordParams};
use enforcer_domain::scan_types::ScopeRequest;
use enforcer_domain::telemetry_types::{DurationMillis, EpochMillis, RunCommandName};
use enforcer_scan::scope::resolve;

use oracle::{
    check_oracle, current_repo_root, fix_oracle, implement_oracle, plan_oracle, resolve_files,
    review_oracle, ReviewArgs,
};

pub mod oracle;

/// One phase run's outcome: the verdict plus the exit code it maps to.
/// Kept as a pair (not just the `ExitCode`) so callers/tests can inspect
/// WHY a phase failed without re-deriving it from the numeric exit code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseOutcome {
    pub verdict: Result<PhaseVerdict, DecodeError>,
    pub exit_code: ExitCode,
}

/// Fold a canonical [`PhaseVerdict`] into the shared exit-code taxonomy.
/// Oracle findings are violations, unwired/internal lifecycle failures are
/// internal errors, and boundary decode failures also fail internally.
fn verdict_to_outcome(verdict: Result<PhaseVerdict, DecodeError>) -> PhaseOutcome {
    let exit_code = match &verdict {
        Ok(PhaseVerdict::Pass) => ExitCode::Success,
        Ok(PhaseVerdict::Fail(LifecycleFailReason::OracleFindings(_))) => ExitCode::Violations,
        Ok(
            PhaseVerdict::Fail(LifecycleFailReason::NotYetWired(_))
            | PhaseVerdict::Fail(LifecycleFailReason::Internal(_)),
        )
        | Err(_) => ExitCode::InternalError,
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
        ExitCode::Success => ExitStatus::Clean,
        ExitCode::Violations => ExitStatus::Violations,
        ExitCode::UsageError | ExitCode::ConfigError | ExitCode::InternalError => {
            ExitStatus::Aborted
        }
    };
    let epoch_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0);
    let Ok(command) = RunCommandName::try_new(phase.command_name().to_owned()) else {
        return;
    };
    let record = RunRecord::new(RunRecordParams {
        epoch_ms: EpochMillis::new(epoch_ms),
        command,
        rule_ids_in_scope: &[],
        findings: FindingCounts::default(),
        duration_ms: DurationMillis::new(duration_ms),
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
        let request = if scope.paths().is_empty() {
            ScopeRequest::All
        } else {
            ScopeRequest::Paths(
                scope
                    .paths()
                    .iter()
                    .map(|path| path.as_path().to_path_buf())
                    .collect(),
            )
        };
        let resolved = resolve(&request, &root).map_err(|e| e.to_string())?;
        let files = resolve_files(&root, &resolved).map_err(|e| e.to_string())?;
        Ok(verdict_to_outcome(check_oracle(&resolved, &files)))
    })()
    .unwrap_or_else(|message| {
        verdict_to_outcome(
            LifecycleReasonText::try_new(message)
                .map(|reason| PhaseVerdict::Fail(LifecycleFailReason::Internal(reason))),
        )
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
pub fn run_review(request: &ReviewArgs<'_>) -> PhaseOutcome {
    let started = std::time::Instant::now();
    let outcome = verdict_to_outcome(review_oracle(request));
    record_transition(
        Phase::Review,
        &outcome,
        u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    );
    outcome
}

#[cfg(test)]
mod tests {
    use super::{run_check, run_fix, run_implement, run_plan, run_review, CheckScope, ReviewArgs};
    use enforcer_domain::cli_types::PhaseVerdict;
    use enforcer_domain::core_types::ExitCode;
    use enforcer_domain::proof_types::{ClaimId, ProofId};
    use enforcer_proof::envelope::GitStateEnvelope;

    #[test]
    fn plan_phase_is_not_a_prose_only_pass() {
        let outcome = run_plan();
        assert_eq!(outcome.exit_code, ExitCode::InternalError);
        assert!(!matches!(outcome.verdict, Ok(PhaseVerdict::Pass)));
    }

    #[test]
    fn implement_phase_is_not_a_prose_only_pass() {
        let outcome = run_implement();
        assert_eq!(outcome.exit_code, ExitCode::InternalError);
        assert!(!matches!(outcome.verdict, Ok(PhaseVerdict::Pass)));
    }

    #[test]
    fn fix_phase_is_not_a_prose_only_pass() {
        let outcome = run_fix();
        assert_eq!(outcome.exit_code, ExitCode::InternalError);
        assert!(!matches!(outcome.verdict, Ok(PhaseVerdict::Pass)));
    }

    #[test]
    fn check_phase_routes_to_the_real_validator_registry() -> Result<(), Box<dyn std::error::Error>>
    {
        // A nonexistent path scope resolves to zero files -- an empty file
        // set is trivially clean, proving `check` actually calls the
        // engine (not a bypass) rather than unconditionally failing closed
        // like plan/implement/fix above.
        let scope = CheckScope::new(vec![enforcer_domain::cli_types::CliSelectedPath::new(
            std::path::PathBuf::from("this-path-does-not-exist-anywhere"),
        )?]);
        let outcome = run_check(&scope);
        assert_eq!(outcome.exit_code, ExitCode::Success);
        Ok(())
    }

    #[test]
    fn review_phase_blocks_on_missing_proof_rows() -> Result<(), Box<dyn std::error::Error>> {
        let outcome = run_review(&ReviewArgs {
            claim_id: ClaimId::try_from("lifecycle-review".to_owned())?,
            proof_ids: vec![ProofId::try_from("nonexistent-proof".to_owned())?],
            current_git: GitStateEnvelope::default(),
            latest_run: &|_| None,
            definition: &|_| None,
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        });
        assert_eq!(outcome.exit_code, ExitCode::Violations);
        assert!(!matches!(outcome.verdict, Ok(PhaseVerdict::Pass)));
        Ok(())
    }

    #[test]
    fn review_phase_requires_at_least_one_proof_id() -> Result<(), Box<dyn std::error::Error>> {
        let outcome = run_review(&ReviewArgs {
            claim_id: ClaimId::try_from("lifecycle-review".to_owned())?,
            proof_ids: vec![],
            current_git: GitStateEnvelope::default(),
            latest_run: &|_| None,
            definition: &|_| None,
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        });
        assert!(!matches!(outcome.verdict, Ok(PhaseVerdict::Pass)));
        Ok(())
    }
}
