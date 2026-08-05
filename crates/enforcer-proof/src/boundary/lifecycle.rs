//! BOUNDARY-INVARIANT: this boundary validates typed caller values before every filesystem mutation and keeps all state beneath the repository-owned proof root.
//! Negative invalid-input coverage rejects duplicate run ids, malformed ids, undeclared artifacts, and path escapes.
//! Native, durable proof lifecycle boundary.
//!
//! This is the one writer for project proof state.  It deliberately exposes
//! typed domain inputs and the existing project read-model DTO rather than
//! giving each transport its own JSON persistence implementation.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use enforcer_core::error::{Error, Result};
use enforcer_core::redaction::Redactor;
use enforcer_domain::paths::RelPath;
use enforcer_domain::proof_types::{JournalEventType, ProofId, ProofRunId};

use crate::boundary::proof_query::{ProofInventoryQuery, ProofRouteQuery, ProofStatusQuery};
use crate::boundary::read_model::ProjectProofSnapshotDto;
use crate::harness::{
    merge_proof_definitions, route_proofs, run_proof, ProofDefinitionEnvelope,
    ProofRegistryEnvelope, RouteRequest, RunOutcome, RunProofArgs,
};
use crate::journal::{JournalRecordEnvelope, ProofJournal, JOURNAL_SCHEMA_VERSION};
use crate::read_model::{
    read_project_proof_snapshot, PROJECT_PROOF_JOURNAL, PROJECT_PROOF_RUN_FILE,
};

/// Maximum bytes a caller may read from a declared artifact in one request.
pub const MAX_DECLARED_ARTIFACT_BYTES: u64 = 256 * 1024;
/// The frozen proof query default. Limits are clamped to avoid accidental
/// repository-sized MCP responses.
pub const DEFAULT_PROOF_QUERY_LIMIT: usize = 20;
pub const MAX_PROOF_QUERY_LIMIT: usize = 100;

/// Compact proof definition returned by the proof-route boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutedProofDefinition {
    pub id: String,
    pub title: String,
    pub family: String,
    pub severity: String,
    pub collector: String,
    pub capabilities: Vec<String>,
    pub docs: Vec<String>,
}

/// Native equivalent of the frozen `proof_route` response.
#[derive(Debug, Clone)]
pub struct ProofRouteResult {
    pub product_name: String,
    pub profile_name: String,
    pub index: String,
    pub query: ProofRouteQuery,
    pub docs: Vec<String>,
    pub proofs: Vec<RoutedProofDefinition>,
}

/// Native equivalent of the frozen bounded `proof_status` response.
#[derive(Debug, Clone)]
pub struct ProofStatusResult {
    pub root: PathBuf,
    pub runs: Vec<crate::envelope::ProofRunEnvelope>,
}

/// One deliberately compact legacy script inventory row. The content-derived
/// fields mirror the frozen inventory's useful classification dimensions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofScript {
    pub path: String,
    pub name: String,
    pub family: String,
    pub plan_bucket: String,
    pub proof_types: Vec<String>,
    pub capabilities: Vec<String>,
    pub signals: ProofScriptSignals,
}

/// Content-derived indicators attached to one inventory script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofScriptSignals {
    pub spawn: bool,
    pub writes_proof: bool,
    pub reads_proof: bool,
    pub manual_or_device: bool,
    pub imports_built_or_schema_parse: bool,
}

/// Native equivalent of the frozen inventory aggregate, with script rows
/// opt-in and bounded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofInventoryResult {
    pub root: String,
    pub scripts_root: String,
    pub totals: ProofInventoryTotals,
    pub by_family: BTreeMap<String, usize>,
    pub by_proof_type: BTreeMap<String, usize>,
    pub by_capability: BTreeMap<String, usize>,
    pub script_rows_included: bool,
    pub script_limit: usize,
    pub omitted_script_count: usize,
    pub scripts: Vec<ProofScript>,
}

/// Aggregate counts for the bounded proof-script inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofInventoryTotals {
    pub scripts: usize,
    pub proof_named: usize,
    pub spawn_commands: usize,
    pub writes_proof: usize,
    pub reads_proof: usize,
    pub manual_or_device: usize,
    pub imports_built_or_schema_parse: usize,
}

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
        let enforce_root = root.join(".enforce");
        reject_redirected_state_path(&root, &enforce_root)?;
        let proof_root = enforce_root.join("proofs");
        reject_redirected_state_path(&root, &proof_root)?;
        if proof_root.join("journal.ndjson").exists() {
            ProofJournal::open(&proof_root.join("journal.ndjson"))?;
        }
        Ok(Self { root, proof_root })
    }

    /// Current public read model.  This reuses the crate-owned DTO boundary.
    pub fn snapshot(&self) -> Result<ProjectProofSnapshotDto> {
        read_project_proof_snapshot(&self.root).map(Into::into)
    }

    /// Route against the Rust-owned packaged proof catalog. The target
    /// repository root is deliberately not used as the pack root: callers may
    /// inspect any repository without allowing it to replace Enforcer policy.
    pub fn route(&self, query: &ProofRouteQuery) -> Result<ProofRouteResult> {
        let profile_name = query.profile.as_ref().map_or(
            "strict",
            enforcer_domain::config_types::ConfigProfileName::as_str,
        );
        let registry = load_pack_registry(&resolve_pack_root()?, profile_name)?;
        let request = RouteRequest {
            proof_id: query.proof_id.clone(),
            files: query.files.clone(),
            plan: query.plan.clone(),
            capability: query.capability.clone(),
            scope: query.scope.clone(),
        };
        let routed = route_proofs(&registry, &request);
        let mut docs = routed
            .iter()
            .flat_map(|proof| proof.docs.iter().cloned())
            .collect::<Vec<_>>();
        docs.sort();
        docs.dedup();
        let proofs = routed.into_iter().map(compact_proof).collect();
        Ok(ProofRouteResult {
            product_name: registry.product_name,
            profile_name: profile_name.to_owned(),
            index: "proof/INDEX.md".to_owned(),
            query: query.clone(),
            docs,
            proofs,
        })
    }

    /// Read only persisted, valid run envelopes, then filter and bound before
    /// returning. This intentionally does not project the full read-model
    /// snapshot through the MCP response.
    pub fn status(&self, query: &ProofStatusQuery) -> Result<ProofStatusResult> {
        let mut runs = self.persisted_runs()?;
        runs.retain(|run| {
            query.proof_id.as_ref().is_none_or(|id| &run.proof_id == id)
                && query.status.is_none_or(|status| run.status == status)
        });
        runs.sort_by(|left, right| right.started_at.cmp(&left.started_at));
        runs.truncate(query.limit.min(MAX_PROOF_QUERY_LIMIT));
        Ok(ProofStatusResult {
            root: self.root.clone(),
            runs,
        })
    }

    /// Safely inspect only repository-contained `scripts/test/**/*.mjs` files.
    /// Symlinks are not followed, and optional rows are bounded after stable
    /// lexical ordering, so one query cannot become a repository snapshot.
    pub fn inventory(&self, query: &ProofInventoryQuery) -> Result<ProofInventoryResult> {
        let scripts_root = self.root.join("scripts").join("test");
        let scripts = collect_inventory_scripts(&self.root, &scripts_root)?;
        let mut by_family = BTreeMap::new();
        let mut by_proof_type = BTreeMap::new();
        let mut by_capability = BTreeMap::new();
        let mut totals = ProofInventoryTotals {
            scripts: scripts.len(),
            proof_named: 0,
            spawn_commands: 0,
            writes_proof: 0,
            reads_proof: 0,
            manual_or_device: 0,
            imports_built_or_schema_parse: 0,
        };
        for script in &scripts {
            if script.name.contains("proof") {
                totals.proof_named += 1;
            }
            totals.spawn_commands += usize::from(script.signals.spawn);
            totals.writes_proof += usize::from(script.signals.writes_proof);
            totals.reads_proof += usize::from(script.signals.reads_proof);
            totals.manual_or_device += usize::from(script.signals.manual_or_device);
            totals.imports_built_or_schema_parse +=
                usize::from(script.signals.imports_built_or_schema_parse);
            increment(&mut by_family, &script.family);
            for value in &script.proof_types {
                increment(&mut by_proof_type, value);
            }
            for value in &script.capabilities {
                increment(&mut by_capability, value);
            }
        }
        let limit = query.limit.min(MAX_PROOF_QUERY_LIMIT);
        let selected = if query.include_scripts {
            scripts.iter().take(limit).cloned().collect()
        } else {
            Vec::new()
        };
        Ok(ProofInventoryResult {
            root: self.root.to_string_lossy().into_owned(),
            scripts_root: "scripts/test".to_owned(),
            totals,
            by_family,
            by_proof_type,
            by_capability,
            script_rows_included: query.include_scripts,
            script_limit: if query.include_scripts { limit } else { 0 },
            omitted_script_count: scripts.len().saturating_sub(selected.len()),
            scripts: selected,
        })
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
        let run_directory = self.persist_legacy_artifact_bytes(&bundle, &run)?;
        if let Err(error) = self.import_run(&run) {
            let _ = std::fs::remove_dir_all(run_directory);
            return Err(error);
        }
        Ok(run)
    }

    fn persist_legacy_artifact_bytes(
        &self,
        bundle: &crate::legacy_import::LegacyBundleEnvelope,
        run: &crate::envelope::ProofRunEnvelope,
    ) -> Result<PathBuf> {
        if bundle.artifacts.len() != run.artifacts.len() {
            return Err(Error::InvalidConfig(
                "legacy artifact manifest and run record differ".to_owned(),
            ));
        }
        let runs_root = self.proof_root.join("runs");
        reject_redirected_state_path(&self.root, &runs_root)?;
        std::fs::create_dir_all(&runs_root)?;
        reject_redirected_state_path(&self.root, &runs_root)?;
        let run_directory = runs_root.join(run.run_id.as_str());
        if run_directory.exists() {
            return Err(Error::InvalidConfig("duplicate proof run id".to_owned()));
        }
        std::fs::create_dir(&run_directory)?;
        reject_redirected_state_path(&self.root, &run_directory)?;
        let copy_result = bundle.artifacts.iter().zip(&run.artifacts).try_for_each(
            |(legacy, declared)| -> Result<()> {
                let source = self.root.join(legacy.path.as_str());
                let canonical_source = source.canonicalize()?;
                if !canonical_source.starts_with(&self.root) {
                    return Err(Error::InvalidConfig(
                        "legacy artifact source escapes repository root".to_owned(),
                    ));
                }
                let bytes = std::fs::read(canonical_source)?;
                let digest = enforcer_core::hash_chain::link_digest(None, &bytes);
                if digest != legacy.sha256
                    || u64::try_from(bytes.len()).unwrap_or(u64::MAX) != legacy.byte_length
                    || declared.sha256 != legacy.sha256
                    || declared.byte_length != legacy.byte_length
                {
                    return Err(Error::InvalidConfig(
                        "legacy artifact changed while it was being imported".to_owned(),
                    ));
                }
                let target = self.root.join(declared.path.as_str());
                let parent = target.parent().ok_or_else(|| {
                    Error::InvalidConfig("legacy artifact target has no parent".to_owned())
                })?;
                std::fs::create_dir_all(parent)?;
                std::fs::write(target, bytes)?;
                Ok(())
            },
        );
        if let Err(error) = copy_result {
            let _ = std::fs::remove_dir_all(&run_directory);
            return Err(error);
        }
        Ok(run_directory)
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
        let reservation = self.reserve_run(&args.run_id)?;
        let result = (|| {
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
        })();
        if result.is_err() && !reservation.join(PROJECT_PROOF_RUN_FILE).exists() {
            let _ = std::fs::remove_dir_all(reservation);
        }
        result
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
        let artifact = run
            .artifacts
            .iter()
            .find(|artifact| artifact.path == *path)
            .ok_or_else(|| {
                Error::InvalidConfig("artifact is not declared by this proof run".to_owned())
            })?;
        let target = self.contained_path(path)?;
        let metadata = std::fs::metadata(&target)?;
        if metadata.len() > MAX_DECLARED_ARTIFACT_BYTES {
            return Err(Error::InvalidConfig(
                "declared artifact exceeds read bound".to_owned(),
            ));
        }
        let bytes = std::fs::read(target)?;
        let byte_length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        let digest = enforcer_core::hash_chain::link_digest(None, &bytes);
        if byte_length != artifact.byte_length || digest != artifact.sha256 {
            return Err(Error::InvalidConfig(
                "declared artifact bytes do not match the persisted proof run".to_owned(),
            ));
        }
        let mut value = serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned());
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
        let id: ProofId = "lifecycle-reset".parse().map_err(Error::Decode)?;
        let run: ProofRunId = "lifecycle-reset".parse().map_err(Error::Decode)?;
        self.append_event("proof-reset-started", &run, &id, serde_json::json!({}))?;
        if self.proof_root.exists() {
            for entry in std::fs::read_dir(&self.proof_root)? {
                let entry = entry?;
                if entry.file_name() == std::ffi::OsStr::new("journal.ndjson") {
                    continue;
                }
                if entry.file_type()?.is_dir() {
                    std::fs::remove_dir_all(entry.path())?;
                } else {
                    std::fs::remove_file(entry.path())?;
                }
            }
        }
        self.append_event("proof-reset-finished", &run, &id, serde_json::json!({}))
    }

    fn persist_run(&self, run: &crate::envelope::ProofRunEnvelope) -> Result<()> {
        let runs_root = self.proof_root.join("runs");
        reject_redirected_state_path(&self.root, &runs_root)?;
        let directory = runs_root.join(run.run_id.as_str());
        std::fs::create_dir_all(&directory)?;
        reject_redirected_state_path(&self.root, &runs_root)?;
        reject_redirected_state_path(&self.root, &directory)?;
        let final_path = directory.join(PROJECT_PROOF_RUN_FILE);
        if final_path.exists() {
            return Err(Error::InvalidConfig("duplicate proof run id".to_owned()));
        }
        let temp_path = directory.join("proof-run.json.tmp");
        std::fs::write(&temp_path, serde_json::to_vec(run)?)?;
        std::fs::rename(temp_path, final_path)?;
        Ok(())
    }

    fn reserve_run(&self, run_id: &ProofRunId) -> Result<PathBuf> {
        let runs_root = self.proof_root.join("runs");
        reject_redirected_state_path(&self.root, &runs_root)?;
        std::fs::create_dir_all(&runs_root)?;
        reject_redirected_state_path(&self.root, &runs_root)?;
        let directory = runs_root.join(run_id.as_str());
        match std::fs::create_dir(&directory) {
            Ok(()) => Ok(directory),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                Err(Error::InvalidConfig("duplicate proof run id".to_owned()))
            }
            Err(error) => Err(error.into()),
        }
    }

    fn read_run(&self, run_id: &ProofRunId) -> Result<crate::envelope::ProofRunEnvelope> {
        let path = self
            .proof_root
            .join("runs")
            .join(run_id.as_str())
            .join(PROJECT_PROOF_RUN_FILE);
        serde_json::from_slice(&std::fs::read(path)?).map_err(Into::into)
    }

    fn persisted_runs(&self) -> Result<Vec<crate::envelope::ProofRunEnvelope>> {
        let runs_root = self.proof_root.join("runs");
        if !runs_root.is_dir() {
            return Ok(Vec::new());
        }
        let mut runs = Vec::new();
        for entry in std::fs::read_dir(runs_root)? {
            let entry = entry?;
            let metadata = entry.file_type()?;
            if !metadata.is_dir() || metadata.is_symlink() {
                continue;
            }
            let path = entry.path().join(PROJECT_PROOF_RUN_FILE);
            if path.is_file() {
                runs.push(serde_json::from_slice(&std::fs::read(path)?)?);
            }
        }
        Ok(runs)
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

fn reject_redirected_state_path(root: &Path, path: &Path) -> Result<()> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() || has_reparse_point(&metadata) {
        return Err(Error::InvalidConfig(
            "proof state path must not be a symlink or reparse point".to_owned(),
        ));
    }
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(root) {
        return Err(Error::InvalidConfig(
            "proof state path escapes repository root".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn has_reparse_point(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
const fn has_reparse_point(_metadata: &std::fs::Metadata) -> bool {
    false
}

fn resolve_pack_root() -> Result<PathBuf> {
    let configured = std::env::var_os("ENFORCER_PACK_ROOT").map(PathBuf::from);
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf);
    let working_ancestors = std::env::current_dir()
        .ok()
        .into_iter()
        .flat_map(|cwd| cwd.ancestors().map(Path::to_path_buf).collect::<Vec<_>>());
    configured
        .into_iter()
        .chain(source_root)
        .chain(working_ancestors)
        .find(|candidate| candidate.join("proof").join("proofs.json").is_file())
        .ok_or_else(|| {
            Error::InvalidConfig(
                "cannot locate native proof pack; set ENFORCER_PACK_ROOT to a directory containing proof/proofs.json"
                    .to_owned(),
            )
        })
}

fn load_pack_registry(pack_root: &Path, profile: &str) -> Result<ProofRegistryEnvelope> {
    if !is_safe_profile(profile) {
        return Err(Error::InvalidConfig(
            "invalid proof profile name".to_owned(),
        ));
    }
    let base: ProofRegistryEnvelope =
        serde_json::from_slice(&std::fs::read(pack_root.join("proof").join("proofs.json"))?)?;
    let profile_path = pack_root.join("profiles").join(profile).join("proofs.json");
    if !profile_path.is_file() {
        return Ok(base);
    }
    let overlay: ProofRegistryEnvelope = serde_json::from_slice(&std::fs::read(profile_path)?)?;
    Ok(merge_proof_definitions(&base, &overlay))
}

fn is_safe_profile(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, b'-' | b'_'))
}

fn compact_proof(proof: &ProofDefinitionEnvelope) -> RoutedProofDefinition {
    RoutedProofDefinition {
        id: proof.id.as_str().to_owned(),
        title: proof.title.clone(),
        family: proof.family.as_str().to_owned(),
        severity: match proof.severity {
            enforcer_domain::severity::Severity::Error => "error",
            enforcer_domain::severity::Severity::Warning => "warning",
            enforcer_domain::severity::Severity::Info => "info",
        }
        .to_owned(),
        collector: proof.collector.as_str().to_owned(),
        capabilities: proof
            .capabilities
            .iter()
            .map(|capability| capability.as_str().to_owned())
            .collect(),
        docs: proof.docs.clone(),
    }
}

fn collect_inventory_scripts(root: &Path, scripts_root: &Path) -> Result<Vec<ProofScript>> {
    let metadata = match std::fs::symlink_metadata(scripts_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() || has_reparse_point(&metadata) {
        return Err(Error::InvalidConfig(
            "proof script root must not be a symlink or reparse point".to_owned(),
        ));
    }
    if !metadata.is_dir() {
        return Ok(Vec::new());
    }
    let canonical_scripts_root = scripts_root.canonicalize()?;
    if !canonical_scripts_root.starts_with(root) {
        return Err(Error::InvalidConfig(
            "proof script root escapes repository root".to_owned(),
        ));
    }
    let mut files = Vec::new();
    let mut directories = vec![scripts_root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                directories.push(entry.path());
            } else if file_type.is_file()
                && entry.path().extension().and_then(|value| value.to_str()) == Some("mjs")
            {
                files.push(entry.path());
            }
        }
    }
    files.sort();
    files
        .into_iter()
        .map(|path| classify_inventory_script(root, &path))
        .collect()
}

fn classify_inventory_script(root: &Path, path: &Path) -> Result<ProofScript> {
    let relative = path.strip_prefix(root).map_err(|error| {
        Error::InvalidConfig(format!("proof script is outside repository root: {error}"))
    })?;
    let path =
        RelPath::try_from(relative.to_string_lossy().replace('\\', "/")).map_err(Error::Decode)?;
    let name = path
        .as_str()
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let source = std::fs::read_to_string(root.join(path.as_str()))?;
    let lower = format!("{name}\n{source}").to_ascii_lowercase();
    let signals = ProofScriptSignals {
        spawn: source.contains("spawn") || source.contains("execFile") || source.contains("exec("),
        writes_proof: source.contains("writeFile")
            || source.contains("appendFile")
            || source.contains("writeJson"),
        reads_proof: source.contains("readJson")
            || source.contains("loadProof")
            || (source.contains("readFile")
                && (source.contains("proof") || source.contains("test-results"))),
        manual_or_device: lower.contains("manual-required")
            || lower.contains("physical")
            || lower.contains("android_serial")
            || lower.contains("adb")
            || lower.contains("ios")
            || lower.contains("simulator")
            || lower.contains("device"),
        imports_built_or_schema_parse: source.contains("dist/")
            || source.contains("await import")
            || source.contains("Schema.parse")
            || source.contains(".parse("),
    };
    let capabilities = inventory_capabilities(&lower);
    let proof_types = inventory_proof_types(&name, &source, &lower, &signals);
    Ok(ProofScript {
        path: path.as_str().to_owned(),
        name,
        family: inventory_family(&lower),
        plan_bucket: inventory_plan_bucket(&path),
        proof_types,
        capabilities,
        signals,
    })
}

fn inventory_family(lower: &str) -> String {
    if [
        "android",
        "ios",
        "device",
        "physical",
        "simulator",
        "xcode",
        "adb",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        return "device-manual".to_owned();
    }
    if [
        "junit",
        "pytest",
        "vitest",
        "jest",
        "playwright",
        "test-results",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        return "test-report".to_owned();
    }
    if lower.contains("sarif")
        || lower.contains("codeql")
        || lower.contains("security")
        || lower.contains("secret")
        || lower.contains("audit")
    {
        return "security-report".to_owned();
    }
    if lower.contains("parity")
        || lower.contains("contract")
        || lower.contains("boundary")
        || lower.contains("schema")
    {
        return "contract-parity".to_owned();
    }
    if lower.contains("event")
        || lower.contains("network")
        || lower.contains("lan")
        || lower.contains("message")
        || lower.contains("codec")
    {
        return "event-network".to_owned();
    }
    "command".to_owned()
}

fn inventory_capabilities(lower: &str) -> Vec<String> {
    let mut capabilities = vec!["local".to_owned()];
    for value in [
        "ci", "windows", "linux", "macos", "wsl", "browser", "network", "cloud",
    ] {
        if lower.contains(value) {
            capabilities.push(value.to_owned());
        }
    }
    if lower.contains("android") {
        capabilities.push(
            if lower.contains("emulator") {
                "android-emulator"
            } else {
                "android-device"
            }
            .to_owned(),
        );
    }
    if lower.contains("ios") {
        capabilities.push(
            if lower.contains("ios-device") {
                "ios-device"
            } else {
                "ios-simulator"
            }
            .to_owned(),
        );
    }
    if lower.contains("manual-required") || lower.contains("physical") {
        capabilities.push("manual-required".to_owned());
    }
    capabilities.sort();
    capabilities.dedup();
    capabilities
}

fn inventory_proof_types(
    name: &str,
    source: &str,
    lower: &str,
    signals: &ProofScriptSignals,
) -> Vec<String> {
    let mut values = Vec::new();
    if signals.manual_or_device {
        values.push("manual-evidence".to_owned());
    }
    if source.contains("claimsProved")
        || source.contains("claimsNotProved")
        || source.contains("mustNotClaim")
    {
        values.push("claim-integrity".to_owned());
    }
    if lower.contains("parity")
        || lower.contains("contract")
        || lower.contains("schema")
        || source.contains("Schema.parse")
    {
        values.push("contract-parity".to_owned());
    }
    if name.contains("event")
        || name.contains("network")
        || name.contains("lan")
        || name.contains("message")
    {
        values.push("runtime-event-contract".to_owned());
    }
    if lower.contains("sarif")
        || lower.contains("codeql")
        || lower.contains("security")
        || lower.contains("secret")
        || lower.contains("audit")
    {
        values.push("security-report".to_owned());
    }
    if lower.contains("vitest")
        || lower.contains("playwright")
        || lower.contains("cargo test")
        || lower.contains("npm run test")
    {
        values.push("test-report".to_owned());
    }
    if signals.spawn {
        values.push("command-execution".to_owned());
    }
    if signals.writes_proof {
        values.push("artifact-snapshot".to_owned());
    }
    if values.is_empty() {
        values.push("command-execution".to_owned());
    }
    values.sort();
    values.dedup();
    values
}

fn inventory_plan_bucket(path: &RelPath) -> String {
    path.as_str()
        .trim_end_matches(".mjs")
        .split('/')
        .next_back()
        .map(|name| name.split('-').take(2).collect::<Vec<_>>().join("-"))
        .filter(|bucket| !bucket.is_empty())
        .unwrap_or_else(|| "unclassified".to_owned())
}

fn increment(values: &mut BTreeMap<String, usize>, key: &str) {
    *values.entry(key.to_owned()).or_default() += 1;
}

fn now_iso() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}Z", d.as_secs()))
        .unwrap_or_else(|_| "0Z".to_owned())
}

#[cfg(test)]
mod tests {
    use super::load_pack_registry;
    use enforcer_core::error::{Error, Result};

    #[test]
    fn pack_catalog_merges_a_safe_profile_over_the_base_definition() -> Result<()> {
        let fixture = tempfile::tempdir()?;
        let proof = fixture.path().join("proof");
        let profile = fixture.path().join("profiles").join("strict");
        std::fs::create_dir_all(&proof)?;
        std::fs::create_dir_all(&profile)?;
        std::fs::write(
            proof.join("proofs.json"),
            r#"{"schemaVersion":1,"productName":"base","proofs":[{"id":"proof.one","title":"base","family":"command","severity":"error","collector":"command"}]}"#,
        )?;
        std::fs::write(
            profile.join("proofs.json"),
            r#"{"schemaVersion":2,"productName":"ignored","proofs":[{"id":"proof.one","title":"profile","family":"command","severity":"warning","collector":"command"}]}"#,
        )?;
        let catalog = load_pack_registry(fixture.path(), "strict")?;
        assert_eq!(catalog.schema_version, 2);
        assert_eq!(catalog.product_name, "base");
        assert_eq!(catalog.proofs.len(), 1);
        assert_eq!(catalog.proofs[0].title, "profile");
        assert!(matches!(
            load_pack_registry(fixture.path(), "../escape"),
            Err(Error::InvalidConfig(_))
        ));
        Ok(())
    }
}
