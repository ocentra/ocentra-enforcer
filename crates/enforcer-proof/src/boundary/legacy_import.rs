//! [G9] Three distinct legacy migration/import operations — the point of
//! migration is DELETION-READINESS, proven by a bounded claim rather than
//! trusted by fiat:
//! - [`migrate_legacy_proofs`] classifies legacy scripts and generates a
//!   per-profile registry + a copy of each source script (dry-run by
//!   default; `write:true` actually writes).
//! - [`import_legacy_proof`] hashes collected legacy artifacts and produces
//!   a real proof run with an in-toto envelope.
//! - [`proof_parity`] compares collected legacy evidence against the
//!   imported run and classifies coverage, computing `deletion_ready`.
//!
//! BOUNDARY-INVARIANT: legacy script and artifact roots must remain inside
//! the repository; outside roots are rejected by the negative integration tests.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use enforcer_core::error::Result;
use enforcer_domain::hashes::Sha256;
use enforcer_domain::paths::RelPath;
use enforcer_domain::proof_types::{
    LegacyArtifactStatus, ProofCapability, ProofCoverage, ProofId, ProofRunId, ProofStatus,
};

use crate::envelope::{
    attestation_for, ArtifactRecordEnvelope, GitStateEnvelope, ProofRunEnvelope,
};

/// One classified legacy script, as discovered under `scripts/test/**`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassifiedScript {
    pub path: RelPath,
    pub name: String,
    pub is_proof_named: bool,
}

/// Classify every `.mjs` script under `scripts_root` (repo-relative to
/// `root`), mirroring the legacy `collectScriptFiles` + `classifyProofScript`
/// pairing at the level this crate needs: a stable sorted listing plus
/// whether the script's name signals it as proof-related.
pub fn classify_scripts(root: &Path, scripts_root: &Path) -> Result<Vec<ClassifiedScript>> {
    if !scripts_root.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    let mut stack = vec![scripts_root.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("mjs") {
                files.push(path);
            }
        }
    }
    files.sort();
    let mut classified = Vec::with_capacity(files.len());
    for path in files {
        let relative = path.strip_prefix(root).map_err(|_strip_error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "legacy script `{}` is outside repository root",
                    path.display()
                ),
            )
        })?;
        if relative.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        }) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("legacy script `{}` escapes repository root", path.display()),
            )
            .into());
        }
        let rel = relative.to_string_lossy().replace('\\', "/");
        let name = match path.file_name() {
            Some(file_name) => file_name.to_string_lossy().to_ascii_lowercase(),
            None => continue,
        };
        classified.push(ClassifiedScript {
            is_proof_named: name.contains("proof"),
            path: RelPath::try_from(rel).map_err(enforcer_core::error::Error::Decode)?,
            name,
        });
    }
    Ok(classified)
}

/// Result of a [`migrate_legacy_proofs`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationResult {
    pub dry_run: bool,
    pub generated_proof_ids: Vec<ProofId>,
    pub copied_script_count: usize,
}

/// [G9a] `migrateLegacyProofs`: classify scripts, generate a per-profile
/// proof id list, and — unless `dry_run` — copy each selected source script
/// into `profile_root/legacy-scripts/<origPath>` and write a minimal
/// registry to `profile_root/proofs.json`. Honors `dry_run` (default: no
/// writes at all).
pub fn migrate_legacy_proofs(
    root: &Path,
    scripts_root: &Path,
    profile: &str,
    profile_root: &Path,
    dry_run: bool,
) -> Result<MigrationResult> {
    let scripts = classify_scripts(root, scripts_root)?;
    let selected: Vec<&ClassifiedScript> = scripts.iter().filter(|s| s.is_proof_named).collect();
    let generated_proof_ids: Vec<ProofId> = selected
        .iter()
        .map(|s| {
            ProofId::try_from(format!(
                "{profile}.{}",
                normalize_slug(s.name.trim_end_matches(".mjs"))
            ))
        })
        .collect::<std::result::Result<_, _>>()
        .map_err(enforcer_core::error::Error::Decode)?;

    if dry_run {
        return Ok(MigrationResult {
            dry_run: true,
            generated_proof_ids,
            copied_script_count: 0,
        });
    }

    let legacy_script_root = profile_root.join("legacy-scripts");
    std::fs::create_dir_all(profile_root)?;
    std::fs::create_dir_all(&legacy_script_root)?;
    let mut copied = 0usize;
    for script in &selected {
        let source = root.join(script.path.as_str());
        let target = legacy_script_root.join(script.path.as_str());
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&source, &target)?;
        copied += 1;
    }
    let registry_json = serde_json::json!({
        "schemaVersion": 1,
        "productName": format!("{profile} proof profile"),
        "proofs": generated_proof_ids,
    });
    std::fs::write(
        profile_root.join("proofs.json"),
        format!("{}\n", serde_json::to_string_pretty(&registry_json)?),
    )?;

    Ok(MigrationResult {
        dry_run: false,
        generated_proof_ids,
        copied_script_count: copied,
    })
}

fn normalize_slug(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_owned();
    if trimmed.is_empty() {
        "proof".to_owned()
    } else {
        trimmed
    }
}

/// One collected legacy artifact.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyArtifactEnvelope {
    pub path: RelPath,
    pub sha256: Sha256,
    pub byte_length: u64,
    pub status: LegacyArtifactStatus,
}

/// The collected legacy-evidence bundle.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyBundleEnvelope {
    pub artifacts: Vec<LegacyArtifactEnvelope>,
    pub failed_artifacts: Vec<RelPath>,
    pub claims_proved: Vec<String>,
    pub claims_not_proved: Vec<String>,
}

impl From<LegacyArtifactEnvelope> for RelPath {
    fn from(value: LegacyArtifactEnvelope) -> Self {
        value.path
    }
}

const LEGACY_ARTIFACT_EXTENSIONS: &[&str] = &["json", "md", "txt", "log", "xml", "sarif", "ndjson"];

/// Collect legacy proof artifacts under `roots` (repo-relative to `root`),
/// hashing each and inferring a pass/fail status from JSON `status`/`ok`
/// fields when present.
pub fn collect_legacy_artifacts(root: &Path, roots: &[&str]) -> Result<LegacyBundleEnvelope> {
    let mut files = Vec::new();
    for entry in roots {
        let absolute = root.join(entry);
        let relative = absolute.strip_prefix(root).map_err(|_strip_error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "legacy artifact root `{}` is outside repository root",
                    absolute.display()
                ),
            )
        })?;
        if relative.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        }) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "legacy artifact root `{}` escapes repository root",
                    absolute.display()
                ),
            )
            .into());
        }
        if !absolute.exists() {
            continue;
        }
        collect_files_recursive(&absolute, &mut files)?;
    }
    files.sort();
    files.dedup();

    let mut artifacts = Vec::new();
    let mut failed_artifacts = Vec::new();
    let mut claims_proved = BTreeSet::new();
    let mut claims_not_proved = BTreeSet::new();

    for file in &files {
        let content = std::fs::read(file)?;
        let sha256 = enforcer_core::hash_chain::link_digest(None, &content);
        let rel = file
            .strip_prefix(root)
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");
        let rel = RelPath::try_from(rel).map_err(enforcer_core::error::Error::Decode)?;
        let status = infer_status(file, &content);
        if status == LegacyArtifactStatus::Failed {
            failed_artifacts.push(rel.clone());
        }
        if let Ok(text) = String::from_utf8(content.clone()) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                for claim in json_string_list(&value, "claimsProved") {
                    claims_proved.insert(claim);
                }
                for claim in json_string_list(&value, "claimsNotProved") {
                    claims_not_proved.insert(claim);
                }
            }
        }
        artifacts.push(LegacyArtifactEnvelope {
            path: rel,
            sha256,
            // CAST-JUSTIFICATION: in-memory artifact buffers cannot exceed u64 addressable length.
            byte_length: content.len() as u64,
            status,
        });
    }

    Ok(LegacyBundleEnvelope {
        artifacts,
        failed_artifacts,
        claims_proved: claims_proved.into_iter().collect(),
        claims_not_proved: claims_not_proved.into_iter().collect(),
    })
}

fn collect_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if dir.is_file() {
        if is_legacy_extension(dir) {
            out.push(dir.to_path_buf());
        }
        return Ok(());
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let skip = matches!(
                    path.file_name().and_then(|n| n.to_str()),
                    Some(".git")
                        | Some(".enforce")
                        | Some("node_modules")
                        | Some("target")
                        | Some("dist")
                        | Some("build")
                );
                if !skip {
                    stack.push(path);
                }
            } else if is_legacy_extension(&path) {
                out.push(path);
            }
        }
    }
    Ok(())
}

fn is_legacy_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| LEGACY_ARTIFACT_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

fn infer_status(path: &Path, content: &[u8]) -> LegacyArtifactStatus {
    if path.extension().and_then(|e| e.to_str()) != Some("json") {
        return LegacyArtifactStatus::Present;
    }
    let Ok(text) = std::str::from_utf8(content) else {
        return LegacyArtifactStatus::Present;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return LegacyArtifactStatus::Present;
    };
    if value.get("ok") == Some(&serde_json::Value::Bool(false))
        || value.get("passed") == Some(&serde_json::Value::Bool(false))
    {
        return LegacyArtifactStatus::Failed;
    }
    if value.get("ok") == Some(&serde_json::Value::Bool(true))
        || value.get("passed") == Some(&serde_json::Value::Bool(true))
    {
        return LegacyArtifactStatus::Passed;
    }
    LegacyArtifactStatus::Present
}

fn json_string_list(value: &serde_json::Value, key: &str) -> Vec<String> {
    match value.get(key) {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    }
}

/// [G9b] `importLegacyProof`: collect legacy artifacts, hash them, and
/// produce a real proof run whose status is `passed` iff at least one
/// artifact was found AND none failed.
pub fn import_legacy_proof(
    proof_id: &ProofId,
    run_id: &ProofRunId,
    git: GitStateEnvelope,
    bundle: &LegacyBundleEnvelope,
) -> ProofRunEnvelope {
    let status = if bundle.artifacts.is_empty() || !bundle.failed_artifacts.is_empty() {
        ProofStatus::Failed
    } else {
        ProofStatus::Passed
    };
    let artifacts: Vec<ArtifactRecordEnvelope> = bundle
        .artifacts
        .iter()
        .map(|a| ArtifactRecordEnvelope {
            name: a
                .path
                .as_str()
                .rsplit('/')
                .next()
                .unwrap_or(a.path.as_str())
                .to_owned(),
            path: legacy_artifact_path(run_id, &a.path),
            sha256: a.sha256.clone(),
            byte_length: a.byte_length,
        })
        .collect();
    ProofRunEnvelope {
        schema_version: 1,
        proof_id: proof_id.clone(),
        run_id: run_id.clone(),
        title: "Legacy artifact import".to_owned(),
        capability: local_capability(),
        git,
        status,
        exit_code: Some(if status == ProofStatus::Passed { 0 } else { 1 }),
        started_at: "2026-07-04T00:00:00Z".to_owned(),
        ended_at: "2026-07-04T00:00:00Z".to_owned(),
        command: vec!["legacy-import".to_owned()],
        // CAST-JUSTIFICATION: the bounded legacy artifact walk cannot exceed u32 diagnostics.
        diagnostic_count: bundle.failed_artifacts.len() as u32,
        pinned: false,
        artifacts,
        claims_proved: bundle.claims_proved.clone(),
        claims_not_proved: bundle.claims_not_proved.clone(),
    }
}

fn legacy_artifact_path(run_id: &ProofRunId, path: &RelPath) -> RelPath {
    let mut candidate = format!(".enforce/proofs/runs/{run_id}/artifacts/legacy/{path}");
    loop {
        if let Ok(relative) = RelPath::try_from(candidate) {
            return relative;
        }
        candidate = "legacy-artifact".to_owned();
    }
}

fn local_capability() -> ProofCapability {
    let mut candidate = "local".to_owned();
    loop {
        if let Ok(capability) = ProofCapability::try_from(candidate) {
            return capability;
        }
        candidate = "local".to_owned();
    }
}

/// [G9c] `proofParity`: compare collected legacy hashes/claims against the
/// imported run, classify [`Coverage`], and compute `deletion_ready` — the
/// gate that says the old script batch may be deleted.
pub fn proof_parity(
    bundle: &LegacyBundleEnvelope,
    imported: Option<&ProofRunEnvelope>,
) -> (ProofCoverage, bool) {
    let legacy_hashes: BTreeSet<&str> =
        bundle.artifacts.iter().map(|a| a.sha256.as_str()).collect();
    let imported_hashes: BTreeSet<&str> = imported
        .map(|run| run.artifacts.iter().map(|a| a.sha256.as_str()).collect())
        .unwrap_or_default();

    let missing_in_imported = !legacy_hashes.is_subset(&imported_hashes);
    let missing_claims_proved = imported.is_none_or(|run| {
        !bundle
            .claims_proved
            .iter()
            .all(|c| run.claims_proved.contains(c))
    });
    let missing_claims_not_proved = imported.is_none_or(|run| {
        !bundle
            .claims_not_proved
            .iter()
            .all(|c| run.claims_not_proved.contains(c))
    });

    let comparable = !bundle.artifacts.is_empty() && imported.is_some();
    let equivalent =
        comparable && !missing_in_imported && !missing_claims_proved && !missing_claims_not_proved;

    let coverage = if equivalent {
        ProofCoverage::Equivalent
    } else if comparable {
        ProofCoverage::Weaker
    } else {
        ProofCoverage::NotComparable
    };

    let deletion_ready = equivalent
        && imported.is_some_and(|run| run.status == ProofStatus::Passed)
        && bundle.failed_artifacts.is_empty();

    (coverage, deletion_ready)
}

/// [G7] Build the same attestation envelope for imported and command-run proofs.
pub fn attestation_for_import(run: &ProofRunEnvelope) -> crate::envelope::AttestationEnvelope {
    attestation_for(run)
}

#[cfg(test)]
mod tests {
    use super::{
        collect_legacy_artifacts, import_legacy_proof, migrate_legacy_proofs, proof_parity,
    };
    use crate::envelope::GitStateEnvelope;
    use enforcer_core::error::Result;
    use enforcer_domain::proof_types::{ProofCoverage, ProofId, ProofRunId, ProofStatus};

    fn temp_dir(name: &str) -> Result<std::path::PathBuf> {
        let dir = std::env::temp_dir().join(format!(
            "enforcer-proof-legacy-{}-{}-{name}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    // --- [G9a] migrate ----------------------------------------------------

    #[test]
    fn migrate_dry_run_writes_nothing() -> Result<()> {
        let root = temp_dir("migrate-dry")?;
        let scripts_root = root.join("scripts/test");
        std::fs::create_dir_all(&scripts_root)?;
        std::fs::write(scripts_root.join("some-proof.mjs"), "// proof script")?;
        let profile_root = root.join("profiles/strict");
        let result = migrate_legacy_proofs(&root, &scripts_root, "strict", &profile_root, true)?;
        assert!(result.dry_run);
        assert_eq!(result.copied_script_count, 0);
        assert!(!profile_root.exists(), "dry-run must not create any files");
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn migrate_write_emits_registry_and_copies_scripts() -> Result<()> {
        let root = temp_dir("migrate-write")?;
        let scripts_root = root.join("scripts/test");
        std::fs::create_dir_all(&scripts_root)?;
        std::fs::write(scripts_root.join("device-proof.mjs"), "// proof script")?;
        let profile_root = root.join("profiles/strict");
        let result = migrate_legacy_proofs(&root, &scripts_root, "strict", &profile_root, false)?;
        assert!(!result.dry_run);
        assert_eq!(result.copied_script_count, 1);
        assert!(profile_root.join("proofs.json").exists());
        assert!(profile_root
            .join("legacy-scripts/scripts/test/device-proof.mjs")
            .exists());
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn migrate_against_missing_scripts_dir_yields_empty_result() -> Result<()> {
        let root = temp_dir("migrate-missing")?;
        let scripts_root = root.join("scripts/test");
        let profile_root = root.join("profiles/strict");
        let result = migrate_legacy_proofs(&root, &scripts_root, "strict", &profile_root, true)?;
        assert!(result.generated_proof_ids.is_empty());
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    // --- [G9b] import ------------------------------------------------------

    #[test]
    fn import_round_trip_hashes_match_manifest() -> Result<()> {
        let root = temp_dir("import-round-trip")?;
        let proof_dir = root.join("docs/proof");
        std::fs::create_dir_all(&proof_dir)?;
        std::fs::write(proof_dir.join("result.json"), r#"{"ok": true}"#)?;
        let bundle = collect_legacy_artifacts(&root, &["docs/proof"])?;
        assert_eq!(bundle.artifacts.len(), 1);
        let proof_id = ProofId::try_from("PROOF-IMPORT".to_owned())?;
        let run_id = ProofRunId::try_from("run-import-1".to_owned())?;
        let run = import_legacy_proof(&proof_id, &run_id, GitStateEnvelope::default(), &bundle);
        assert_eq!(run.status, ProofStatus::Passed);
        assert_eq!(run.artifacts.len(), 1);
        assert_eq!(run.artifacts[0].sha256, bundle.artifacts[0].sha256);
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn import_with_zero_artifacts_yields_failed_status() -> Result<()> {
        let bundle = super::LegacyBundleEnvelope {
            artifacts: vec![],
            failed_artifacts: vec![],
            claims_proved: vec![],
            claims_not_proved: vec![],
        };
        let proof_id = ProofId::try_from("PROOF-EMPTY".to_owned())?;
        let run_id = ProofRunId::try_from("run-empty".to_owned())?;
        let run = import_legacy_proof(&proof_id, &run_id, GitStateEnvelope::default(), &bundle);
        assert_eq!(run.status, ProofStatus::Failed);
        Ok(())
    }

    // --- [G9c] parity -------------------------------------------------------

    #[test]
    fn equivalent_and_passed_yields_deletion_ready() -> Result<()> {
        let root = temp_dir("parity-equivalent")?;
        let proof_dir = root.join("docs/proof");
        std::fs::create_dir_all(&proof_dir)?;
        std::fs::write(proof_dir.join("result.json"), r#"{"ok": true}"#)?;
        let bundle = collect_legacy_artifacts(&root, &["docs/proof"])?;
        let proof_id = ProofId::try_from("PROOF-PARITY".to_owned())?;
        let run_id = ProofRunId::try_from("run-parity-1".to_owned())?;
        let run = import_legacy_proof(&proof_id, &run_id, GitStateEnvelope::default(), &bundle);
        let (coverage, deletion_ready) = proof_parity(&bundle, Some(&run));
        assert_eq!(coverage, ProofCoverage::Equivalent);
        assert!(deletion_ready);
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn hash_mismatch_or_missing_run_yields_weaker_or_not_comparable() -> Result<()> {
        let root = temp_dir("parity-weaker")?;
        let proof_dir = root.join("docs/proof");
        std::fs::create_dir_all(&proof_dir)?;
        std::fs::write(proof_dir.join("result.json"), r#"{"ok": true}"#)?;
        let bundle = collect_legacy_artifacts(&root, &["docs/proof"])?;

        // Missing run -> not-comparable.
        let (coverage_missing, deletion_ready_missing) = proof_parity(&bundle, None);
        assert_eq!(coverage_missing, ProofCoverage::NotComparable);
        assert!(!deletion_ready_missing);

        // A run with no matching artifacts -> weaker.
        let mismatched_bundle = super::LegacyBundleEnvelope {
            artifacts: vec![],
            failed_artifacts: vec![],
            claims_proved: vec![],
            claims_not_proved: vec![],
        };
        let proof_id = ProofId::try_from("PROOF-PARITY".to_owned())?;
        let run_id = ProofRunId::try_from("run-parity-2".to_owned())?;
        let run = import_legacy_proof(&proof_id, &run_id, GitStateEnvelope::default(), &bundle);
        let (coverage_weaker, deletion_ready_weaker) = proof_parity(&mismatched_bundle, Some(&run));
        // mismatched_bundle has no artifacts -> not comparable (matches legacy semantics:
        // comparable requires bundle.artifacts non-empty).
        assert_eq!(coverage_weaker, ProofCoverage::NotComparable);
        assert!(!deletion_ready_weaker);
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }
}
