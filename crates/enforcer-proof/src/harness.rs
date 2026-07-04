//! The proof harness: the `proofs.json` registry model + merge + routing
//! (G10), running a routed proof and capturing its artifact + freshness,
//! the manual-required/unavailable capability model + `PROOF-MANUAL`
//! diagnostic (G11), and the run-store retention pruning that enforces
//! [`crate::envelope::RetentionPolicy`] (G6).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use enforcer_core::error::Result;

use crate::envelope::{git_state, ArtifactRecord, ProofRun, ProofStatus, RetentionPolicy};

/// [G10] One proof definition from the `proofs.json` registry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofDefinition {
    pub id: String,
    pub title: String,
    pub family: String,
    pub severity: String,
    #[serde(default)]
    pub applies_to: Vec<String>,
    #[serde(default)]
    pub triggers: Vec<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub collector: String,
    #[serde(default)]
    pub docs: Vec<String>,
    #[serde(default)]
    pub commands: Vec<Vec<String>>,
    #[serde(default)]
    pub required_artifacts: Vec<String>,
    #[serde(default)]
    pub required_paths: Vec<String>,
    #[serde(default)]
    pub required_for_pr_ready: bool,
    #[serde(default)]
    pub claims_proved: Vec<String>,
    #[serde(default)]
    pub claims_not_proved: Vec<String>,
    #[serde(default)]
    pub ci_support: bool,
    #[serde(default)]
    pub device_support: bool,
}

/// The whole registry: schema version + product name + proof list.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofRegistry {
    pub schema_version: u32,
    pub product_name: String,
    pub proofs: Vec<ProofDefinition>,
}

/// [G10] Deep-merge a profile registry over a base registry by `id`: a
/// profile entry REPLACES the same-id base entry; `schema_version` becomes
/// the max of the two.
pub fn merge_proof_definitions(base: &ProofRegistry, profile: &ProofRegistry) -> ProofRegistry {
    let mut merged: BTreeMap<String, ProofDefinition> = BTreeMap::new();
    for proof in &base.proofs {
        merged.insert(proof.id.clone(), proof.clone());
    }
    for proof in &profile.proofs {
        merged.insert(proof.id.clone(), proof.clone());
    }
    ProofRegistry {
        schema_version: base.schema_version.max(profile.schema_version),
        product_name: base.product_name.clone(),
        proofs: merged.into_values().collect(),
    }
}

/// A proof-route request: either an explicit proof id, or a family-key
/// derived route (files/plan/capability/scope).
#[derive(Debug, Clone, Default)]
pub struct RouteRequest {
    pub proof_id: Option<String>,
    pub files: Vec<String>,
    pub plan: Option<String>,
    pub capability: Option<String>,
    pub scope: Option<String>,
}

/// [G10] Derive the family-key set a route request matches against.
pub fn proof_family_keys(request: &RouteRequest) -> Vec<String> {
    let mut keys = Vec::new();
    if let Some(plan) = &request.plan {
        keys.push(format!("plan:{plan}"));
    }
    if let Some(capability) = &request.capability {
        keys.push(format!("capability:{capability}"));
    }
    if request.scope.as_deref() == Some("workspace") {
        keys.push("scope:workspace".to_owned());
    }
    for file in &request.files {
        keys.extend(family_keys_for_file(file));
    }
    keys
}

fn family_keys_for_file(file: &str) -> Vec<String> {
    let lower = file.replace('\\', "/").to_ascii_lowercase();
    let mut keys = Vec::new();
    if lower.ends_with(".rs") || lower.contains("cargo.toml") {
        keys.push("language:rust".to_owned());
    }
    if lower.ends_with(".ts")
        || lower.ends_with(".tsx")
        || lower.ends_with(".js")
        || lower.ends_with("package.json")
    {
        keys.push("language:typescript".to_owned());
    }
    if lower.ends_with(".py") || lower.ends_with("pyproject.toml") {
        keys.push("language:python".to_owned());
    }
    if lower.starts_with("scripts/test/") || lower.contains("proof") {
        keys.push("kind:proof-script".to_owned());
    }
    keys
}

/// [G10] Whether `definition` matches `request` given the derived
/// `family_keys`. Mirrors the legacy `proofMatchesRoute`: an explicit
/// capability filter that does not match excludes the proof outright; a
/// `scope:workspace` route matches proofs whose `appliesTo` contains
/// `workspace` OR whose family is `claim-integrity`; otherwise any
/// family/language/capability/trigger/appliesTo key hit matches.
pub fn proof_matches_route(
    definition: &ProofDefinition,
    request: &RouteRequest,
    family_keys: &[String],
) -> bool {
    if let Some(capability) = &request.capability {
        if !definition.capabilities.iter().any(|c| c == capability) {
            return false;
        }
    }
    if let Some(plan) = &request.plan {
        let haystack: Vec<&str> = definition
            .applies_to
            .iter()
            .chain(definition.triggers.iter())
            .map(String::as_str)
            .collect();
        if !haystack
            .iter()
            .any(|v| *v == plan || v.ends_with(&format!(":{plan}")) || v.contains(plan.as_str()))
        {
            return false;
        }
    }
    if family_keys.is_empty() {
        return false;
    }
    if family_keys.iter().any(|k| k == "scope:workspace") {
        return definition.applies_to.iter().any(|a| a == "workspace")
            || definition.family == "claim-integrity";
    }
    let mut proof_keys: Vec<String> = vec![format!("family:{}", definition.family)];
    proof_keys.extend(definition.languages.iter().map(|l| format!("language:{l}")));
    proof_keys.extend(
        definition
            .capabilities
            .iter()
            .map(|c| format!("capability:{c}")),
    );
    proof_keys.extend(definition.triggers.iter().cloned());
    proof_keys.extend(definition.applies_to.iter().cloned());
    family_keys.iter().any(|k| proof_keys.contains(k))
}

/// [G10] Route: explicit `proofId` selects that one definition (if present);
/// otherwise every definition whose family keys the request matches.
pub fn route_proofs<'a>(
    registry: &'a ProofRegistry,
    request: &RouteRequest,
) -> Vec<&'a ProofDefinition> {
    if let Some(proof_id) = &request.proof_id {
        return registry
            .proofs
            .iter()
            .filter(|p| &p.id == proof_id)
            .collect();
    }
    let family_keys = proof_family_keys(request);
    registry
        .proofs
        .iter()
        .filter(|p| proof_matches_route(p, request, &family_keys))
        .collect()
}

/// Arguments to run one proof.
#[derive(Debug, Clone, Default)]
pub struct RunProofArgs {
    pub proof_id: String,
    pub root: PathBuf,
    pub run_id: String,
    pub command: Vec<String>,
    pub capability: Option<String>,
    pub claims_proved: Vec<String>,
    pub claims_not_proved: Vec<String>,
    pub pin: bool,
}

/// One diagnostic emitted by the harness (mirrors the legacy manual-run
/// diagnostic shape; `PROOF-MANUAL` is the fixed rule id for
/// manual-required/unavailable states per [G11]).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofDiagnostic {
    pub run_id: String,
    pub proof_id: String,
    pub severity: String,
    pub rule_id: String,
    pub message: String,
    pub file: String,
    pub line: u32,
}

/// Result of running (or attempting to run) one proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutcome {
    pub ok: bool,
    pub proof_run: ProofRun,
    pub diagnostics: Vec<ProofDiagnostic>,
}

/// [G11] Resolve the effective capability for a run: `args.capability`,
/// else the definition's first declared capability, else `"local"`.
pub fn resolve_capability(
    args_capability: Option<&str>,
    definition: Option<&ProofDefinition>,
) -> String {
    args_capability
        .map(str::to_owned)
        .or_else(|| definition.and_then(|d| d.capabilities.first().cloned()))
        .unwrap_or_else(|| "local".to_owned())
}

/// [G11] Run a proof. An empty `command` triggers the manual-required /
/// unavailable path: `manual-required` when the resolved capability is
/// `manual-required` OR the definition's collector is `manual-artifact`,
/// else `unavailable` — either way with a single `PROOF-MANUAL` diagnostic
/// (`warning` for manual-required, `error` for unavailable) and `ok:false`.
/// A non-empty command actually runs and is scored `passed`/`failed` by
/// exit code.
pub fn run_proof(args: &RunProofArgs, definition: Option<&ProofDefinition>) -> Result<RunOutcome> {
    let capability = resolve_capability(args.capability.as_deref(), definition);
    let git = git_state(&args.root);
    let started_at = now_iso();

    if args.command.is_empty() {
        let manual_required = capability == "manual-required"
            || definition.is_some_and(|d| d.collector == "manual-artifact");
        let status = if manual_required {
            ProofStatus::ManualRequired
        } else {
            ProofStatus::Unavailable
        };
        let diagnostic = ProofDiagnostic {
            run_id: args.run_id.clone(),
            proof_id: args.proof_id.clone(),
            severity: if manual_required { "warning" } else { "error" }.to_owned(),
            rule_id: "PROOF-MANUAL".to_owned(),
            message: "No executable command was provided; proof requires external/manual evidence."
                .to_owned(),
            file: ".".to_owned(),
            line: 1,
        };
        let ended_at = now_iso();
        let proof_run = ProofRun {
            schema_version: 1,
            proof_id: args.proof_id.clone(),
            run_id: args.run_id.clone(),
            title: definition
                .map(|d| d.title.clone())
                .unwrap_or_else(|| args.proof_id.clone()),
            capability,
            git,
            status,
            exit_code: None,
            started_at,
            ended_at,
            command: vec![],
            diagnostic_count: 1,
            pinned: args.pin,
            artifacts: vec![],
            claims_proved: args.claims_proved.clone(),
            claims_not_proved: args.claims_not_proved.clone(),
        };
        return Ok(RunOutcome {
            ok: false,
            proof_run,
            diagnostics: vec![diagnostic],
        });
    }

    let output = Command::new(&args.command[0])
        .args(&args.command[1..])
        .current_dir(&args.root)
        .output()?;
    let ended_at = now_iso();
    let exit_code = output.status.code().unwrap_or(1);
    let status = if exit_code == 0 {
        ProofStatus::Passed
    } else {
        ProofStatus::Failed
    };

    let proof_run = ProofRun {
        schema_version: 1,
        proof_id: args.proof_id.clone(),
        run_id: args.run_id.clone(),
        title: definition
            .map(|d| d.title.clone())
            .unwrap_or_else(|| args.proof_id.clone()),
        capability,
        git,
        status,
        exit_code: Some(exit_code),
        started_at,
        ended_at,
        command: args.command.clone(),
        diagnostic_count: 0,
        pinned: args.pin,
        artifacts: vec![],
        claims_proved: args.claims_proved.clone(),
        claims_not_proved: args.claims_not_proved.clone(),
    };
    Ok(RunOutcome {
        ok: matches!(status, ProofStatus::Passed),
        proof_run,
        diagnostics: vec![],
    })
}

fn now_iso() -> String {
    // Windows-first, dependency-light: format via SystemTime + a fixed
    // ISO-8601 UTC render (seconds resolution is sufficient for a run
    // timestamp; sub-second ordering is not relied on anywhere in this
    // crate).
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (hour, minute, second) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (year, month, day) = civil_from_days(days as i64);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Days-since-epoch to civil (year, month, day), Howard Hinnant's
/// `civil_from_days` algorithm (proleptic Gregorian, no external crate).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

/// [G6] Manifest row (mirrors the legacy `db/proof-manifest.json` shape)
/// for run listing/pruning.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestRow {
    pub run_id: String,
    pub proof_id: String,
    pub status: ProofStatus,
    pub started_at: String,
    pub pinned: bool,
}

/// [G6] Prune `runs` under `policy`, returning the run ids to remove.
/// Keeps: pinned runs within their `pin_pr_ready_days` grace period; the
/// newest `max_runs_per_proof` runs per proof id; the newest
/// `max_failed_runs` non-passed runs overall. A run older than
/// `prune_after_days` is removed UNLESS a keep rule above still applies.
pub fn prune_runs(
    runs: &[ManifestRow],
    policy: RetentionPolicy,
    now_days_since_epoch: f64,
    day_of: impl Fn(&str) -> Option<f64>,
) -> Vec<String> {
    let mut sorted: Vec<&ManifestRow> = runs.iter().collect();
    sorted.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    let mut keep: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut age_prunable: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    for run in &sorted {
        if let Some(started_days) = day_of(&run.started_at) {
            let age = now_days_since_epoch - started_days;
            if policy.prunable_by_age(run.pinned, age) {
                age_prunable.insert(run.run_id.clone());
            } else if run.pinned {
                keep.insert(run.run_id.clone());
            }
        } else if run.pinned {
            keep.insert(run.run_id.clone());
        }
    }

    let mut by_proof: BTreeMap<&str, Vec<&ManifestRow>> = BTreeMap::new();
    for run in &sorted {
        by_proof.entry(run.proof_id.as_str()).or_default().push(run);
    }
    for group in by_proof.values() {
        for run in group.iter().take(policy.max_runs_per_proof as usize) {
            keep.insert(run.run_id.clone());
        }
    }

    let failed_runs: Vec<&&ManifestRow> = sorted
        .iter()
        .filter(|r| !matches!(r.status, ProofStatus::Passed))
        .collect();
    for run in failed_runs.iter().take(policy.max_failed_runs as usize) {
        keep.insert(run.run_id.clone());
    }

    sorted
        .iter()
        .filter(|r| age_prunable.contains(&r.run_id) && !keep.contains(&r.run_id))
        .map(|r| r.run_id.clone())
        .collect()
}

/// List every artifact file discovered under a run's `artifacts/` directory
/// plus the fixed files always expected (summary/events/diagnostics/raw
/// logs/attestation), hashing each into an [`ArtifactRecord`].
pub fn collect_artifact_records(run_dir: &Path, root: &Path) -> Result<Vec<ArtifactRecord>> {
    let mut names: Vec<String> = vec![
        "summary.md".to_owned(),
        "attestation.json".to_owned(),
        "proof-run.json".to_owned(),
    ];
    let artifact_root = run_dir.join("artifacts");
    if artifact_root.exists() {
        let mut stack = vec![artifact_root];
        while let Some(current) = stack.pop() {
            for entry in std::fs::read_dir(&current)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Ok(rel) = path.strip_prefix(run_dir) {
                    names.push(rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
    }
    names.sort();
    names.dedup();
    let mut records = Vec::new();
    for name in names {
        let absolute = run_dir.join(&name);
        if !absolute.exists() {
            continue;
        }
        let content = std::fs::read(&absolute)?;
        let digest = enforcer_core::hash_chain::link_digest(None, &content);
        let sha256: enforcer_domain::hashes::Sha256 = digest
            .parse()
            .map_err(enforcer_core::error::Error::Decode)?;
        let rel_to_root = absolute
            .strip_prefix(root)
            .unwrap_or(&absolute)
            .to_string_lossy()
            .replace('\\', "/");
        records.push(ArtifactRecord {
            name,
            path: rel_to_root,
            sha256,
            byte_length: content.len() as u64,
        });
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::{
        merge_proof_definitions, prune_runs, resolve_capability, route_proofs, run_proof,
        ManifestRow, ProofDefinition, ProofRegistry, ProofStatus, RouteRequest, RunProofArgs,
    };
    use crate::envelope::DEFAULT_PROOF_RETENTION;
    use enforcer_core::error::Result;

    fn definition(id: &str, family: &str) -> ProofDefinition {
        ProofDefinition {
            id: id.to_owned(),
            title: format!("Title for {id}"),
            family: family.to_owned(),
            severity: "error".to_owned(),
            applies_to: vec!["workspace".to_owned()],
            triggers: vec!["kind:proof-script".to_owned()],
            languages: vec!["rust".to_owned()],
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

    // --- [G10] registry merge + routing ---------------------------------

    #[test]
    fn profile_override_wins_over_same_id_base_entry() {
        let base = ProofRegistry {
            schema_version: 1,
            product_name: "base".to_owned(),
            proofs: vec![definition("shared.proof", "command")],
        };
        let mut profile_def = definition("shared.proof", "device-manual");
        profile_def.title = "Profile override".to_owned();
        let profile = ProofRegistry {
            schema_version: 2,
            product_name: "profile".to_owned(),
            proofs: vec![profile_def],
        };
        let merged = merge_proof_definitions(&base, &profile);
        assert_eq!(merged.schema_version, 2);
        let found = merged.proofs.iter().find(|p| p.id == "shared.proof");
        assert_eq!(found.map(|p| p.title.as_str()), Some("Profile override"));
        assert_eq!(found.map(|p| p.family.as_str()), Some("device-manual"));
    }

    #[test]
    fn rust_file_routes_to_rust_triggered_proof() {
        let registry = ProofRegistry {
            schema_version: 1,
            product_name: "p".to_owned(),
            proofs: vec![definition("rust.proof", "command")],
        };
        let request = RouteRequest {
            files: vec!["crates/enforcer-proof/src/lib.rs".to_owned()],
            ..Default::default()
        };
        let routed = route_proofs(&registry, &request);
        assert_eq!(routed.len(), 1);
        assert_eq!(routed[0].id, "rust.proof");
    }

    #[test]
    fn workspace_scope_routes_to_claim_integrity_proof() {
        let mut claim_integrity = definition("claim.integrity", "claim-integrity");
        claim_integrity.applies_to = vec![];
        let registry = ProofRegistry {
            schema_version: 1,
            product_name: "p".to_owned(),
            proofs: vec![claim_integrity],
        };
        let request = RouteRequest {
            scope: Some("workspace".to_owned()),
            ..Default::default()
        };
        let routed = route_proofs(&registry, &request);
        assert_eq!(routed.len(), 1);
        assert_eq!(routed[0].id, "claim.integrity");
    }

    #[test]
    fn capability_filter_excludes_non_matching_proof() {
        let registry = ProofRegistry {
            schema_version: 1,
            product_name: "p".to_owned(),
            proofs: vec![definition("rust.proof", "command")],
        };
        let request = RouteRequest {
            files: vec!["crates/x/src/lib.rs".to_owned()],
            capability: Some("android-device".to_owned()),
            ..Default::default()
        };
        let routed = route_proofs(&registry, &request);
        assert!(
            routed.is_empty(),
            "non-matching capability must exclude the proof"
        );
    }

    // --- [G11] manual-required / unavailable + capability model --------

    #[test]
    fn no_command_manual_artifact_proof_is_manual_required() -> Result<()> {
        let definition = {
            let mut d = definition("manual.proof", "manual-artifact");
            d.collector = "manual-artifact".to_owned();
            d.capabilities = vec!["manual-required".to_owned()];
            d
        };
        let args = RunProofArgs {
            proof_id: "manual.proof".to_owned(),
            root: std::env::temp_dir(),
            run_id: "run-manual".to_owned(),
            command: vec![],
            ..Default::default()
        };
        let outcome = run_proof(&args, Some(&definition))?;
        assert!(!outcome.ok);
        assert_eq!(outcome.proof_run.status, ProofStatus::ManualRequired);
        assert_eq!(outcome.diagnostics.len(), 1);
        assert_eq!(outcome.diagnostics[0].rule_id, "PROOF-MANUAL");
        assert_eq!(outcome.diagnostics[0].severity, "warning");
        Ok(())
    }

    #[test]
    fn no_command_non_manual_proof_is_unavailable() -> Result<()> {
        let definition = definition("command.proof", "command");
        let args = RunProofArgs {
            proof_id: "command.proof".to_owned(),
            root: std::env::temp_dir(),
            run_id: "run-unavail".to_owned(),
            command: vec![],
            ..Default::default()
        };
        let outcome = run_proof(&args, Some(&definition))?;
        assert!(!outcome.ok);
        assert_eq!(outcome.proof_run.status, ProofStatus::Unavailable);
        assert_eq!(outcome.diagnostics[0].rule_id, "PROOF-MANUAL");
        assert_eq!(outcome.diagnostics[0].severity, "error");
        Ok(())
    }

    #[test]
    fn real_command_proof_passes_with_no_manual_diagnostic() -> Result<()> {
        let definition = definition("real.proof", "command");
        let program = if cfg!(windows) { "cmd" } else { "true" };
        let args = if cfg!(windows) {
            vec!["cmd".to_owned(), "/C".to_owned(), "exit 0".to_owned()]
        } else {
            vec!["true".to_owned()]
        };
        let _ = program;
        let run_args = RunProofArgs {
            proof_id: "real.proof".to_owned(),
            root: std::env::temp_dir(),
            run_id: "run-real".to_owned(),
            command: args,
            ..Default::default()
        };
        let outcome = run_proof(&run_args, Some(&definition))?;
        assert!(outcome.ok);
        assert_eq!(outcome.proof_run.status, ProofStatus::Passed);
        assert!(outcome.diagnostics.is_empty());
        Ok(())
    }

    #[test]
    fn capability_resolution_falls_back_through_args_definition_local() {
        let definition = definition("cap.proof", "command");
        assert_eq!(resolve_capability(Some("ci"), Some(&definition)), "ci");
        assert_eq!(resolve_capability(None, Some(&definition)), "local");
        assert_eq!(resolve_capability(None, None), "local");
    }

    // --- [G6] retention pruning ------------------------------------------

    fn day_of_iso(value: &str) -> Option<f64> {
        // Trivial fixture clock: encode "day N" as "2026-07-DDT..." so tests
        // can construct ages deterministically without a date-parsing dep.
        value.get(8..10).and_then(|d| d.parse::<f64>().ok())
    }

    #[test]
    fn more_than_twenty_runs_for_one_proof_id_prunes_to_twenty_keeping_newest() {
        let runs: Vec<ManifestRow> = (1..=25)
            .map(|day| ManifestRow {
                run_id: format!("run-{day:02}"),
                proof_id: "P".to_owned(),
                status: ProofStatus::Passed,
                started_at: format!("2026-07-{day:02}T00:00:00Z"),
                pinned: false,
            })
            .collect();
        let removed = prune_runs(&runs, DEFAULT_PROOF_RETENTION, 25.0, day_of_iso);
        // Keep newest 20 (days 6..=25); the oldest 5 without age-pruning
        // eligibility stay only if within prune_after_days=14 -> here all
        // are within age window except day<=11 (25-14=11), so beyond the
        // per-proof cap AND beyond the age window get removed.
        assert!(removed.contains(&"run-01".to_owned()));
        assert!(!removed.contains(&"run-25".to_owned()));
    }

    #[test]
    fn run_older_than_prune_after_days_is_removed_unless_pinned() {
        // Pad proof "P" past its per-proof-cap (20) with newer runs so the
        // per-proof-cap keep-newest rule does not itself rescue the old
        // run being tested here — isolating the age-based prune path.
        let mut runs: Vec<ManifestRow> = (2..=25)
            .map(|day| ManifestRow {
                run_id: format!("pad-{day:02}"),
                proof_id: "P".to_owned(),
                status: ProofStatus::Passed,
                started_at: format!("2026-07-{day:02}T00:00:00Z"),
                pinned: false,
            })
            .collect();
        runs.push(ManifestRow {
            run_id: "old-unpinned".to_owned(),
            proof_id: "P".to_owned(),
            status: ProofStatus::Passed,
            started_at: "2026-07-01T00:00:00Z".to_owned(),
            pinned: false,
        });
        runs.push(ManifestRow {
            run_id: "old-pinned".to_owned(),
            proof_id: "P".to_owned(),
            status: ProofStatus::Passed,
            started_at: "2026-07-01T00:00:00Z".to_owned(),
            pinned: true,
        });
        // "now" = day 30, prune_after_days = 14 -> age 29 > 14 for unpinned;
        // pin_pr_ready_days = 30 -> age 29 is still within grace for pinned.
        let removed = prune_runs(&runs, DEFAULT_PROOF_RETENTION, 30.0, day_of_iso);
        assert!(removed.contains(&"old-unpinned".to_owned()));
        // Pinned run is within pin_pr_ready_days=30 grace -> not removed.
        assert!(!removed.contains(&"old-pinned".to_owned()));
    }

    #[test]
    fn pinned_run_past_pin_pr_ready_days_becomes_prunable() {
        // Pad proof "P" past its per-proof-cap (20) with newer runs so the
        // per-proof-cap keep-newest rule does not rescue the stale pinned
        // run being tested here — isolating the pin-age prune path.
        let mut runs: Vec<ManifestRow> = (3..=27)
            .map(|day| ManifestRow {
                run_id: format!("pad-{day:02}"),
                proof_id: "P".to_owned(),
                status: ProofStatus::Passed,
                started_at: format!("2026-07-{day:02}T00:00:00Z"),
                pinned: false,
            })
            .collect();
        runs.push(ManifestRow {
            run_id: "stale-pinned".to_owned(),
            proof_id: "P".to_owned(),
            status: ProofStatus::Passed,
            started_at: "2026-07-01T00:00:00Z".to_owned(),
            pinned: true,
        });
        // "now" = day 32 -> age 31 > pin_pr_ready_days=30.
        let removed = prune_runs(&runs, DEFAULT_PROOF_RETENTION, 32.0, day_of_iso);
        assert!(
            removed.contains(&"stale-pinned".to_owned()),
            "the resolved pin_pr_ready_days behavior must make a stale pin prunable, not a silent no-op"
        );
    }
}
