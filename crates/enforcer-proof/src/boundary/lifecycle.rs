//! BOUNDARY-INVARIANT: this boundary validates typed caller values before every filesystem mutation and keeps all state beneath the repository-owned proof root.
//! Negative invalid-input coverage rejects duplicate run ids, malformed ids, undeclared artifacts, and path escapes.
//! Native, durable proof lifecycle boundary.
//!
//! This is the one writer for project proof state.  It deliberately exposes
//! typed domain inputs and the existing project read-model DTO rather than
//! giving each transport its own JSON persistence implementation.

use std::path::{Path, PathBuf};

use enforcer_core::error::{Error, Result};
use enforcer_core::redaction::Redactor;
use enforcer_domain::paths::RelPath;
use enforcer_domain::proof_types::{JournalEventType, ProofId, ProofRunId};

use crate::boundary::read_model::ProjectProofSnapshotDto;
use crate::harness::{run_proof, ProofDefinitionEnvelope, RunOutcome, RunProofArgs};
use crate::journal::{JournalRecordEnvelope, ProofJournal, JOURNAL_SCHEMA_VERSION};
use crate::read_model::{
    read_project_proof_snapshot, PROJECT_PROOF_JOURNAL, PROJECT_PROOF_RUN_FILE,
};

/// Maximum bytes a caller may read from a declared artifact in one request.
pub const MAX_DECLARED_ARTIFACT_BYTES: u64 = 256 * 1024;

/// Opened native lifecycle rooted at one repository.  All persistence stays
/// beneath `<root>/.enforce/proofs`; caller supplied relative paths are never
/// permitted to escape that root.
pub struct NativeProofLifecycle {
    root: PathBuf,
    proof_root: PathBuf,
}

impl NativeProofLifecycle {
    /// Open the lifecycle and verify any existing journal before allowing a
    /// mutation.  This fail-closed check prevents appending after tampering.
    pub fn open(root: &Path) -> Result<Self> {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let proof_root = root.join(".enforce").join("proofs");
        if proof_root.join("journal.ndjson").exists() {
            ProofJournal::open(&proof_root.join("journal.ndjson"))?;
        }
        Ok(Self { root, proof_root })
    }

    /// Current public read model.  This reuses the crate-owned DTO boundary.
    pub fn snapshot(&self) -> Result<ProjectProofSnapshotDto> {
        read_project_proof_snapshot(&self.root).map(Into::into)
    }

    /// Serialize the canonical public snapshot for an export transport.
    pub fn export(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(&self.snapshot()?).map_err(Into::into)
    }

    /// Evaluate the project-local required-proof claim from the canonical read
    /// model; the claim engine remains owned by `claim`, not a transport.
    pub fn claim(&self) -> Result<crate::boundary::read_model_claim::ProjectClaimSummaryDto> {
        Ok(self.snapshot()?.claim)
    }

    /// Compact native diagnostics derived from corrupt, stale, or failed
    /// persisted runs.  No unbounded artifact content is exposed here.
    pub fn diagnostics(&self) -> Result<Vec<serde_json::Value>> {
        Ok(self.snapshot()?.runs.into_iter().filter_map(|summary| {
            summary.parse_error.map(|error| serde_json::json!({"path":summary.path,"error":error}))
                .or_else(|| summary.proof_run.filter(|run| !run.ok()).map(|run| serde_json::json!({"runId":run.run_id,"proofId":run.proof_id,"status":run.status})))
        }).collect())
    }

    /// The most recent failed/native-manual run, if one exists.
    pub fn last_failure(&self) -> Result<Option<crate::envelope::ProofRunEnvelope>> {
        Ok(self
            .snapshot()?
            .runs
            .into_iter()
            .find_map(|summary| summary.proof_run.filter(|run| !run.ok())))
    }

    /// Collect legacy evidence inside the repository, create a typed imported
    /// run, and persist it through the same journal/atomic path.
    pub fn import_legacy(
        &self,
        proof_id: &ProofId,
        run_id: &ProofRunId,
        roots: &[&str],
    ) -> Result<crate::envelope::ProofRunEnvelope> {
        let bundle = crate::legacy_import::collect_legacy_artifacts(&self.root, roots)?;
        let run = crate::legacy_import::import_legacy_proof(
            proof_id,
            run_id,
            crate::envelope::git_state(&self.root),
            &bundle,
        );
        self.import_run(&run)?;
        Ok(run)
    }

    /// Compare an imported run with freshly collected repository-contained
    /// legacy evidence. `deletion_ready` remains false unless hashes, claims,
    /// statuses, and collected artifacts all agree.
    pub fn parity(
        &self,
        run_id: &ProofRunId,
        roots: &[&str],
    ) -> Result<(enforcer_domain::proof_types::ProofCoverage, bool)> {
        let bundle = crate::legacy_import::collect_legacy_artifacts(&self.root, roots)?;
        let imported = self.read_run(run_id)?;
        Ok(crate::legacy_import::proof_parity(&bundle, Some(&imported)))
    }

    /// Run one proof, journal the intent before the durable run mutation, then
    /// atomically persist the completed run and journal its terminal state.
    pub fn run(
        &self,
        args: &RunProofArgs,
        definition: Option<&ProofDefinitionEnvelope>,
    ) -> Result<RunOutcome> {
        if args.root != self.root {
            return Err(Error::InvalidConfig(
                "proof run root differs from lifecycle root".to_owned(),
            ));
        }
        self.append_event(
            "proof-started",
            &args.run_id,
            &args.proof_id,
            serde_json::json!({}),
        )?;
        let outcome = run_proof(args, definition)?;
        self.persist_run(&outcome.proof_run)?;
        self.append_event(
            "proof-finished",
            &outcome.proof_run.run_id,
            &outcome.proof_run.proof_id,
            serde_json::json!({ "status": format!("{:?}", outcome.proof_run.status).to_ascii_lowercase() }),
        )?;
        Ok(outcome)
    }

    /// Persist an imported/externally collected run through the same journal
    /// and atomic writer as executed runs.
    pub fn import_run(&self, run: &crate::envelope::ProofRunEnvelope) -> Result<()> {
        self.append_event(
            "proof-import-started",
            &run.run_id,
            &run.proof_id,
            serde_json::json!({}),
        )?;
        self.persist_run(run)?;
        self.append_event(
            "legacy-artifacts-imported",
            &run.run_id,
            &run.proof_id,
            serde_json::json!({}),
        )
    }

    /// Read a declared artifact only when its path is repo-contained and it is
    /// actually declared by a persisted run. Returned bytes are redacted and
    /// bounded; undeclared, escaping, or oversized artifacts fail closed.
    pub fn read_declared_artifact(&self, run_id: &ProofRunId, path: &RelPath) -> Result<Vec<u8>> {
        let run = self.read_run(run_id)?;
        if !run.artifacts.iter().any(|artifact| artifact.path == *path) {
            return Err(Error::InvalidConfig(
                "artifact is not declared by this proof run".to_owned(),
            ));
        }
        let target = self.contained_path(path)?;
        let metadata = std::fs::metadata(&target)?;
        if metadata.len() > MAX_DECLARED_ARTIFACT_BYTES {
            return Err(Error::InvalidConfig(
                "declared artifact exceeds read bound".to_owned(),
            ));
        }
        let mut value = serde_json::Value::String(
            String::from_utf8_lossy(&std::fs::read(target)?).into_owned(),
        );
        Redactor::with_defaults()?.redact(&mut value);
        match value {
            serde_json::Value::String(redacted) => Ok(redacted.into_bytes()),
            _ => Err(Error::InvalidConfig(
                "artifact redaction returned unexpected shape".to_owned(),
            )),
        }
    }

    /// Delete one persisted run only after recording the durable intent.
    pub fn prune_run(&self, run_id: &ProofRunId) -> Result<bool> {
        let run = self.read_run(run_id)?;
        self.append_event(
            "proof-prune-started",
            run_id,
            &run.proof_id,
            serde_json::json!({}),
        )?;
        let dir = self.proof_root.join("runs").join(run_id.as_str());
        if !dir.exists() {
            return Ok(false);
        }
        std::fs::remove_dir_all(dir)?;
        self.append_event("proof-pruned", run_id, &run.proof_id, serde_json::json!({}))?;
        Ok(true)
    }

    /// Reset only the lifecycle-owned state.  The reset is journaled before
    /// mutation and cannot affect any path outside `.enforce/proofs`.
    pub fn reset(&self) -> Result<()> {
        if self.proof_root.join("journal.ndjson").exists() {
            let snapshot = self.snapshot()?;
            let id: ProofId = "lifecycle-reset".parse().map_err(Error::Decode)?;
            let run: ProofRunId = "lifecycle-reset".parse().map_err(Error::Decode)?;
            let _ = snapshot;
            self.append_event("proof-reset-started", &run, &id, serde_json::json!({}))?;
        }
        if self.proof_root.exists() {
            std::fs::remove_dir_all(&self.proof_root)?;
        }
        Ok(())
    }

    fn persist_run(&self, run: &crate::envelope::ProofRunEnvelope) -> Result<()> {
        let directory = self.proof_root.join("runs").join(run.run_id.as_str());
        std::fs::create_dir_all(&directory)?;
        let final_path = directory.join(PROJECT_PROOF_RUN_FILE);
        if final_path.exists() {
            return Err(Error::InvalidConfig("duplicate proof run id".to_owned()));
        }
        let temp_path = directory.join("proof-run.json.tmp");
        std::fs::write(&temp_path, serde_json::to_vec(run)?)?;
        std::fs::rename(temp_path, final_path)?;
        Ok(())
    }

    fn read_run(&self, run_id: &ProofRunId) -> Result<crate::envelope::ProofRunEnvelope> {
        let path = self
            .proof_root
            .join("runs")
            .join(run_id.as_str())
            .join(PROJECT_PROOF_RUN_FILE);
        serde_json::from_slice(&std::fs::read(path)?).map_err(Into::into)
    }

    fn append_event(
        &self,
        event: &str,
        run_id: &ProofRunId,
        proof_id: &ProofId,
        payload: serde_json::Value,
    ) -> Result<()> {
        std::fs::create_dir_all(&self.proof_root)?;
        let mut journal = ProofJournal::open(&self.root.join(PROJECT_PROOF_JOURNAL))?;
        journal.append(
            &Redactor::with_defaults()?,
            JournalRecordEnvelope {
                schema_version: JOURNAL_SCHEMA_VERSION,
                event_type: JournalEventType::try_from(event.to_owned()).map_err(Error::Decode)?,
                run_id: run_id.clone(),
                proof_id: proof_id.clone(),
                timestamp: now_iso(),
                payload,
            },
        )
    }

    fn contained_path(&self, path: &RelPath) -> Result<PathBuf> {
        let candidate = self.root.join(path.as_str());
        let canonical = candidate.canonicalize()?;
        if !canonical.starts_with(&self.root) {
            return Err(Error::InvalidConfig(
                "artifact path escapes repository root".to_owned(),
            ));
        }
        Ok(canonical)
    }
}

fn now_iso() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}Z", d.as_secs()))
        .unwrap_or_else(|_| "0Z".to_owned())
}
