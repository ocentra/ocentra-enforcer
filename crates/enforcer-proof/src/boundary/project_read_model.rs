//! Project-facing proof filesystem read boundary.
//!
//! The proof harness owns the on-disk layout for current Rust proof runs so
//! callers do not invent a separate convention. This module is intentionally
//! read-only: it verifies an existing journal and surfaces malformed run
//! records, but never creates a journal or infers a claim from unrelated
//! artifacts.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use enforcer_core::error::Result;
use enforcer_domain::paths::RelPath;
use enforcer_domain::proof_types::{
    ClaimId, JournalState, ProjectClaimState, ProofFreshness, ProofId,
};

use crate::claim::{claim_proof, ClaimArgs};
use crate::envelope::{git_state, GitStateEnvelope, ProofRunEnvelope};
use crate::harness::ProofRegistryEnvelope;
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
struct ProjectProofPaths {
    proof_root: PathBuf,
    journal: PathBuf,
    runs: PathBuf,
    registry: PathBuf,
}

/// Internal, typed journal summary. Only the boundary DTO serializes it.
pub(crate) struct ProjectJournalSummary {
    pub(crate) path: RelPath,
    pub(crate) state: JournalState,
    pub(crate) record_count: usize,
    pub(crate) latest_event_type: Option<enforcer_domain::proof_types::JournalEventType>,
    pub(crate) latest_proof_id: Option<ProofId>,
    pub(crate) latest_timestamp: Option<String>,
    pub(crate) error: Option<String>,
}

/// Internal artifact accounting used while building a project snapshot.
pub(crate) struct ProjectRunArtifacts {
    pub(crate) declared: usize,
    pub(crate) present: usize,
    pub(crate) missing: usize,
    pub(crate) total_bytes: u64,
}

/// Internal parsed-run summary; the DTO is constructed only at the API edge.
pub(crate) struct ProjectProofRunSummary {
    pub(crate) path: RelPath,
    pub(crate) proof_run: Option<ProofRunEnvelope>,
    pub(crate) freshness: ProofFreshness,
    pub(crate) artifacts: ProjectRunArtifacts,
    pub(crate) parse_error: Option<String>,
}

/// Internal PR-ready claim summary.
pub(crate) struct ProjectClaimSummary {
    pub(crate) registry_path: RelPath,
    pub(crate) state: ProjectClaimState,
    pub(crate) required_proof_ids: Vec<ProofId>,
    pub(crate) claim: Option<crate::claim::ClaimEnvelope>,
    pub(crate) error: Option<String>,
}

/// Internal proof snapshot. Serialization is delegated to its boundary DTO.
pub(crate) struct ProjectProofSnapshot {
    pub(crate) proof_root: RelPath,
    pub(crate) current_git: GitStateEnvelope,
    pub(crate) journal: ProjectJournalSummary,
    pub(crate) runs: Vec<ProjectProofRunSummary>,
    pub(crate) claim: ProjectClaimSummary,
}

impl ProjectProofPaths {
    /// Resolve the fixed project proof layout below `root`.
    #[must_use]
    fn for_root(root: &Path) -> Self {
        Self {
            proof_root: root.join(PROJECT_PROOF_DIRECTORY),
            journal: root.join(PROJECT_PROOF_JOURNAL),
            runs: root.join(PROJECT_PROOF_RUNS_DIRECTORY),
            registry: root.join(PROJECT_PROOF_REGISTRY),
        }
    }
}

/// Read the project-owned proof data at the fixed layout.
///
/// A missing journal or registry is a represented state, not an error. File
/// I/O failures while enumerating the declared run directory remain errors so
/// callers do not show a partial inventory as complete.
pub(crate) fn read_project_proof_snapshot(root: &Path) -> Result<ProjectProofSnapshot> {
    let paths = ProjectProofPaths::for_root(root);
    let current_git = git_state(root);
    let journal = read_journal(root, &paths.journal)?;
    let runs = read_runs(root, &paths.runs, &current_git)?;
    let claim = read_claim(root, &paths.registry, &current_git, &runs)?;

    Ok(ProjectProofSnapshot {
        proof_root: RelPath::try_from(PROJECT_PROOF_DIRECTORY.to_owned())
            .map_err(enforcer_core::error::Error::Decode)?,
        current_git,
        journal,
        runs,
        claim,
    })
}

fn read_journal(root: &Path, path: &Path) -> Result<ProjectJournalSummary> {
    let relative = relative_path(root, path)?;
    if !path.exists() {
        return Ok(ProjectJournalSummary {
            path: relative,
            state: JournalState::Missing,
            record_count: 0,
            latest_event_type: None,
            latest_proof_id: None,
            latest_timestamp: None,
            error: None,
        });
    }

    Ok(
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
                    // CLONE-JUSTIFICATION: this summary owns journal metadata
                    // after the replayed record list is dropped.
                    latest_event_type: latest.map(|record| record.event_type.clone()),
                    // CLONE-JUSTIFICATION: this summary owns journal metadata
                    // after the replayed record list is dropped.
                    latest_proof_id: latest.map(|record| record.proof_id.clone()),
                    // CLONE-JUSTIFICATION: this summary owns journal metadata
                    // after the replayed record list is dropped.
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
        },
    )
}

fn read_runs(
    root: &Path,
    runs_root: &Path,
    current_git: &GitStateEnvelope,
) -> Result<Vec<ProjectProofRunSummary>> {
    if !runs_root.is_dir() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    collect_run_files(runs_root, &mut files)?;
    let mut runs = files
        .into_iter()
        .map(|path| read_run(root, &path, current_git))
        .collect::<Result<Vec<_>>>()?;
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

fn read_run(
    root: &Path,
    path: &Path,
    current_git: &GitStateEnvelope,
) -> Result<ProjectProofRunSummary> {
    let relative = relative_path(root, path)?;
    Ok(
        match std::fs::read(path).and_then(|bytes| {
            serde_json::from_slice::<ProofRunEnvelope>(&bytes)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
        }) {
            Ok(proof_run) => {
                let artifacts = artifact_summary(root, &proof_run);
                let freshness = freshness_for(&proof_run, current_git);
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
                freshness: ProofFreshness::Invalid,
                artifacts: ProjectRunArtifacts {
                    declared: 0,
                    present: 0,
                    missing: 0,
                    total_bytes: 0,
                },
                parse_error: Some(error.to_string()),
            },
        },
    )
}

fn artifact_summary(root: &Path, run: &ProofRunEnvelope) -> ProjectRunArtifacts {
    let declared = run.artifacts.len();
    let present = run
        .artifacts
        .iter()
        .filter(|artifact| project_path(root, &artifact.path).is_file())
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

fn freshness_for(run: &ProofRunEnvelope, current_git: &GitStateEnvelope) -> ProofFreshness {
    match (&current_git.commit, &run.git.commit) {
        (Some(current), Some(recorded)) if current == recorded => ProofFreshness::Current,
        (Some(_), Some(_)) => ProofFreshness::Stale,
        _ => ProofFreshness::Unavailable,
    }
}

fn project_path(root: &Path, path: &RelPath) -> PathBuf {
    root.join(path.as_str())
}

fn read_claim(
    root: &Path,
    registry_path: &Path,
    current_git: &GitStateEnvelope,
    runs: &[ProjectProofRunSummary],
) -> Result<ProjectClaimSummary> {
    let relative = relative_path(root, registry_path)?;
    if !registry_path.is_file() {
        return Ok(ProjectClaimSummary {
            registry_path: relative,
            state: ProjectClaimState::Unconfigured,
            required_proof_ids: Vec::new(),
            claim: None,
            error: None,
        });
    }

    let registry = match std::fs::read(registry_path)
        .map_err(enforcer_core::error::Error::from)
        .and_then(|bytes| {
            serde_json::from_slice::<ProofRegistryEnvelope>(&bytes).map_err(Into::into)
        }) {
        Ok(registry) => registry,
        Err(error) => {
            return Ok(ProjectClaimSummary {
                registry_path: relative,
                state: ProjectClaimState::InvalidRegistry,
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
        // CLONE-JUSTIFICATION: the returned project claim summary retains
        // required IDs independently of the parsed registry.
        .map(|definition| definition.id.clone())
        .collect::<Vec<_>>();
    if required_proof_ids.is_empty() {
        return Ok(ProjectClaimSummary {
            registry_path: relative,
            state: ProjectClaimState::NoRequiredProofs,
            required_proof_ids,
            claim: None,
            error: None,
        });
    }

    let definitions = registry
        .proofs
        .iter()
        .map(|definition| {
            // CLONE-JUSTIFICATION: the lookup map outlives the borrowed
            // registry while claim evaluation receives owned definitions.
            (definition.id.clone(), definition.clone())
        })
        .collect::<BTreeMap<_, _>>();
    let latest_runs = latest_runs_by_proof(runs);
    let claim = claim_proof(&ClaimArgs {
        claim_id: project_claim_id(),
        pr_ready: true,
        allow_dirty: false,
        // CLONE-JUSTIFICATION: claim evaluation consumes its input while the
        // response must retain the required IDs.
        proof_ids: required_proof_ids.clone(),
        // CLONE-JUSTIFICATION: claim evaluation owns Git state while the
        // caller's snapshot remains borrowed.
        current_git: current_git.clone(),
        // CLONE-JUSTIFICATION: the callback contract returns an owned proof
        // run from the read-model's retained lookup map.
        latest_run: &|proof_id| latest_runs.get(proof_id).cloned(),
        // CLONE-JUSTIFICATION: the callback contract returns an owned proof
        // definition from the read-model's retained lookup map.
        definition: &|proof_id| definitions.get(proof_id).cloned(),
        artifact_exists: &|path| project_path(root, path).is_file(),
        required_path_exists: &|path| project_path(root, path).exists(),
    });
    Ok(ProjectClaimSummary {
        registry_path: relative,
        state: if claim.violations.is_empty() {
            ProjectClaimState::Ready
        } else {
            ProjectClaimState::Blocked
        },
        required_proof_ids,
        claim: Some(claim),
        error: None,
    })
}

fn project_claim_id() -> ClaimId {
    let mut candidate = "project-pr-ready".to_owned();
    loop {
        if let Ok(claim_id) = ClaimId::try_from(candidate) {
            return claim_id;
        }
        candidate = "project-pr-ready".to_owned();
    }
}

fn latest_runs_by_proof(runs: &[ProjectProofRunSummary]) -> BTreeMap<ProofId, ProofRunEnvelope> {
    let mut latest = BTreeMap::new();
    for summary in runs {
        let Some(run) = &summary.proof_run else {
            continue;
        };
        let replace = latest
            .get(&run.proof_id)
            .is_none_or(|existing: &ProofRunEnvelope| run.ended_at > existing.ended_at);
        if replace {
            // CLONE-JUSTIFICATION: the latest-run index owns both its key and
            // snapshot after the borrowed read-model entries are released.
            latest.insert(run.proof_id.clone(), run.clone());
        }
    }
    latest
}

fn relative_path(root: &Path, path: &Path) -> Result<RelPath> {
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    RelPath::try_from(relative).map_err(enforcer_core::error::Error::Decode)
}

#[cfg(test)]
mod tests {
    use super::{read_project_proof_snapshot, JournalState, PROJECT_PROOF_JOURNAL};
    use crate::envelope::ProofRunEnvelope;
    use crate::journal::{JournalRecordEnvelope, ProofJournal, JOURNAL_SCHEMA_VERSION};
    use enforcer_core::error::Result;
    use enforcer_core::redaction::Redactor;
    use enforcer_domain::proof_types::ProjectClaimState;
    use enforcer_domain::proof_types::{JournalEventType, ProofId, ProofRunId};

    fn passed_run() -> Result<ProofRunEnvelope> {
        Ok(serde_json::from_value(serde_json::json!({
            "schemaVersion":1,"proofId":"PROOF-FIXTURE","runId":"run-fixture","title":"Fixture proof",
            "capability":"local","git":{},"status":"passed","exitCode":0,
            "startedAt":"2026-07-10T12:00:00Z","endedAt":"2026-07-10T12:01:00Z",
            "command":["fixture"],"diagnosticCount":0,"pinned":false,"artifacts":[],
            "claimsProved":["fixture claim"],"claimsNotProved":[]
        }))?)
    }

    #[test]
    fn project_snapshot_verifies_journal_reads_runs_and_evaluates_local_claim() -> Result<()> {
        let fixture = tempfile::tempdir()?;
        let root = fixture.path();
        let journal_path = root.join(PROJECT_PROOF_JOURNAL);
        std::fs::create_dir_all(journal_path.parent().ok_or_else(|| {
            enforcer_core::error::Error::InvalidConfig("journal path has no parent".to_owned())
        })?)?;
        let redactor = Redactor::with_defaults()?;
        let mut journal = ProofJournal::open(&journal_path)?;
        journal.append(
            &redactor,
            JournalRecordEnvelope {
                schema_version: JOURNAL_SCHEMA_VERSION,
                event_type: JournalEventType::try_from("proof-finished".to_owned())?,
                run_id: ProofRunId::try_from("run-fixture".to_owned())?,
                proof_id: ProofId::try_from("PROOF-FIXTURE".to_owned())?,
                timestamp: "2026-07-10T12:01:00Z".to_owned(),
                payload: serde_json::json!({ "summary": "fixture" }),
            },
        )?;

        let run_path = root
            .join(".enforce/proofs/runs/run-fixture")
            .join("proof-run.json");
        std::fs::create_dir_all(run_path.parent().ok_or_else(|| {
            enforcer_core::error::Error::InvalidConfig("run path has no parent".to_owned())
        })?)?;
        std::fs::write(&run_path, serde_json::to_vec(&passed_run()?)?)?;
        let registry = serde_json::json!({
            "schemaVersion":1,"productName":"fixture","proofs":[{
                "id":"PROOF-FIXTURE","title":"Fixture proof","family":"fixture",
                "severity":"error","capabilities":["local"],"collector":"command",
                "requiredForPrReady":true
            }]
        });
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
        assert_eq!(snapshot.claim.state, ProjectClaimState::Ready);
        assert!(snapshot
            .claim
            .claim
            .as_ref()
            .is_some_and(|claim| claim.violations.is_empty()));
        Ok(())
    }
}
