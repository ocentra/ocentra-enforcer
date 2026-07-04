//! [G8] Git-provenance claim gates: gate a PR-ready claim against the
//! current [`crate::envelope::GitState`] and each proof's latest run,
//! emitting typed violations. `ok` iff the violation list is empty.

use crate::envelope::{GitState, ProofRun};
use crate::harness::ProofDefinition;

/// One claim-gate violation code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ViolationCode {
    /// No proof run exists for this proof id at all.
    MissingProofRun,
    /// The latest run's status is not `passed`.
    ProofNotPassed,
    /// The latest run's recorded commit does not match the current commit.
    StaleCommit,
    /// `prReady` was requested against a dirty worktree without the
    /// `allowDirty` escape hatch.
    DirtyWorktree,
    /// One of the run's recorded artifacts no longer exists on disk.
    MissingArtifact,
    /// One of the definition's `requiredPaths` no longer exists on disk.
    DeletedRequiredPath,
}

/// One typed claim-gate violation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimViolation {
    pub proof_id: String,
    pub code: ViolationCode,
    pub message: String,
    pub severity: String,
}

fn violation(proof_id: &str, code: ViolationCode, message: impl Into<String>) -> ClaimViolation {
    ClaimViolation {
        proof_id: proof_id.to_owned(),
        code,
        message: message.into(),
        severity: "error".to_owned(),
    }
}

/// One accepted (non-violating) proof id in a claim.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedProof {
    pub proof_id: String,
    pub run_id: String,
    pub status: String,
    pub commit: Option<String>,
}

/// The claim result.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Claim {
    pub claim_id: String,
    pub pr_ready: bool,
    pub proof_ids: Vec<String>,
    pub current_git: GitState,
    pub accepted: Vec<AcceptedProof>,
    pub violations: Vec<ClaimViolation>,
}

impl Claim {
    /// `ok` iff no violations were raised.
    pub fn ok(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Arguments to [`claim_proof`].
pub struct ClaimArgs<'a> {
    pub claim_id: String,
    pub pr_ready: bool,
    pub allow_dirty: bool,
    pub proof_ids: Vec<String>,
    pub current_git: GitState,
    /// Resolve the latest run for a proof id, if any.
    pub latest_run: &'a dyn Fn(&str) -> Option<ProofRun>,
    /// Resolve the registry definition for a proof id, if any.
    pub definition: &'a dyn Fn(&str) -> Option<ProofDefinition>,
    /// Whether an artifact path (repo-relative) exists on disk.
    pub artifact_exists: &'a dyn Fn(&str) -> bool,
    /// Whether a required path (repo-relative) exists on disk.
    pub required_path_exists: &'a dyn Fn(&str) -> bool,
}

/// [G8] Gate a PR-ready claim. Default proof set (when `proof_ids` is
/// empty) is every `requiredForPrReady` proof in the registry the caller
/// supplied via `definition` lookups is NOT resolvable here (registry
/// iteration is the caller's job); callers must pass an already-resolved
/// `proof_ids` list — see [`crate::harness::ProofRegistry`] for building
/// the default set from `requiredForPrReady`.
pub fn claim_proof(args: &ClaimArgs<'_>) -> Claim {
    let mut violations = Vec::new();
    let mut accepted = Vec::new();

    for proof_id in &args.proof_ids {
        let definition = (args.definition)(proof_id);
        let Some(run) = (args.latest_run)(proof_id) else {
            violations.push(violation(
                proof_id,
                ViolationCode::MissingProofRun,
                "No proof run exists for this proof id.",
            ));
            continue;
        };

        if !run.ok() {
            violations.push(violation(
                proof_id,
                ViolationCode::ProofNotPassed,
                format!("Latest proof status is {:?}.", run.status),
            ));
        }

        if let (Some(current_commit), Some(run_commit)) =
            (&args.current_git.commit, &run.git.commit)
        {
            if current_commit != run_commit {
                violations.push(violation(
                    proof_id,
                    ViolationCode::StaleCommit,
                    format!(
                        "Proof commit {run_commit} does not match current commit {current_commit}."
                    ),
                ));
            }
        }

        if args.pr_ready && args.current_git.dirty == Some(true) && !args.allow_dirty {
            violations.push(violation(
                proof_id,
                ViolationCode::DirtyWorktree,
                "PR-ready proof claims require a clean worktree unless allowDirty is explicit.",
            ));
        }

        for artifact in &run.artifacts {
            if !(args.artifact_exists)(&artifact.path) {
                violations.push(violation(
                    proof_id,
                    ViolationCode::MissingArtifact,
                    format!("Missing artifact {}.", artifact.path),
                ));
            }
        }

        if let Some(definition) = &definition {
            for required_path in &definition.required_paths {
                if !(args.required_path_exists)(required_path) {
                    violations.push(violation(
                        proof_id,
                        ViolationCode::DeletedRequiredPath,
                        format!("Required path is missing: {required_path}."),
                    ));
                }
            }
        }

        accepted.push(AcceptedProof {
            proof_id: proof_id.clone(),
            run_id: run.run_id.clone(),
            status: format!("{:?}", run.status),
            commit: run.git.commit.clone(),
        });
    }

    Claim {
        claim_id: args.claim_id.clone(),
        pr_ready: args.pr_ready,
        proof_ids: args.proof_ids.clone(),
        current_git: args.current_git.clone(),
        accepted,
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::{claim_proof, ClaimArgs, ViolationCode};
    use crate::envelope::{ArtifactRecord, GitState, ProofRun, ProofStatus};
    use crate::harness::ProofDefinition;

    fn base_run(commit: &str, status: ProofStatus) -> ProofRun {
        ProofRun {
            schema_version: 1,
            proof_id: "P".to_owned(),
            run_id: "run-1".to_owned(),
            title: "P".to_owned(),
            capability: "local".to_owned(),
            git: GitState {
                commit: Some(commit.to_owned()),
                branch: Some("main".to_owned()),
                dirty: Some(false),
            },
            status,
            exit_code: Some(0),
            started_at: "2026-07-04T00:00:00Z".to_owned(),
            ended_at: "2026-07-04T00:00:01Z".to_owned(),
            command: vec![],
            diagnostic_count: 0,
            pinned: false,
            artifacts: vec![],
            claims_proved: vec![],
            claims_not_proved: vec![],
        }
    }

    fn definition() -> ProofDefinition {
        ProofDefinition {
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
        }
    }

    #[test]
    fn fresh_clean_present_claim_is_ok_with_no_violations() {
        let run = base_run("abc", ProofStatus::Passed);
        let args = ClaimArgs {
            claim_id: "claim-1".to_owned(),
            pr_ready: true,
            allow_dirty: false,
            proof_ids: vec!["P".to_owned()],
            current_git: GitState {
                commit: Some("abc".to_owned()),
                branch: Some("main".to_owned()),
                dirty: Some(false),
            },
            latest_run: &|_| Some(run.clone()),
            definition: &|_| Some(definition()),
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        };
        let claim = claim_proof(&args);
        assert!(claim.ok());
        assert!(claim.violations.is_empty());
        assert_eq!(claim.accepted.len(), 1);
    }

    #[test]
    fn missing_run_yields_missing_proof_run_violation() {
        let args = ClaimArgs {
            claim_id: "claim-2".to_owned(),
            pr_ready: false,
            allow_dirty: false,
            proof_ids: vec!["P".to_owned()],
            current_git: GitState::default(),
            latest_run: &|_| None,
            definition: &|_| Some(definition()),
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        };
        let claim = claim_proof(&args);
        assert!(!claim.ok());
        assert_eq!(claim.violations[0].code, ViolationCode::MissingProofRun);
    }

    #[test]
    fn not_passed_run_yields_proof_not_passed_violation() {
        let run = base_run("abc", ProofStatus::Failed);
        let args = ClaimArgs {
            claim_id: "claim-3".to_owned(),
            pr_ready: false,
            allow_dirty: false,
            proof_ids: vec!["P".to_owned()],
            current_git: GitState {
                commit: Some("abc".to_owned()),
                ..GitState::default()
            },
            latest_run: &|_| Some(run.clone()),
            definition: &|_| Some(definition()),
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        };
        let claim = claim_proof(&args);
        assert!(claim
            .violations
            .iter()
            .any(|v| v.code == ViolationCode::ProofNotPassed));
    }

    #[test]
    fn stale_commit_yields_stale_commit_violation() {
        let run = base_run("old-commit", ProofStatus::Passed);
        let args = ClaimArgs {
            claim_id: "claim-4".to_owned(),
            pr_ready: false,
            allow_dirty: false,
            proof_ids: vec!["P".to_owned()],
            current_git: GitState {
                commit: Some("new-commit".to_owned()),
                ..GitState::default()
            },
            latest_run: &|_| Some(run.clone()),
            definition: &|_| Some(definition()),
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        };
        let claim = claim_proof(&args);
        assert!(claim
            .violations
            .iter()
            .any(|v| v.code == ViolationCode::StaleCommit));
    }

    #[test]
    fn dirty_worktree_without_allow_dirty_yields_violation() {
        let run = base_run("abc", ProofStatus::Passed);
        let args = ClaimArgs {
            claim_id: "claim-5".to_owned(),
            pr_ready: true,
            allow_dirty: false,
            proof_ids: vec!["P".to_owned()],
            current_git: GitState {
                commit: Some("abc".to_owned()),
                dirty: Some(true),
                ..GitState::default()
            },
            latest_run: &|_| Some(run.clone()),
            definition: &|_| Some(definition()),
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        };
        let claim = claim_proof(&args);
        assert!(claim
            .violations
            .iter()
            .any(|v| v.code == ViolationCode::DirtyWorktree));
    }

    #[test]
    fn dirty_worktree_with_allow_dirty_suppresses_only_that_violation() {
        let run = base_run("abc", ProofStatus::Passed);
        let args = ClaimArgs {
            claim_id: "claim-6".to_owned(),
            pr_ready: true,
            allow_dirty: true,
            proof_ids: vec!["P".to_owned()],
            current_git: GitState {
                commit: Some("abc".to_owned()),
                dirty: Some(true),
                ..GitState::default()
            },
            latest_run: &|_| Some(run.clone()),
            definition: &|_| Some(definition()),
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        };
        let claim = claim_proof(&args);
        assert!(
            claim.ok(),
            "allowDirty must suppress the dirty-worktree violation"
        );
        assert!(!claim
            .violations
            .iter()
            .any(|v| v.code == ViolationCode::DirtyWorktree));
    }

    #[test]
    fn deleted_artifact_yields_missing_artifact_violation(
    ) -> Result<(), enforcer_core::error::DecodeError> {
        let mut run = base_run("abc", ProofStatus::Passed);
        run.artifacts.push(ArtifactRecord {
            name: "summary.md".to_owned(),
            path: "gone.md".to_owned(),
            sha256: enforcer_core::hash_chain::link_digest(None, b"x").parse()?,
            byte_length: 1,
        });
        let args = ClaimArgs {
            claim_id: "claim-7".to_owned(),
            pr_ready: false,
            allow_dirty: false,
            proof_ids: vec!["P".to_owned()],
            current_git: GitState {
                commit: Some("abc".to_owned()),
                ..GitState::default()
            },
            latest_run: &|_| Some(run.clone()),
            definition: &|_| Some(definition()),
            artifact_exists: &|_| false,
            required_path_exists: &|_| true,
        };
        let claim = claim_proof(&args);
        assert!(claim
            .violations
            .iter()
            .any(|v| v.code == ViolationCode::MissingArtifact));
        Ok(())
    }

    #[test]
    fn deleted_required_path_yields_deleted_required_path_violation() {
        let run = base_run("abc", ProofStatus::Passed);
        let mut def = definition();
        def.required_paths.push("crates/x/src/lib.rs".to_owned());
        let args = ClaimArgs {
            claim_id: "claim-8".to_owned(),
            pr_ready: false,
            allow_dirty: false,
            proof_ids: vec!["P".to_owned()],
            current_git: GitState {
                commit: Some("abc".to_owned()),
                ..GitState::default()
            },
            latest_run: &|_| Some(run.clone()),
            definition: &|_| Some(def.clone()),
            artifact_exists: &|_| true,
            required_path_exists: &|_| false,
        };
        let claim = claim_proof(&args);
        assert!(claim
            .violations
            .iter()
            .any(|v| v.code == ViolationCode::DeletedRequiredPath));
    }
}
