//! Project-facing proof read model.
//!
//! The proof harness owns the on-disk layout for current Rust proof runs so
//! callers do not invent a separate convention. This module is intentionally
//! read-only: it verifies an existing journal and surfaces malformed run
//! records, but never creates a journal or infers a claim from unrelated
//! artifacts.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use enforcer_core::error::Result;

use crate::claim::{claim_proof, Claim, ClaimArgs};
use crate::envelope::{git_state, GitState, ProofRun};
use crate::harness::ProofRegistry;
use crate::journal::ProofJournal;

/// The project-relative directory for Rust proof state.
pub const PROJECT_PROOF_DIRECTORY: &str = ".enforce/proofs";
/// The project-relative hash-chained proof journal location.
pub const PROJECT_PROOF_JOURNAL: &str = ".enforce/proofs/journal.ndjson";
/// The project-relative directory containing one folder per proof run.
pub const PROJECT_PROOF_RUNS_DIRECTORY: &str = ".enforce/proofs/runs";
/// The file that serializes one completed proof run.
pub const PROJECT_PROOF_RUN_FILE: &str = "proof-run.json";
/// An optional project-local registry used to evaluate required PR-ready
/// claims. Profile registries are intentionally not guessed here.
pub const PROJECT_PROOF_REGISTRY: &str = "proofs.json";

/// Canonical paths for a project's Rust proof data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectProofPaths {
    pub proof_root: PathBuf,
    pub journal: PathBuf,
    pub runs: PathBuf,
    pub registry: PathBuf,
}

impl ProjectProofPaths {
    /// Resolve the fixed project proof layout below `root`.
    #[must_use]
    pub fn for_root(root: &Path) -> Self {
        Self {
            proof_root: root.join(PROJECT_PROOF_DIRECTORY),
            journal: root.join(PROJECT_PROOF_JOURNAL),
            runs: root.join(PROJECT_PROOF_RUNS_DIRECTORY),
            registry: root.join(PROJECT_PROOF_REGISTRY),
        }
    }
}

/// Hash-chain verification result for the project's journal.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JournalState {
    Missing,
    Verified,
    Invalid,
}

/// The verified (or rejected) state of the current project's journal.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectJournalSummary {
    pub path: String,
    pub state: JournalState,
    pub record_count: usize,
    pub latest_event_type: Option<String>,
    pub latest_proof_id: Option<String>,
    pub latest_timestamp: Option<String>,
    pub error: Option<String>,
}

/// Artifact presence associated with one parsed proof run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRunArtifacts {
    pub declared: usize,
    pub present: usize,
    pub missing: usize,
    pub total_bytes: u64,
}

/// One project proof-run record. A malformed `proof-run.json` remains visible
/// as `parse_error`; callers must not treat it as a missing or passed run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectProofRunSummary {
    pub path: String,
    pub proof_run: Option<ProofRun>,
    pub freshness: String,
    pub artifacts: ProjectRunArtifacts,
    pub parse_error: Option<String>,
}

/// Project claim result. It is absent when the project has not opted into a
/// local registry, rather than silently selecting an unrelated profile.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectClaimSummary {
    pub registry_path: String,
    pub state: String,
    pub required_proof_ids: Vec<String>,
    pub claim: Option<Claim>,
    pub error: Option<String>,
}

/// Read-only snapshot used by desktop/API consumers.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectProofSnapshot {
    pub proof_root: String,
    pub current_git: GitState,
    pub journal: ProjectJournalSummary,
    pub runs: Vec<ProjectProofRunSummary>,
    pub claim: ProjectClaimSummary,
}

/// Read the project-owned proof data at the fixed layout.
///
/// A missing journal or registry is a represented state, not an error. File
/// I/O failures while enumerating the declared run directory remain errors so
/// callers do not show a partial inventory as complete.
pub fn read_project_proof_snapshot(root: &Path) -> Result<ProjectProofSnapshot> {
    let paths = ProjectProofPaths::for_root(root);
    let current_git = git_state(root);
    let journal = read_journal(root, &paths.journal);
    let runs = read_runs(root, &paths.runs, &current_git)?;
    let claim = read_claim(root, &paths.registry, &current_git, &runs)?;

    Ok(ProjectProofSnapshot {
        proof_root: PROJECT_PROOF_DIRECTORY.to_owned(),
        current_git,
        journal,
        runs,
        claim,
    })
}

fn read_journal(root: &Path, path: &Path) -> ProjectJournalSummary {
    let relative = relative_path(root, path);
    if !path.exists() {
        return ProjectJournalSummary {
            path: relative,
            state: JournalState::Missing,
            record_count: 0,
            latest_event_type: None,
            latest_proof_id: None,
            latest_timestamp: None,
            error: None,
        };
    }

    match ProofJournal::open(path).and_then(|journal| {
        journal.verify_on_replay()?;
        journal.records()
    }) {
        Ok(records) => {
            let latest = records.last();
            ProjectJournalSummary {
                path: relative,
                state: JournalState::Verified,
                record_count: records.len(),
                latest_event_type: latest.map(|record| record.event_type.clone()),
                latest_proof_id: latest.map(|record| record.proof_id.clone()),
                latest_timestamp: latest.map(|record| record.timestamp.clone()),
                error: None,
            }
        }
        Err(error) => ProjectJournalSummary {
            path: relative,
            state: JournalState::Invalid,
            record_count: 0,
            latest_event_type: None,
            latest_proof_id: None,
            latest_timestamp: None,
            error: Some(error.to_string()),
        },
    }
}

fn read_runs(
    root: &Path,
    runs_root: &Path,
    current_git: &GitState,
) -> Result<Vec<ProjectProofRunSummary>> {
    if !runs_root.is_dir() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    collect_run_files(runs_root, &mut files)?;
    let mut runs = files
        .into_iter()
        .map(|path| read_run(root, &path, current_git))
        .collect::<Vec<_>>();
    runs.sort_by(|left, right| {
        let right_timestamp = right
            .proof_run
            .as_ref()
            .map(|run| run.ended_at.as_str())
            .unwrap_or_default();
        let left_timestamp = left
            .proof_run
            .as_ref()
            .map(|run| run.ended_at.as_str())
            .unwrap_or_default();
        right_timestamp
            .cmp(left_timestamp)
            .then_with(|| left.path.cmp(&right.path))
    });
    Ok(runs)
}

fn collect_run_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_run_files(&path, files)?;
        } else if path
            .file_name()
            .is_some_and(|name| name == PROJECT_PROOF_RUN_FILE)
        {
            files.push(path);
        }
    }
    Ok(())
}

fn read_run(root: &Path, path: &Path, current_git: &GitState) -> ProjectProofRunSummary {
    let relative = relative_path(root, path);
    match std::fs::read(path).and_then(|bytes| {
        serde_json::from_slice::<ProofRun>(&bytes)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
    }) {
        Ok(proof_run) => {
            let artifacts = artifact_summary(root, &proof_run);
            let freshness = freshness_for(&proof_run, current_git).to_owned();
            ProjectProofRunSummary {
                path: relative,
                proof_run: Some(proof_run),
                freshness,
                artifacts,
                parse_error: None,
            }
        }
        Err(error) => ProjectProofRunSummary {
            path: relative,
            proof_run: None,
            freshness: "invalid".to_owned(),
            artifacts: ProjectRunArtifacts {
                declared: 0,
                present: 0,
                missing: 0,
                total_bytes: 0,
            },
            parse_error: Some(error.to_string()),
        },
    }
}

fn artifact_summary(root: &Path, run: &ProofRun) -> ProjectRunArtifacts {
    let declared = run.artifacts.len();
    let present = run
        .artifacts
        .iter()
        .filter(|artifact| root.join(&artifact.path).is_file())
        .count();
    ProjectRunArtifacts {
        declared,
        present,
        missing: declared.saturating_sub(present),
        total_bytes: run
            .artifacts
            .iter()
            .map(|artifact| artifact.byte_length)
            .fold(0_u64, u64::saturating_add),
    }
}

fn freshness_for(run: &ProofRun, current_git: &GitState) -> &'static str {
    match (&current_git.commit, &run.git.commit) {
        (Some(current), Some(recorded)) if current == recorded => "current",
        (Some(_), Some(_)) => "stale",
        _ => "unavailable",
    }
}

fn read_claim(
    root: &Path,
    registry_path: &Path,
    current_git: &GitState,
    runs: &[ProjectProofRunSummary],
) -> Result<ProjectClaimSummary> {
    let relative = relative_path(root, registry_path);
    if !registry_path.is_file() {
        return Ok(ProjectClaimSummary {
            registry_path: relative,
            state: "unconfigured".to_owned(),
            required_proof_ids: Vec::new(),
            claim: None,
            error: None,
        });
    }

    let registry = match std::fs::read(registry_path)
        .map_err(enforcer_core::error::Error::from)
        .and_then(|bytes| serde_json::from_slice::<ProofRegistry>(&bytes).map_err(Into::into))
    {
        Ok(registry) => registry,
        Err(error) => {
            return Ok(ProjectClaimSummary {
                registry_path: relative,
                state: "invalid-registry".to_owned(),
                required_proof_ids: Vec::new(),
                claim: None,
                error: Some(error.to_string()),
            })
        }
    };

    let required_proof_ids = registry
        .proofs
        .iter()
        .filter(|definition| definition.required_for_pr_ready)
        .map(|definition| definition.id.clone())
        .collect::<Vec<_>>();
    if required_proof_ids.is_empty() {
        return Ok(ProjectClaimSummary {
            registry_path: relative,
            state: "no-required-proofs".to_owned(),
            required_proof_ids,
            claim: None,
            error: None,
        });
    }

    let definitions = registry
        .proofs
        .iter()
        .map(|definition| (definition.id.clone(), definition.clone()))
        .collect::<BTreeMap<_, _>>();
    let latest_runs = latest_runs_by_proof(runs);
    let claim = claim_proof(&ClaimArgs {
        claim_id: "project-pr-ready".to_owned(),
        pr_ready: true,
        allow_dirty: false,
        proof_ids: required_proof_ids.clone(),
        current_git: current_git.clone(),
        latest_run: &|proof_id| latest_runs.get(proof_id).cloned(),
        definition: &|proof_id| definitions.get(proof_id).cloned(),
        artifact_exists: &|path| root.join(path).is_file(),
        required_path_exists: &|path| root.join(path).exists(),
    });
    Ok(ProjectClaimSummary {
        registry_path: relative,
        state: if claim.violations.is_empty() {
            "ready"
        } else {
            "blocked"
        }
        .to_owned(),
        required_proof_ids,
        claim: Some(claim),
        error: None,
    })
}

fn latest_runs_by_proof(runs: &[ProjectProofRunSummary]) -> BTreeMap<String, ProofRun> {
    let mut latest = BTreeMap::new();
    for summary in runs {
        let Some(run) = &summary.proof_run else {
            continue;
        };
        let replace = latest
            .get(&run.proof_id)
            .is_none_or(|existing: &ProofRun| run.ended_at > existing.ended_at);
        if replace {
            latest.insert(run.proof_id.clone(), run.clone());
        }
    }
    latest
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::{read_project_proof_snapshot, JournalState, PROJECT_PROOF_JOURNAL};
    use crate::envelope::{GitState, ProofRun, ProofStatus};
    use crate::harness::{ProofDefinition, ProofRegistry};
    use crate::journal::{JournalRecord, ProofJournal, JOURNAL_SCHEMA_VERSION};
    use enforcer_core::error::Result;
    use enforcer_core::redaction::Redactor;

    fn passed_run() -> ProofRun {
        ProofRun {
            schema_version: 1,
            proof_id: "PROOF-FIXTURE".to_owned(),
            run_id: "run-fixture".to_owned(),
            title: "Fixture proof".to_owned(),
            capability: "local".to_owned(),
            git: GitState::default(),
            status: ProofStatus::Passed,
            exit_code: Some(0),
            started_at: "2026-07-10T12:00:00Z".to_owned(),
            ended_at: "2026-07-10T12:01:00Z".to_owned(),
            command: vec!["fixture".to_owned()],
            diagnostic_count: 0,
            pinned: false,
            artifacts: Vec::new(),
            claims_proved: vec!["fixture claim".to_owned()],
            claims_not_proved: Vec::new(),
        }
    }

    #[test]
    fn project_snapshot_verifies_journal_reads_runs_and_evaluates_local_claim() -> Result<()> {
        let fixture = tempfile::tempdir()?;
        let root = fixture.path();
        let journal_path = root.join(PROJECT_PROOF_JOURNAL);
        std::fs::create_dir_all(journal_path.parent().expect("journal parent"))?;
        let redactor = Redactor::with_defaults()?;
        let mut journal = ProofJournal::open(&journal_path)?;
        journal.append(
            &redactor,
            JournalRecord {
                schema_version: JOURNAL_SCHEMA_VERSION,
                event_type: "proof-finished".to_owned(),
                run_id: "run-fixture".to_owned(),
                proof_id: "PROOF-FIXTURE".to_owned(),
                timestamp: "2026-07-10T12:01:00Z".to_owned(),
                payload: serde_json::json!({ "summary": "fixture" }),
            },
        )?;

        let run_path = root
            .join(".enforce/proofs/runs/run-fixture")
            .join("proof-run.json");
        std::fs::create_dir_all(run_path.parent().expect("run parent"))?;
        std::fs::write(&run_path, serde_json::to_vec(&passed_run())?)?;
        let registry = ProofRegistry {
            schema_version: 1,
            product_name: "fixture".to_owned(),
            proofs: vec![ProofDefinition {
                id: "PROOF-FIXTURE".to_owned(),
                title: "Fixture proof".to_owned(),
                family: "fixture".to_owned(),
                severity: "error".to_owned(),
                applies_to: Vec::new(),
                triggers: Vec::new(),
                languages: Vec::new(),
                capabilities: vec!["local".to_owned()],
                collector: "command".to_owned(),
                docs: Vec::new(),
                commands: Vec::new(),
                required_artifacts: Vec::new(),
                required_paths: Vec::new(),
                required_for_pr_ready: true,
                claims_proved: Vec::new(),
                claims_not_proved: Vec::new(),
                ci_support: false,
                device_support: false,
            }],
        };
        std::fs::write(root.join("proofs.json"), serde_json::to_vec(&registry)?)?;

        let snapshot = read_project_proof_snapshot(root)?;
        assert_eq!(snapshot.journal.state, JournalState::Verified);
        assert_eq!(snapshot.journal.record_count, 1);
        assert_eq!(snapshot.runs.len(), 1);
        assert_eq!(
            snapshot.runs[0]
                .proof_run
                .as_ref()
                .map(|run| run.run_id.as_str()),
            Some("run-fixture")
        );
        assert_eq!(snapshot.claim.state, "ready");
        assert!(snapshot
            .claim
            .claim
            .as_ref()
            .is_some_and(|claim| claim.violations.is_empty()));
        Ok(())
    }
}
