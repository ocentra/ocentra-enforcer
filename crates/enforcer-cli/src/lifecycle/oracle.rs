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

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::cli_types::{LifecycleFailReason, LifecycleReasonText, PhaseVerdict};
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::proof_types::{ClaimId, ProofId};
use enforcer_domain::scan_types::ResolvedScope;
use enforcer_proof::claim::{claim_proof, ClaimArgs};
use enforcer_proof::envelope::GitStateEnvelope;
use enforcer_scan::engine;

fn reason(value: String) -> Result<LifecycleReasonText, DecodeError> {
    LifecycleReasonText::try_new(value)
}

/// `plan` oracle. arc-20 (`enforcer-plan`) owns workpack/plan-shape
/// validation; no Rust entry point for "is this plan well-formed" has
/// landed on `rust-build` yet (the crate exists but exposes no such
/// check), so this oracle fails closed rather than rubber-stamping any
/// plan as valid.
pub fn plan_oracle() -> Result<PhaseVerdict, DecodeError> {
    Ok(PhaseVerdict::Fail(LifecycleFailReason::NotYetWired(
        reason(
            "plan phase has no landed oracle yet -- arc-20 (enforcer-plan) has not shipped a \
         plan-shape validation entry point on this branch"
                .to_owned(),
        )?,
    )))
}

/// `implement` oracle. There is no dedicated "implementation is complete"
/// validator anywhere in the workspace (mechanization/d01 proves a RULE's
/// fixture parity, not that arbitrary implementation work is done) —
/// fails closed rather than inventing an ad hoc heuristic.
pub fn implement_oracle() -> Result<PhaseVerdict, DecodeError> {
    Ok(PhaseVerdict::Fail(LifecycleFailReason::NotYetWired(
        reason(
            "implement phase has no landed oracle -- there is no general \
         \"implementation complete\" validator in the workspace; only \
         rule-fixture parity (d01) and file-scoped checks (check phase) exist"
                .to_owned(),
        )?,
    )))
}

/// `check` oracle: delegates to the SAME validator registry `enforcer
/// check`/`scan` runs (`enforcer_scan::engine`), never a reimplementation.
/// A pass requires the built [`enforcer_domain::findings::Report::ok`] to
/// be true — a phase-local wrapper cannot report success while the
/// underlying report carries violations.
pub fn check_oracle(
    resolved: &ResolvedScope,
    files: &[enforcer_domain::paths::RelPath],
) -> Result<PhaseVerdict, DecodeError> {
    let validators = match engine::build_family_validators() {
        Ok(validators) => validators,
        Err(err) => {
            return Ok(PhaseVerdict::Fail(LifecycleFailReason::Internal(reason(
                format!("failed to build validator registry: {err}"),
            )?)));
        }
    };
    let report = engine::run(resolved, files, &validators);
    if report.ok == enforcer_domain::findings::ReportOutcome::Clean {
        Ok(PhaseVerdict::Pass)
    } else {
        Ok(PhaseVerdict::Fail(LifecycleFailReason::OracleFindings(
            reason(format!(
                "{} violation(s), {} warning(s)",
                report.violations.len(),
                report.warnings.len()
            ))?,
        )))
    }
}

/// `fix` oracle. d07 (`enforcer-coordination` self-correct fix loop) is
/// this phase's owning workpack; `enforcer-coordination` on this branch
/// exposes only hub/lane/claim/guard/ledger/presence/sync
/// (`crates/enforcer-coordination/src/api.rs`) — no fix-loop entry point
/// exists to delegate to. Fails closed with the same `NotYetWired` posture
/// as `plan`/`implement` above; this is the seam d07 must close.
pub fn fix_oracle() -> Result<PhaseVerdict, DecodeError> {
    Ok(PhaseVerdict::Fail(LifecycleFailReason::NotYetWired(
        reason(
            "fix phase has no landed oracle -- d07's enforcer-coordination fix-loop entry point \
         has not landed on this branch (api.rs exposes hub/lane/claim/guard/ledger/sync only)"
                .to_owned(),
        )?,
    )))
}

/// Arguments for [`review_oracle`], grouped so the call site stays
/// self-describing (mirrors [`enforcer_proof::claim::ClaimArgs`]'s own
/// shape, one level up).
pub struct ReviewArgs<'a> {
    pub claim_id: ClaimId,
    pub proof_ids: Vec<ProofId>,
    pub current_git: GitStateEnvelope,
    pub latest_run: &'a dyn Fn(&ProofId) -> Option<enforcer_proof::envelope::ProofRunEnvelope>,
    pub definition:
        &'a dyn Fn(&ProofId) -> Option<enforcer_proof::harness::ProofDefinitionEnvelope>,
    pub artifact_exists: &'a dyn Fn(&RelPath) -> bool,
    pub required_path_exists: &'a dyn Fn(&RelPath) -> bool,
}

/// `review` oracle: blocks on missing/failed proof rows via the landed
/// `enforcer-proof::claim` gate (`pr_ready` claim, so a dirty worktree
/// without `allowDirty` also blocks). This is the one part of `review`'s
/// evidence base that HAS landed; the d10 resilience-auditor obligation
/// rows the workpack also names have not landed a Rust API on this
/// branch (no auditor crate/module exists yet) and are therefore NOT
/// consulted here -- `review` proves only the proof-row gate, honestly,
/// rather than claiming to prove auditor obligations it cannot yet check.
pub fn review_oracle(args: &ReviewArgs<'_>) -> Result<PhaseVerdict, DecodeError> {
    if args.proof_ids.is_empty() {
        return Ok(PhaseVerdict::Fail(LifecycleFailReason::OracleFindings(
            reason("review requires at least one proof id; none were given".to_owned())?,
        )));
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
        Ok(PhaseVerdict::Pass)
    } else {
        Ok(PhaseVerdict::Fail(LifecycleFailReason::OracleFindings(
            reason(format!(
                "{} proof-claim violation(s): {:?}",
                claim.violations.len(),
                claim.violations
            ))?,
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
        check_oracle, fix_oracle, implement_oracle, plan_oracle, review_oracle, ReviewArgs,
    };
    use enforcer_domain::cli_types::{LifecycleFailReason, PhaseVerdict};
    use enforcer_domain::proof_types::{
        ClaimId, GitCommit, GitRefName, ProofCapability, ProofCollector, ProofFamily, ProofId,
        ProofRunId,
    };
    use enforcer_domain::severity::Severity;
    use enforcer_proof::envelope::GitStateEnvelope;

    #[test]
    fn plan_oracle_fails_closed_not_yet_wired() -> Result<(), Box<dyn std::error::Error>> {
        let verdict = plan_oracle()?;
        assert!(matches!(
            verdict,
            PhaseVerdict::Fail(LifecycleFailReason::NotYetWired(_))
        ));
        Ok(())
    }

    #[test]
    fn implement_oracle_fails_closed_not_yet_wired() -> Result<(), Box<dyn std::error::Error>> {
        let verdict = implement_oracle()?;
        assert!(matches!(
            verdict,
            PhaseVerdict::Fail(LifecycleFailReason::NotYetWired(_))
        ));
        Ok(())
    }

    #[test]
    fn fix_oracle_fails_closed_not_yet_wired() -> Result<(), Box<dyn std::error::Error>> {
        let verdict = fix_oracle()?;
        assert!(matches!(
            verdict,
            PhaseVerdict::Fail(LifecycleFailReason::NotYetWired(_))
        ));
        Ok(())
    }

    #[test]
    fn check_oracle_passes_on_an_empty_file_set() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_domain::scan_types::ScopeRequest;
        use enforcer_scan::scope::resolve;

        let root: enforcer_domain::paths::RepoRoot = std::env::temp_dir()
            .to_string_lossy()
            .parse()
            .map_err(|e: enforcer_domain::boundary::decode_error::DecodeError| e.to_string())?;
        let resolved = resolve(&ScopeRequest::Paths(vec![]), &root)?;
        let verdict = check_oracle(&resolved, &[])?;
        assert!(matches!(verdict, PhaseVerdict::Pass));
        Ok(())
    }

    #[test]
    fn review_oracle_rejects_empty_proof_id_list() -> Result<(), Box<dyn std::error::Error>> {
        let verdict = review_oracle(&ReviewArgs {
            claim_id: ClaimId::try_from("c1".to_owned())?,
            proof_ids: vec![],
            current_git: GitStateEnvelope::default(),
            latest_run: &|_| None,
            definition: &|_| None,
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        })?;
        assert!(matches!(
            verdict,
            PhaseVerdict::Fail(LifecycleFailReason::OracleFindings(_))
        ));
        Ok(())
    }

    #[test]
    fn review_oracle_fails_when_no_proof_run_exists() -> Result<(), Box<dyn std::error::Error>> {
        let verdict = review_oracle(&ReviewArgs {
            claim_id: ClaimId::try_from("c2".to_owned())?,
            proof_ids: vec![ProofId::try_from("P".to_owned())?],
            current_git: GitStateEnvelope::default(),
            latest_run: &|_| None,
            definition: &|_| None,
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        })?;
        assert!(!matches!(verdict, PhaseVerdict::Pass));
        Ok(())
    }

    #[test]
    fn review_oracle_passes_when_claim_is_clean() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_domain::proof_types::ProofStatus;
        use enforcer_proof::envelope::{GitStateEnvelope as GS, ProofRunEnvelope};
        use enforcer_proof::harness::ProofDefinitionEnvelope;

        let run = ProofRunEnvelope {
            schema_version: 1,
            proof_id: ProofId::try_from("P".to_owned())?,
            run_id: ProofRunId::try_from("run-1".to_owned())?,
            title: "P".to_owned(),
            capability: ProofCapability::try_from("local".to_owned())?,
            git: GS {
                commit: Some(GitCommit::try_from("abcdef0".to_owned())?),
                branch: Some(GitRefName::try_from("main".to_owned())?),
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
        let definition = ProofDefinitionEnvelope {
            id: ProofId::try_from("P".to_owned())?,
            title: "P".to_owned(),
            family: ProofFamily::try_from("command".to_owned())?,
            severity: Severity::Error,
            applies_to: vec![],
            triggers: vec![],
            languages: vec![],
            capabilities: vec![ProofCapability::try_from("local".to_owned())?],
            collector: ProofCollector::try_from("command".to_owned())?,
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
            claim_id: ClaimId::try_from("c3".to_owned())?,
            proof_ids: vec![ProofId::try_from("P".to_owned())?],
            current_git: GitStateEnvelope {
                commit: Some(GitCommit::try_from("abcdef0".to_owned())?),
                branch: Some(GitRefName::try_from("main".to_owned())?),
                dirty: Some(false),
            },
            latest_run: &|_| Some(run.clone()),
            definition: &|_| Some(definition.clone()),
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        })?;
        assert!(matches!(verdict, PhaseVerdict::Pass));
        Ok(())
    }
}
