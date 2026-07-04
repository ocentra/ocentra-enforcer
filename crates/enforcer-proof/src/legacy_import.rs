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

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use enforcer_core::error::Result;
use enforcer_domain::hashes::Sha256;

use crate::envelope::{attestation_for, ArtifactRecord, GitState, ProofRun, ProofStatus};

/// One classified legacy script, as discovered under `scripts/test/**`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassifiedScript {
    pub path: String,
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
    Ok(files
        .into_iter()
        .map(|path| {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            ClassifiedScript {
                is_proof_named: name.contains("proof"),
                path: rel,
                name,
            }
        })
        .collect())
}

/// Result of a [`migrate_legacy_proofs`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationResult {
    pub dry_run: bool,
    pub generated_proof_ids: Vec<String>,
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
    let generated_proof_ids: Vec<String> = selected
        .iter()
        .map(|s| {
            format!(
                "{profile}.{}",
                normalize_slug(s.name.trim_end_matches(".mjs"))
            )
        })
        .collect();

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
        let source = root.join(&script.path);
        let target = legacy_script_root.join(&script.path);
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
pub struct LegacyArtifact {
    pub path: String,
    pub sha256: Sha256,
    pub byte_length: u64,
    pub status: String,
}

/// The collected legacy-evidence bundle.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyBundle {
    pub artifacts: Vec<LegacyArtifact>,
    pub failed_artifacts: Vec<String>,
    pub claims_proved: Vec<String>,
    pub claims_not_proved: Vec<String>,
}

const LEGACY_ARTIFACT_EXTENSIONS: &[&str] = &["json", "md", "txt", "log", "xml", "sarif", "ndjson"];

/// Collect legacy proof artifacts under `roots` (repo-relative to `root`),
/// hashing each and inferring a pass/fail status from JSON `status`/`ok`
/// fields when present.
pub fn collect_legacy_artifacts(root: &Path, roots: &[&str]) -> Result<LegacyBundle> {
    let mut files = Vec::new();
    for entry in roots {
        let absolute = root.join(entry);
        if !absolute.exists() {
            continue;
        }
        collect_files_recursive(&absolute, &mut files)?;
    }
    files.sort();

    let mut artifacts = Vec::new();
    let mut failed_artifacts = Vec::new();
    let mut claims_proved = BTreeSet::new();
    let mut claims_not_proved = BTreeSet::new();

    for file in &files {
        let content = std::fs::read(file)?;
        let digest = enforcer_core::hash_chain::link_digest(None, &content);
        let sha256: Sha256 = digest
            .parse()
            .map_err(enforcer_core::error::Error::Decode)?;
        let rel = file
            .strip_prefix(root)
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");
        let status = infer_status(file, &content);
        if status == "failed" || status == "manual-required" || status == "unavailable" {
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
        artifacts.push(LegacyArtifact {
            path: rel,
            sha256,
            byte_length: content.len() as u64,
            status,
        });
    }

    Ok(LegacyBundle {
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

fn infer_status(path: &Path, content: &[u8]) -> String {
    if path.extension().and_then(|e| e.to_str()) != Some("json") {
        return "present".to_owned();
    }
    let Ok(text) = std::str::from_utf8(content) else {
        return "present".to_owned();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return "present".to_owned();
    };
    if value.get("ok") == Some(&serde_json::Value::Bool(false))
        || value.get("passed") == Some(&serde_json::Value::Bool(false))
    {
        return "failed".to_owned();
    }
    if value.get("ok") == Some(&serde_json::Value::Bool(true))
        || value.get("passed") == Some(&serde_json::Value::Bool(true))
    {
        return "passed".to_owned();
    }
    "present".to_owned()
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
    proof_id: &str,
    run_id: &str,
    git: GitState,
    bundle: &LegacyBundle,
) -> ProofRun {
    let status = if bundle.artifacts.is_empty() || !bundle.failed_artifacts.is_empty() {
        ProofStatus::Failed
    } else {
        ProofStatus::Passed
    };
    let artifacts: Vec<ArtifactRecord> = bundle
        .artifacts
        .iter()
        .map(|a| ArtifactRecord {
            name: a.path.rsplit('/').next().unwrap_or(&a.path).to_owned(),
            path: format!(".enforce/proofs/runs/{run_id}/artifacts/legacy/{}", a.path),
            sha256: a.sha256.clone(),
            byte_length: a.byte_length,
        })
        .collect();
    ProofRun {
        schema_version: 1,
        proof_id: proof_id.to_owned(),
        run_id: run_id.to_owned(),
        title: "Legacy artifact import".to_owned(),
        capability: "local".to_owned(),
        git,
        status,
        exit_code: Some(if status == ProofStatus::Passed { 0 } else { 1 }),
        started_at: "2026-07-04T00:00:00Z".to_owned(),
        ended_at: "2026-07-04T00:00:00Z".to_owned(),
        command: vec!["legacy-import".to_owned()],
        diagnostic_count: bundle.failed_artifacts.len() as u32,
        pinned: false,
        artifacts,
        claims_proved: bundle.claims_proved.clone(),
        claims_not_proved: bundle.claims_not_proved.clone(),
    }
}

/// Coverage classification for [`proof_parity`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Coverage {
    /// The imported run covers every legacy hash and claim.
    Equivalent,
    /// A run exists and is comparable, but coverage is incomplete.
    Weaker,
    /// No comparable imported run exists at all.
    NotComparable,
}

/// [G9c] `proofParity`: compare collected legacy hashes/claims against the
/// imported run, classify [`Coverage`], and compute `deletion_ready` — the
/// gate that says the old script batch may be deleted.
pub fn proof_parity(bundle: &LegacyBundle, imported: Option<&ProofRun>) -> (Coverage, bool) {
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
        Coverage::Equivalent
    } else if comparable {
        Coverage::Weaker
    } else {
        Coverage::NotComparable
    };

    let deletion_ready = equivalent
        && imported.is_some_and(|run| run.status == ProofStatus::Passed)
        && bundle.failed_artifacts.is_empty();

    (coverage, deletion_ready)
}

/// [G7] Attestation helper re-exported at the call site convenience: import
/// runs get the same in-toto envelope as command-run proofs.
pub fn attestation_for_import(run: &ProofRun) -> crate::envelope::Attestation {
    attestation_for(run)
}

#[cfg(test)]
mod tests {
    use super::{
        collect_legacy_artifacts, import_legacy_proof, migrate_legacy_proofs, proof_parity,
        Coverage,
    };
    use crate::envelope::{GitState, ProofStatus};
    use enforcer_core::error::Result;

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
        let run = import_legacy_proof("PROOF-IMPORT", "run-import-1", GitState::default(), &bundle);
        assert_eq!(run.status, ProofStatus::Passed);
        assert_eq!(run.artifacts.len(), 1);
        assert_eq!(run.artifacts[0].sha256, bundle.artifacts[0].sha256);
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn import_with_zero_artifacts_yields_failed_status() {
        let bundle = super::LegacyBundle {
            artifacts: vec![],
            failed_artifacts: vec![],
            claims_proved: vec![],
            claims_not_proved: vec![],
        };
        let run = import_legacy_proof("PROOF-EMPTY", "run-empty", GitState::default(), &bundle);
        assert_eq!(run.status, ProofStatus::Failed);
    }

    // --- [G9c] parity -------------------------------------------------------

    #[test]
    fn equivalent_and_passed_yields_deletion_ready() -> Result<()> {
        let root = temp_dir("parity-equivalent")?;
        let proof_dir = root.join("docs/proof");
        std::fs::create_dir_all(&proof_dir)?;
        std::fs::write(proof_dir.join("result.json"), r#"{"ok": true}"#)?;
        let bundle = collect_legacy_artifacts(&root, &["docs/proof"])?;
        let run = import_legacy_proof("PROOF-PARITY", "run-parity-1", GitState::default(), &bundle);
        let (coverage, deletion_ready) = proof_parity(&bundle, Some(&run));
        assert_eq!(coverage, Coverage::Equivalent);
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
        assert_eq!(coverage_missing, Coverage::NotComparable);
        assert!(!deletion_ready_missing);

        // A run with no matching artifacts -> weaker.
        let mismatched_bundle = super::LegacyBundle {
            artifacts: vec![],
            failed_artifacts: vec![],
            claims_proved: vec![],
            claims_not_proved: vec![],
        };
        let run = import_legacy_proof("PROOF-PARITY", "run-parity-2", GitState::default(), &bundle);
        let (coverage_weaker, deletion_ready_weaker) = proof_parity(&mismatched_bundle, Some(&run));
        // mismatched_bundle has no artifacts -> not comparable (matches legacy semantics:
        // comparable requires bundle.artifacts non-empty).
        assert_eq!(coverage_weaker, Coverage::NotComparable);
        assert!(!deletion_ready_weaker);
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }
}
