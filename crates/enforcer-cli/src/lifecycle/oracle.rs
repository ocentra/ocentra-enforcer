//! d06 lifecycle oracles — one typed oracle function per
//! `plan|implement|check|fix|review` phase. Every oracle returns a
//! [`PhaseVerdict`]; there is no prose-only pass path anywhere in this
//! module — a phase can only report [`PhaseVerdict::Pass`] when a real
//! Rust computation (a validator registry run, a proof-claim gate) says
//! so, never from a model's self-report.
//!
//! # Honest seam: `fix` and `review`'s dependencies are not landed yet
//! This workpack (d06) depends on d07 (`enforcer-coordination` self-correct
//! fix loop) for the `fix` oracle and d10 (resilience auditor obligation
//! rows) for part of the `review` oracle's evidence. Neither has landed a
//! concrete Rust API on `rust-build` as of this build (`enforcer-coordination`
//! exposes hub/lane/claim/guard/ledger/sync — no fix-loop entry point;
//! there is no auditor crate/module at all). Per the workpack's own
//! fail-closed posture ("no phase can report success unless its oracle
//! returns a pass"), `fix` reports [`PhaseVerdict::Fail`] with an explicit
//! `NotYetWired` reason rather than silently no-op-passing — the same
//! posture `enforcer-cli::commands`/`main.rs` already uses for
//! `install`/`plan`/`proof`/`coordination` before their owning workpacks
//! land. `review`'s proof-row gate (the part that HAS landed, via
//! `enforcer-proof::claim`) is real and wired; only the d10 auditor
//! evidence layer is stubbed.

use std::path::Path;

use enforcer_domain::paths::RepoRoot;
use enforcer_proof::claim::{claim_proof, ClaimArgs};
use enforcer_proof::envelope::GitState;
use enforcer_scan::{engine, scope::ResolvedScope};

/// Why a phase did not pass. Kept as a closed enum (not a bare `String`)
/// so callers can match on the failure class instead of parsing prose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailReason {
    /// The oracle ran and found real findings/violations.
    OracleFindings(String),
    /// The phase's oracle has no landed implementation yet (its owning
    /// workpack has not shipped a Rust API on this branch). This is a
    /// real, permanent-until-landed failure, never coerced into a pass.
    NotYetWired(String),
    /// An internal failure unrelated to the scanned project (I/O, decode).
    Internal(String),
}

/// The outcome of one lifecycle phase's oracle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhaseVerdict {
    /// The oracle's computation says this phase is clean.
    Pass,
    /// The oracle's computation says this phase is not clean.
    Fail(FailReason),
}

impl PhaseVerdict {
    /// `true` iff this verdict is [`PhaseVerdict::Pass`].
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass)
    }
}

/// `plan` oracle. arc-20 (`enforcer-plan`) owns workpack/plan-shape
/// validation; no Rust entry point for "is this plan well-formed" has
/// landed on `rust-build` yet (the crate exists but exposes no such
/// check), so this oracle fails closed rather than rubber-stamping any
/// plan as valid.
pub fn plan_oracle() -> PhaseVerdict {
    PhaseVerdict::Fail(FailReason::NotYetWired(
        "plan phase has no landed oracle yet -- arc-20 (enforcer-plan) has not shipped a \
         plan-shape validation entry point on this branch"
            .to_owned(),
    ))
}

/// `implement` oracle. There is no dedicated "implementation is complete"
/// validator anywhere in the workspace (mechanization/d01 proves a RULE's
/// fixture parity, not that arbitrary implementation work is done) —
/// fails closed rather than inventing an ad hoc heuristic.
pub fn implement_oracle() -> PhaseVerdict {
    PhaseVerdict::Fail(FailReason::NotYetWired(
        "implement phase has no landed oracle -- there is no general \
         \"implementation complete\" validator in the workspace; only \
         rule-fixture parity (d01) and file-scoped checks (check phase) exist"
            .to_owned(),
    ))
}

/// `check` oracle: delegates to the SAME validator registry `enforcer
/// check`/`scan` runs (`enforcer_scan::engine`), never a reimplementation.
/// A pass requires the built [`enforcer_domain::findings::Report::ok`] to
/// be true — a phase-local wrapper cannot report success while the
/// underlying report carries violations.
pub fn check_oracle(
    resolved: &ResolvedScope,
    files: &[enforcer_domain::paths::RelPath],
) -> PhaseVerdict {
    let validators = match engine::build_family_validators() {
        Ok(validators) => validators,
        Err(err) => {
            return PhaseVerdict::Fail(FailReason::Internal(format!(
                "failed to build validator registry: {err}"
            )))
        }
    };
    let report = engine::run(resolved, files, &validators);
    if report.ok {
        PhaseVerdict::Pass
    } else {
        PhaseVerdict::Fail(FailReason::OracleFindings(format!(
            "{} violation(s), {} warning(s)",
            report.violations.len(),
            report.warnings.len()
        )))
    }
}

/// `fix` oracle. d07 (`enforcer-coordination` self-correct fix loop) is
/// this phase's owning workpack; `enforcer-coordination` on this branch
/// exposes only hub/lane/claim/guard/ledger/presence/sync
/// (`crates/enforcer-coordination/src/api.rs`) — no fix-loop entry point
/// exists to delegate to. Fails closed with the same `NotYetWired` posture
/// as `plan`/`implement` above; this is the seam d07 must close.
pub fn fix_oracle() -> PhaseVerdict {
    PhaseVerdict::Fail(FailReason::NotYetWired(
        "fix phase has no landed oracle -- d07's enforcer-coordination fix-loop entry point \
         has not landed on this branch (api.rs exposes hub/lane/claim/guard/ledger/sync only)"
            .to_owned(),
    ))
}

/// Arguments for [`review_oracle`], grouped so the call site stays
/// self-describing (mirrors [`enforcer_proof::claim::ClaimArgs`]'s own
/// shape, one level up).
pub struct ReviewArgs<'a> {
    pub claim_id: String,
    pub proof_ids: Vec<String>,
    pub current_git: GitState,
    pub latest_run: &'a dyn Fn(&str) -> Option<enforcer_proof::envelope::ProofRun>,
    pub definition: &'a dyn Fn(&str) -> Option<enforcer_proof::harness::ProofDefinition>,
    pub artifact_exists: &'a dyn Fn(&str) -> bool,
    pub required_path_exists: &'a dyn Fn(&str) -> bool,
}

/// `review` oracle: blocks on missing/failed proof rows via the landed
/// `enforcer-proof::claim` gate (`pr_ready` claim, so a dirty worktree
/// without `allowDirty` also blocks). This is the one part of `review`'s
/// evidence base that HAS landed; the d10 resilience-auditor obligation
/// rows the workpack also names have not landed a Rust API on this
/// branch (no auditor crate/module exists yet) and are therefore NOT
/// consulted here -- `review` proves only the proof-row gate, honestly,
/// rather than claiming to prove auditor obligations it cannot yet check.
pub fn review_oracle(args: &ReviewArgs<'_>) -> PhaseVerdict {
    if args.proof_ids.is_empty() {
        return PhaseVerdict::Fail(FailReason::OracleFindings(
            "review requires at least one proof id; none were given".to_owned(),
        ));
    }
    let claim = claim_proof(&ClaimArgs {
        claim_id: args.claim_id.clone(),
        pr_ready: true,
        allow_dirty: false,
        proof_ids: args.proof_ids.clone(),
        current_git: args.current_git.clone(),
        latest_run: args.latest_run,
        definition: args.definition,
        artifact_exists: args.artifact_exists,
        required_path_exists: args.required_path_exists,
    });
    if claim.ok() {
        PhaseVerdict::Pass
    } else {
        PhaseVerdict::Fail(FailReason::OracleFindings(format!(
            "{} proof-claim violation(s): {:?}",
            claim.violations.len(),
            claim.violations
        )))
    }
}

/// Resolve the repo root the same way `enforcer-cli::commands` does
/// (current working directory, canonicalized). Kept here rather than
/// importing `crate::commands` (which is not `pub` in a way this module
/// should reach into) so `lifecycle` stays a self-contained sibling.
pub fn current_repo_root() -> Result<RepoRoot, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cannot read current directory: {e}"))?;
    cwd.to_string_lossy()
        .parse::<RepoRoot>()
        .map_err(|e| e.to_string())
}

/// Resolve the file list for a resolved scope by walking `root`, exactly
/// mirroring `enforcer-cli::commands::resolve_files`'s semantics (kept as
/// a separate small copy, not a shared helper, so `lifecycle` does not
/// reach into `commands`'s private surface -- both are trivial wrappers
/// over the same `enforcer_scan::walk` primitive).
pub fn resolve_files(
    root: &RepoRoot,
    resolved: &ResolvedScope,
) -> std::io::Result<Vec<enforcer_domain::paths::RelPath>> {
    let root_path = Path::new(root.as_str());
    let all_files =
        enforcer_scan::walk::walk(root_path, &enforcer_scan::walk::IgnoreRules::default())?;
    if resolved.explicit_paths.is_empty() {
        return Ok(all_files);
    }
    Ok(all_files
        .into_iter()
        .filter(|file| {
            resolved
                .explicit_paths
                .iter()
                .any(|explicit| file.as_str().starts_with(explicit.as_str()))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{
        check_oracle, fix_oracle, implement_oracle, plan_oracle, review_oracle, FailReason,
        PhaseVerdict, ReviewArgs,
    };
    use enforcer_proof::envelope::GitState;

    #[test]
    fn plan_oracle_fails_closed_not_yet_wired() {
        let verdict = plan_oracle();
        assert!(!verdict.is_pass());
        assert!(matches!(
            verdict,
            PhaseVerdict::Fail(FailReason::NotYetWired(_))
        ));
    }

    #[test]
    fn implement_oracle_fails_closed_not_yet_wired() {
        let verdict = implement_oracle();
        assert!(!verdict.is_pass());
        assert!(matches!(
            verdict,
            PhaseVerdict::Fail(FailReason::NotYetWired(_))
        ));
    }

    #[test]
    fn fix_oracle_fails_closed_not_yet_wired() {
        let verdict = fix_oracle();
        assert!(!verdict.is_pass());
        assert!(matches!(
            verdict,
            PhaseVerdict::Fail(FailReason::NotYetWired(_))
        ));
    }

    #[test]
    fn check_oracle_passes_on_an_empty_file_set() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_scan::scope::{resolve, ScopeRequest};

        let root: enforcer_domain::paths::RepoRoot = std::env::temp_dir()
            .to_string_lossy()
            .parse()
            .map_err(|e: enforcer_domain::boundary::decode_error::DecodeError| e.to_string())?;
        let resolved = resolve(&ScopeRequest::Paths(vec![]), &root)?;
        let verdict = check_oracle(&resolved, &[]);
        assert!(verdict.is_pass(), "expected pass, got {verdict:?}");
        Ok(())
    }

    #[test]
    fn review_oracle_rejects_empty_proof_id_list() {
        let verdict = review_oracle(&ReviewArgs {
            claim_id: "c1".to_owned(),
            proof_ids: vec![],
            current_git: GitState::default(),
            latest_run: &|_| None,
            definition: &|_| None,
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        });
        assert!(!verdict.is_pass());
        assert!(matches!(
            verdict,
            PhaseVerdict::Fail(FailReason::OracleFindings(_))
        ));
    }

    #[test]
    fn review_oracle_fails_when_no_proof_run_exists() {
        let verdict = review_oracle(&ReviewArgs {
            claim_id: "c2".to_owned(),
            proof_ids: vec!["P".to_owned()],
            current_git: GitState::default(),
            latest_run: &|_| None,
            definition: &|_| None,
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        });
        assert!(!verdict.is_pass());
    }

    #[test]
    fn review_oracle_passes_when_claim_is_clean() {
        use enforcer_proof::envelope::{GitState as GS, ProofRun, ProofStatus};
        use enforcer_proof::harness::ProofDefinition;

        let run = ProofRun {
            schema_version: 1,
            proof_id: "P".to_owned(),
            run_id: "run-1".to_owned(),
            title: "P".to_owned(),
            capability: "local".to_owned(),
            git: GS {
                commit: Some("abc".to_owned()),
                branch: Some("main".to_owned()),
                dirty: Some(false),
            },
            status: ProofStatus::Passed,
            exit_code: Some(0),
            started_at: "2026-07-04T00:00:00Z".to_owned(),
            ended_at: "2026-07-04T00:00:01Z".to_owned(),
            command: vec![],
            diagnostic_count: 0,
            pinned: false,
            artifacts: vec![],
            claims_proved: vec![],
            claims_not_proved: vec![],
        };
        let definition = ProofDefinition {
            id: "P".to_owned(),
            title: "P".to_owned(),
            family: "command".to_owned(),
            severity: "error".to_owned(),
            applies_to: vec![],
            triggers: vec![],
            languages: vec![],
            capabilities: vec!["local".to_owned()],
            collector: "command".to_owned(),
            docs: vec![],
            commands: vec![],
            required_artifacts: vec![],
            required_paths: vec![],
            required_for_pr_ready: true,
            claims_proved: vec![],
            claims_not_proved: vec![],
            ci_support: true,
            device_support: false,
        };
        let verdict = review_oracle(&ReviewArgs {
            claim_id: "c3".to_owned(),
            proof_ids: vec!["P".to_owned()],
            current_git: GitState {
                commit: Some("abc".to_owned()),
                branch: Some("main".to_owned()),
                dirty: Some(false),
            },
            latest_run: &|_| Some(run.clone()),
            definition: &|_| Some(definition.clone()),
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        });
        assert!(verdict.is_pass(), "expected pass, got {verdict:?}");
    }
}
