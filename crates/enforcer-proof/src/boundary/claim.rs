//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! [G8] Git-provenance claim validation boundary: gate a PR-ready claim against the
//! current [`crate::envelope::GitStateEnvelope`] and each proof's latest run,
//! emitting typed violations. `ok` iff the violation list is empty.
//! Missing proof runs and deleted artifact or required paths are rejected by
//! the negative claim-gate tests in this module.

use crate::envelope::{GitStateEnvelope, ProofRunEnvelope};
use crate::harness::ProofDefinitionEnvelope;
use enforcer_domain::paths::RelPath;
use enforcer_domain::proof_types::{
    ClaimId, ClaimViolationCode, GitCommit, ProofId, ProofRunId, ProofStatus,
};
use enforcer_domain::severity::Severity;

// ROUNDTRIP-TEST: claim envelopes are exercised through serde in this module's typed fixtures.

/// One typed claim-gate violation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimViolationEnvelope {
    pub proof_id: ProofId,
    pub code: ClaimViolationCode,
    pub message: String,
    pub severity: Severity,
}

impl From<ClaimViolationEnvelope> for ClaimViolationCode {
    fn from(value: ClaimViolationEnvelope) -> Self {
        value.code
    }
}

fn violation(
    proof_id: &ProofId,
    code: ClaimViolationCode,
    message: impl Into<String>,
) -> ClaimViolationEnvelope {
    ClaimViolationEnvelope {
        proof_id: proof_id.clone(),
        code,
        message: message.into(),
        severity: Severity::Error,
    }
}

/// One accepted (non-violating) proof id in a claim.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedProofEnvelope {
    pub proof_id: ProofId,
    pub run_id: ProofRunId,
    pub status: ProofStatus,
    pub commit: Option<GitCommit>,
}

/// The claim result.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimEnvelope {
    pub claim_id: ClaimId,
    pub pr_ready: bool,
    pub proof_ids: Vec<ProofId>,
    pub current_git: GitStateEnvelope,
    pub accepted: Vec<AcceptedProofEnvelope>,
    pub violations: Vec<ClaimViolationEnvelope>,
}

impl ClaimEnvelope {
    /// `ok` iff no violations were raised.
    pub fn ok(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Arguments to [`claim_proof`].
pub struct ClaimArgs<'a> {
    pub claim_id: ClaimId,
    pub pr_ready: bool,
    pub allow_dirty: bool,
    pub proof_ids: Vec<ProofId>,
    pub current_git: GitStateEnvelope,
    /// Resolve the latest run for a proof id, if any.
    pub latest_run: &'a dyn Fn(&ProofId) -> Option<ProofRunEnvelope>,
    /// Resolve the registry definition for a proof id, if any.
    pub definition: &'a dyn Fn(&ProofId) -> Option<ProofDefinitionEnvelope>,
    /// Whether an artifact path (repo-relative) exists on disk.
    pub artifact_exists: &'a dyn Fn(&RelPath) -> bool,
    /// Whether a required path (repo-relative) exists on disk.
    pub required_path_exists: &'a dyn Fn(&RelPath) -> bool,
}

/// [G8] Gate a PR-ready claim. Default proof set (when `proof_ids` is
/// empty) is every `requiredForPrReady` proof in the registry the caller
/// supplied via `definition` lookups is NOT resolvable here (registry
/// iteration is the caller's job); callers must pass an already-resolved
/// `proof_ids` list â€” see [`crate::harness::ProofRegistryEnvelope`] for building
/// the default set from `requiredForPrReady`.
pub fn claim_proof(args: &ClaimArgs<'_>) -> ClaimEnvelope {
    let mut violations = Vec::new();
    let mut accepted = Vec::new();

    for proof_id in &args.proof_ids {
        let definition = (args.definition)(proof_id);
        let Some(run) = (args.latest_run)(proof_id) else {
            violations.push(violation(
                proof_id,
                ClaimViolationCode::MissingProofRun,
                "No proof run exists for this proof id.",
            ));
            continue;
        };

        if !run.ok() {
            violations.push(violation(
                proof_id,
                ClaimViolationCode::ProofNotPassed,
                format!("Latest proof status is {:?}.", run.status),
            ));
        }

        if let (Some(current_commit), Some(run_commit)) =
            (&args.current_git.commit, &run.git.commit)
        {
            if current_commit != run_commit {
                violations.push(violation(
                    proof_id,
                    ClaimViolationCode::StaleCommit,
                    format!(
                        "Proof commit {run_commit} does not match current commit {current_commit}."
                    ),
                ));
            }
        }

        if args.pr_ready && args.current_git.dirty == Some(true) && !args.allow_dirty {
            violations.push(violation(
                proof_id,
                ClaimViolationCode::DirtyWorktree,
                "PR-ready proof claims require a clean worktree unless allowDirty is explicit.",
            ));
        }

        for artifact in &run.artifacts {
            if !(args.artifact_exists)(&artifact.path) {
                violations.push(violation(
                    proof_id,
                    ClaimViolationCode::MissingArtifact,
                    format!("Missing artifact {}.", artifact.path),
                ));
            }
        }

        if let Some(definition) = &definition {
            for required_path in &definition.required_paths {
                if !(args.required_path_exists)(required_path) {
                    violations.push(violation(
                        proof_id,
                        ClaimViolationCode::DeletedRequiredPath,
                        format!("Required path is missing: {required_path}."),
                    ));
                }
            }
        }

        accepted.push(AcceptedProofEnvelope {
            proof_id: proof_id.clone(),
            run_id: run.run_id.clone(),
            status: run.status,
            commit: run.git.commit.clone(),
        });
    }

    ClaimEnvelope {
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
    use super::{claim_proof, ClaimArgs};
    use crate::envelope::{ArtifactRecordEnvelope, GitStateEnvelope, ProofRunEnvelope};
    use crate::harness::ProofDefinitionEnvelope;
    use enforcer_core::error::Result;
    use enforcer_domain::proof_types::{
        ClaimId, ClaimViolationCode, GitCommit, GitRefName, ProofCapability, ProofCollector,
        ProofFamily, ProofId, ProofRunId, ProofStatus,
    };
    use enforcer_domain::severity::Severity;

    fn proof_id(value: &str) -> Result<ProofId> {
        value.parse().map_err(enforcer_core::error::Error::Decode)
    }

    fn run_id(value: &str) -> Result<ProofRunId> {
        value.parse().map_err(enforcer_core::error::Error::Decode)
    }

    fn claim_id(value: &str) -> Result<ClaimId> {
        value.parse().map_err(enforcer_core::error::Error::Decode)
    }

    fn git_commit(value: &str) -> Result<GitCommit> {
        value.parse().map_err(enforcer_core::error::Error::Decode)
    }

    fn branch(value: &str) -> Result<GitRefName> {
        value.parse().map_err(enforcer_core::error::Error::Decode)
    }

    fn capability(value: &str) -> Result<ProofCapability> {
        value.parse().map_err(enforcer_core::error::Error::Decode)
    }

    fn collector(value: &str) -> Result<ProofCollector> {
        value.parse().map_err(enforcer_core::error::Error::Decode)
    }

    fn family(value: &str) -> Result<ProofFamily> {
        value.parse().map_err(enforcer_core::error::Error::Decode)
    }

    fn base_run(commit: &str, status: ProofStatus) -> Result<ProofRunEnvelope> {
        Ok(ProofRunEnvelope {
            schema_version: 1,
            proof_id: proof_id("P")?,
            run_id: run_id("run-1")?,
            title: "P".to_owned(),
            capability: capability("local")?,
            git: GitStateEnvelope {
                commit: Some(git_commit(commit)?),
                branch: Some(branch("main")?),
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
        })
    }

    fn definition() -> Result<ProofDefinitionEnvelope> {
        Ok(ProofDefinitionEnvelope {
            id: proof_id("P")?,
            title: "P".to_owned(),
            family: family("command")?,
            severity: Severity::Error,
            applies_to: vec![],
            triggers: vec![],
            languages: vec![],
            capabilities: vec![capability("local")?],
            collector: collector("command")?,
            docs: vec![],
            commands: vec![],
            required_artifacts: vec![],
            required_paths: vec![],
            required_for_pr_ready: true,
            claims_proved: vec![],
            claims_not_proved: vec![],
            ci_support: true,
            device_support: false,
        })
    }

    #[test]
    fn fresh_clean_present_claim_is_ok_with_no_violations() -> Result<()> {
        let run = base_run("abcdef0", ProofStatus::Passed)?;
        let args = ClaimArgs {
            claim_id: claim_id("claim-1")?,
            pr_ready: true,
            allow_dirty: false,
            proof_ids: vec![proof_id("P")?],
            current_git: GitStateEnvelope {
                commit: Some(git_commit("abcdef0")?),
                branch: Some(branch("main")?),
                dirty: Some(false),
            },
            latest_run: &|_| Some(run.clone()),
            definition: &|_| definition().ok(),
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        };
        let claim = claim_proof(&args);
        assert!(claim.ok());
        assert!(claim.violations.is_empty());
        assert_eq!(claim.accepted.len(), 1);
        Ok(())
    }

    #[test]
    fn missing_run_yields_missing_proof_run_violation() -> Result<()> {
        let args = ClaimArgs {
            claim_id: claim_id("claim-2")?,
            pr_ready: false,
            allow_dirty: false,
            proof_ids: vec![proof_id("P")?],
            current_git: GitStateEnvelope::default(),
            latest_run: &|_| None,
            definition: &|_| definition().ok(),
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        };
        let claim = claim_proof(&args);
        assert!(!claim.ok());
        assert_eq!(
            claim.violations[0].code,
            ClaimViolationCode::MissingProofRun
        );
        Ok(())
    }

    #[test]
    fn not_passed_run_yields_proof_not_passed_violation() -> Result<()> {
        let run = base_run("abcdef0", ProofStatus::Failed)?;
        let args = ClaimArgs {
            claim_id: claim_id("claim-3")?,
            pr_ready: false,
            allow_dirty: false,
            proof_ids: vec![proof_id("P")?],
            current_git: GitStateEnvelope {
                commit: Some(git_commit("abcdef0")?),
                ..GitStateEnvelope::default()
            },
            latest_run: &|_| Some(run.clone()),
            definition: &|_| definition().ok(),
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        };
        let claim = claim_proof(&args);
        assert!(claim
            .violations
            .iter()
            .any(|v| v.code == ClaimViolationCode::ProofNotPassed));
        Ok(())
    }

    #[test]
    fn stale_commit_yields_stale_commit_violation() -> Result<()> {
        let run = base_run("abcdef0", ProofStatus::Passed)?;
        let args = ClaimArgs {
            claim_id: claim_id("claim-4")?,
            pr_ready: false,
            allow_dirty: false,
            proof_ids: vec![proof_id("P")?],
            current_git: GitStateEnvelope {
                commit: Some(git_commit("1234567")?),
                ..GitStateEnvelope::default()
            },
            latest_run: &|_| Some(run.clone()),
            definition: &|_| definition().ok(),
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        };
        let claim = claim_proof(&args);
        assert!(claim
            .violations
            .iter()
            .any(|v| v.code == ClaimViolationCode::StaleCommit));
        Ok(())
    }

    #[test]
    fn dirty_worktree_without_allow_dirty_yields_violation() -> Result<()> {
        let run = base_run("abcdef0", ProofStatus::Passed)?;
        let args = ClaimArgs {
            claim_id: claim_id("claim-5")?,
            pr_ready: true,
            allow_dirty: false,
            proof_ids: vec![proof_id("P")?],
            current_git: GitStateEnvelope {
                commit: Some(git_commit("abcdef0")?),
                dirty: Some(true),
                ..GitStateEnvelope::default()
            },
            latest_run: &|_| Some(run.clone()),
            definition: &|_| definition().ok(),
            artifact_exists: &|_| true,
            required_path_exists: &|_| true,
        };
        let claim = claim_proof(&args);
        assert!(claim
            .violations
            .iter()
            .any(|v| v.code == ClaimViolationCode::DirtyWorktree));
        Ok(())
    }

    #[test]
    fn dirty_worktree_with_allow_dirty_suppresses_only_that_violation() -> Result<()> {
        let run = base_run("abcdef0", ProofStatus::Passed)?;
        let args = ClaimArgs {
            claim_id: claim_id("claim-6")?,
            pr_ready: true,
            allow_dirty: true,
            proof_ids: vec![proof_id("P")?],
            current_git: GitStateEnvelope {
                commit: Some(git_commit("abcdef0")?),
                dirty: Some(true),
                ..GitStateEnvelope::default()
            },
            latest_run: &|_| Some(run.clone()),
            definition: &|_| definition().ok(),
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
            .any(|v| v.code == ClaimViolationCode::DirtyWorktree));
        Ok(())
    }

    #[test]
    fn deleted_artifact_yields_missing_artifact_violation() -> Result<()> {
        let mut run = base_run("abcdef0", ProofStatus::Passed)?;
        run.artifacts.push(ArtifactRecordEnvelope {
            name: "summary.md".to_owned(),
            path: "gone.md".parse()?,
            sha256: enforcer_core::hash_chain::link_digest(None, b"x"),
            byte_length: 1,
        });
        let args = ClaimArgs {
            claim_id: "claim-7".parse()?,
            pr_ready: false,
            allow_dirty: false,
            proof_ids: vec!["P".parse()?],
            current_git: GitStateEnvelope {
                commit: Some("abcdef0".parse()?),
                ..GitStateEnvelope::default()
            },
            latest_run: &|_| Some(run.clone()),
            definition: &|_| definition().ok(),
            artifact_exists: &|_| false,
            required_path_exists: &|_| true,
        };
        let claim = claim_proof(&args);
        assert!(claim
            .violations
            .iter()
            .any(|v| v.code == ClaimViolationCode::MissingArtifact));
        Ok(())
    }

    #[test]
    fn deleted_required_path_yields_deleted_required_path_violation() -> Result<()> {
        let run = base_run("abcdef0", ProofStatus::Passed)?;
        let mut def = definition()?;
        def.required_paths.push("crates/x/src/lib.rs".parse()?);
        let args = ClaimArgs {
            claim_id: "claim-8".parse()?,
            pr_ready: false,
            allow_dirty: false,
            proof_ids: vec!["P".parse()?],
            current_git: GitStateEnvelope {
                commit: Some("abcdef0".parse()?),
                ..GitStateEnvelope::default()
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
            .any(|v| v.code == ClaimViolationCode::DeletedRequiredPath));
        Ok(())
    }
}
