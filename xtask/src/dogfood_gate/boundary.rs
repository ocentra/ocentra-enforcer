//! z01 boundary: the effectful composition of the terminal gate.
//!
//! Owns everything raw or effectful the gate touches: the on-disk
//! locations ([`GatePaths`]), the committed T2-ceiling record
//! ([`CeilingDto`]), the e01 literal-scan invocation, the b02 PLAN-*
//! read-only sweep, the persisted `proof/dogfood-manifest.json`
//! ([`ManifestDto`]) and the tamper-evident `enforcer-proof` journal
//! append. The PASS/FAIL policy itself lives in the domain half
//! ([`crate::dogfood_gate::judge`]); this module only gathers inputs,
//! invokes it, and records the outcome.

use std::path::{Path, PathBuf};

use enforcer_core::redaction::Redactor;
use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::hashes::Sha256;
use enforcer_domain::proof_types::{JournalEventType, ProofId, ProofRunId};
use enforcer_domain::scan_types::{
    LiteralScanCount, LiteralScanPaths, LiteralScanRoot, ScanTargetCount,
};
use enforcer_domain::telemetry_types::{FindingCount, RecordSchemaVersion};
use enforcer_domain::xtask_types::{
    DogfoodFamily, DogfoodGateVerdict, LiteralFloorCheck, ToolchainMode,
};
use enforcer_proof::journal::{JournalRecordEnvelope, ProofJournal, JOURNAL_SCHEMA_VERSION};
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::dogfood::{self, RustRuleScanResult};
use crate::dogfood_gate::{judge, ruleset_fingerprint, GateError};

/// Manifest schema version. Bump only on a wire-incompatible change.
const MANIFEST_SCHEMA_VERSION: RecordSchemaVersion = RecordSchemaVersion::V1;

/// Ceiling record schema version. Bump only on a wire-incompatible change.
const CEILING_SCHEMA_VERSION: RecordSchemaVersion = RecordSchemaVersion::V1;

/// Where the gate reads/writes everything, all derived from one root so
/// fixture repos and the live workspace share the identical layout.
#[derive(Debug, Clone)]
#[doc = "The gate's on-disk locations; see the module docs."]
pub struct GatePaths {
    root: PathBuf,
    baseline_store: PathBuf,
    ceiling_store: PathBuf,
    manifest_file: PathBuf,
    journal_file: PathBuf,
    rules_dir: PathBuf,
}

impl GatePaths {
    /// Derive every gate location from `root` (the workspace root for the
    /// live run; a temp dir for fixtures).
    pub fn under(root: &Path) -> Self {
        Self::under_with_proof_output(root, &root.join("proof"))
    }

    /// Derive gate locations from `root`, writing the mutable proof outputs
    /// below an explicit directory. The baseline, ceiling, rules, and scan
    /// root deliberately remain rooted at the real workspace: changing the
    /// proof sink must never change what the gate validates.
    pub fn under_with_proof_output(root: &Path, proof_output: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            baseline_store: root.join("xtask/dogfood-baseline.json"),
            ceiling_store: root.join("xtask/dogfood-t2-ceiling.json"),
            manifest_file: proof_output.join("dogfood-manifest.json"),
            journal_file: proof_output.join("dogfood-journal.ndjson"),
            rules_dir: root.join("crates/enforcer-rules/rules"),
        }
    }

    /// The committed T2-ceiling record location.
    pub fn ceiling_store(&self) -> &Path {
        &self.ceiling_store
    }

    /// The persisted manifest location.
    pub fn manifest_file(&self) -> &Path {
        &self.manifest_file
    }

    /// The hash-chained proof-journal location.
    pub fn journal_file(&self) -> &Path {
        &self.journal_file
    }
}

/// The committed T2 ceiling for the e01 literal-scan floor (z01's own
/// "below its committed T2 ceiling" clause). The floor is a T2 advisory
/// family with hundreds of pre-existing, grandfathered hard findings
/// across `crates/**`; a gate that hard-failed on their mere existence
/// would start red and get bypassed forever -- the exact failure mode the
/// a10 baseline exists to prevent for the T1 families. So the same
/// posture applies: the CURRENT count is committed as the ceiling, the
/// gate fails only when the count GROWS past it, and the ceiling is
/// refreshed only by the explicit [`write_ceiling_snapshot`] maintenance
/// operation, never as a side effect of a normal gate run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[doc = "The committed T2-ceiling wire record; see the note above."]
struct CeilingDto {
    schema_version: RecordSchemaVersion,
    literal_scan_hard_findings: u64,
}

impl From<LiteralScanCount> for CeilingDto {
    /// Build the wire record for a freshly observed hard-finding count.
    fn from(hard_findings: LiteralScanCount) -> Self {
        Self {
            schema_version: CEILING_SCHEMA_VERSION,
            literal_scan_hard_findings: finding_count_of(hard_findings.get()).get(),
        }
    }
}

impl From<&CeilingDto> for LiteralScanCount {
    /// Recover the committed ceiling count from the wire record.
    fn from(record: &CeilingDto) -> Self {
        LiteralScanCount::from_count(
            usize::try_from(record.literal_scan_hard_findings).unwrap_or_default(),
        )
    }
}

/// Load the committed ceiling. A missing or malformed record fails CLOSED
/// as a ZERO ceiling (any hard finding blocks) -- never silently treated
/// as "unlimited".
fn load_ceiling(ceiling_store: &Path) -> LiteralScanCount {
    let Ok(raw) = std::fs::read(ceiling_store) else {
        return LiteralScanCount::from_count(0);
    };
    match serde_json::from_slice::<CeilingDto>(&raw) {
        Ok(record) => LiteralScanCount::from(&record),
        Err(_) => LiteralScanCount::from_count(0),
    }
}

/// One literal-scan floor observation.
#[derive(Debug, Clone, Copy)]
#[doc = "One e01 floor observation; see the module docs."]
struct LiteralFloor {
    hard_findings: LiteralScanCount,
    risks: LiteralScanCount,
}

/// Run the e01 literal-scan floor over the workspace's `crates/**` (the
/// same shipped-source scope the a10 rust-rule scan gates), through the
/// exact `enforcer_literal_scan::run_scan` entry point `enforcer advise
/// literals` uses.
fn run_literal_scan_floor(root: &Path) -> Result<LiteralFloor, GateError> {
    let opts = enforcer_literal_scan::CliOptions {
        root: LiteralScanRoot::from(root.to_path_buf()),
        files: LiteralScanPaths::from(vec![PathBuf::from("crates")]),
        ..enforcer_literal_scan::CliOptions::default()
    };
    let report = enforcer_literal_scan::run_scan(&opts).map_err(GateError::from_display)?;
    Ok(LiteralFloor {
        hard_findings: LiteralScanCount::from(report.hard_findings.len()),
        risks: LiteralScanCount::from(report.literal_risks.len()),
    })
}

/// Widen a length into the wire count width.
fn finding_count_of(length: usize) -> FindingCount {
    // CAST-JUSTIFICATION: usize -> u64 is lossless on every supported
    // platform (usize is at most 64 bits wide).
    FindingCount::new(length as u64)
}

/// Compare a floor observation against the committed ceiling.
pub fn check_floor(
    hard_findings: LiteralScanCount,
    ceiling: LiteralScanCount,
) -> LiteralFloorCheck {
    if hard_findings <= ceiling {
        LiteralFloorCheck::WithinCeiling
    } else {
        LiteralFloorCheck::ExceedsCeiling
    }
}

/// The explicit `--ceiling-write` maintenance operation: observe the
/// current floor and commit its hard-finding count as the new ceiling.
///
/// # Errors
/// Returns [`GateError`] if the scan or the write fails.
pub fn write_ceiling_snapshot(paths: &GatePaths) -> Result<LiteralScanCount, GateError> {
    let floor = run_literal_scan_floor(&paths.root)?;
    let record = CeilingDto::from(floor.hard_findings);
    if let Some(parent) = paths.ceiling_store.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(GateError::from_display)?;
        }
    }
    let payload = serde_json::to_vec_pretty(&record).map_err(GateError::from_display)?;
    std::fs::write(&paths.ceiling_store, payload).map_err(GateError::from_display)?;
    Ok(floor.hard_findings)
}

/// Run the b02 PLAN-* structure validators, read-only, over every
/// workpack under `docs/plans/enforcer-selfhost-plan/workpacks/`. Reports
/// the finding count (visible in the manifest); does not gate the verdict
/// -- see the domain module's docs for why.
fn plan_structure_report(root: &Path) -> FindingCount {
    let workpacks_dir = root.join("docs/plans/enforcer-selfhost-plan/workpacks");
    let Ok(entries) = std::fs::read_dir(&workpacks_dir) else {
        return FindingCount::new(0);
    };
    let ids: Result<
        (
            enforcer_domain::ids::RuleId,
            enforcer_domain::ids::RuleId,
            enforcer_domain::ids::RuleId,
        ),
        enforcer_domain::boundary::decode_error::DecodeError,
    > = Ok((
        match "PLAN-CAPSULE.1".parse() {
            Ok(id) => id,
            Err(_) => return FindingCount::new(0),
        },
        match "PLAN-SKELETON.1".parse() {
            Ok(id) => id,
            Err(_) => return FindingCount::new(0),
        },
        match "PLAN-FRONTMATTER.1".parse() {
            Ok(id) => id,
            Err(_) => return FindingCount::new(0),
        },
    ));
    let Ok((capsule_id, skeleton_id, frontmatter_id)) = ids else {
        return FindingCount::new(0);
    };
    let capsule = enforcer_plan::validator::PlanCapsuleValidator::new(capsule_id);
    let skeleton = enforcer_plan::validator::PlanSkeletonValidator::new(skeleton_id);
    let frontmatter = enforcer_plan::validator::PlanFrontmatterValidator::new(frontmatter_id);

    let mut total = FindingCount::new(0);
    for entry in entries.filter_map(Result::ok) {
        let doc = entry.path();
        if doc.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&doc) else {
            continue;
        };
        let rel = doc
            .strip_prefix(root)
            .unwrap_or(&doc)
            .to_string_lossy()
            .replace('\\', "/");
        let Ok(rel_path) = rel.parse::<enforcer_domain::paths::RelPath>() else {
            continue;
        };
        let input_for = |scope| ValidationInput {
            file: &rel_path,
            source: ValidationSource::from_text(&source),
            scope,
        };
        let scope = enforcer_domain::findings::ScanScope::Files;
        total = FindingCount::new(
            total
                .get()
                .saturating_add(finding_count_of(capsule.validate(input_for(scope)).len()).get())
                .saturating_add(finding_count_of(skeleton.validate(input_for(scope)).len()).get())
                .saturating_add(
                    finding_count_of(frontmatter.validate(input_for(scope)).len()).get(),
                ),
        );
    }
    total
}

/// One per-family count row in the manifest.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[doc = "One per-family manifest count row."]
struct FamilyCountDto {
    family: DogfoodFamily,
    count: FindingCount,
}

impl FamilyCountDto {
    fn row(family: DogfoodFamily, count: FindingCount) -> Self {
        Self { family, count }
    }
}

/// The durable proof artifact: `proof/dogfood-manifest.json`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[doc = "The persisted dogfood-manifest wire record; see the module docs."]
struct ManifestDto {
    schema_version: RecordSchemaVersion,
    timestamp: String,
    ruleset_fingerprint: Sha256,
    ran_count: ScanTargetCount,
    toolchain_included: bool,
    family_counts: Vec<FamilyCountDto>,
    verdict: DogfoodGateVerdict,
}

/// Everything one gate run produced, for the console renderer and the
/// CLI integration test.
#[derive(Debug)]
#[doc = "One composed gate run; see the module docs."]
pub struct GateRun {
    verdict: DogfoodGateVerdict,
    floor_check: LiteralFloorCheck,
    scan: RustRuleScanResult,
}

impl GateRun {
    /// The terminal verdict this run recorded.
    pub fn verdict(&self) -> DogfoodGateVerdict {
        self.verdict
    }

    /// The e01 floor's standing against its committed ceiling.
    pub fn floor_check(&self) -> LiteralFloorCheck {
        self.floor_check
    }

    /// The a10 baseline-gated scan result this run composed.
    pub fn scan(&self) -> &RustRuleScanResult {
        &self.scan
    }
}

/// Current wall-clock time in the manifest's ISO-8601 UTC form.
fn timestamp_now() -> Result<String, GateError> {
    let millis = enforcer_core::platform::epoch_millis().map_err(GateError::from_display)?;
    Ok(enforcer_core::platform::iso8601_utc(millis))
}

/// Persist the manifest, creating `proof/` as needed.
fn persist_manifest(manifest_file: &Path, manifest: &ManifestDto) -> Result<(), GateError> {
    if let Some(parent) = manifest_file.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(GateError::from_display)?;
        }
    }
    let payload = serde_json::to_vec_pretty(manifest).map_err(GateError::from_display)?;
    std::fs::write(manifest_file, payload).map_err(GateError::from_display)?;
    Ok(())
}

/// Append the run's record to the `enforcer-proof` hash-chained journal
/// (verify-on-open per that crate's contract) and re-verify the whole
/// chain afterwards, failing closed on any break.
fn append_journal_entry(journal_file: &Path, manifest: &ManifestDto) -> Result<(), GateError> {
    if let Some(parent) = journal_file.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(GateError::from_display)?;
        }
    }
    let mut journal = ProofJournal::open(journal_file).map_err(GateError::from_display)?;
    let redactor = Redactor::with_defaults().map_err(GateError::from_display)?;
    let payload = serde_json::to_value(manifest).map_err(GateError::from_display)?;
    // CLONE-JUSTIFICATION: the journal record owns its timestamp; the
    // manifest stays borrowed for the persist step alongside this append.
    let record_timestamp = manifest.timestamp.clone();
    let proof_id =
        ProofId::try_from(String::from("PROOF-DOGFOOD-GATE")).map_err(GateError::from_display)?;
    let event_type = JournalEventType::try_from(format!("dogfood-gate-{}", manifest.verdict))
        .map_err(GateError::from_display)?;
    let run_id = ProofRunId::try_from(format!("dogfood-gate-{}", manifest.timestamp))
        .map_err(GateError::from_display)?;
    let record = JournalRecordEnvelope {
        schema_version: JOURNAL_SCHEMA_VERSION,
        event_type,
        run_id,
        proof_id,
        timestamp: record_timestamp,
        payload,
    };
    journal
        .append(&redactor, record)
        .map_err(GateError::from_display)?;
    journal
        .verify_on_replay()
        .map_err(GateError::from_display)?;
    Ok(())
}

/// Run the full terminal gate: compose a10 + e01 + b02, judge, persist
/// the manifest, and append the tamper-evident journal record.
///
/// # Errors
/// Returns [`GateError`] on any composition/io failure (including a
/// hollow a10 self-scan -- see [`dogfood::run_rust_rule_scan`]).
pub fn run_gate(paths: &GatePaths, mode: ToolchainMode) -> Result<GateRun, GateError> {
    let outcome = dogfood::run_dogfood(&paths.root, &paths.baseline_store, mode)?;
    let floor = run_literal_scan_floor(&paths.root)?;
    let ceiling = load_ceiling(&paths.ceiling_store);
    let floor_check = check_floor(floor.hard_findings, ceiling);
    let plan_count = plan_structure_report(&paths.root);
    let fingerprint = ruleset_fingerprint(&paths.rules_dir)?;
    let verdict = judge(
        &outcome.rust_rule_scan.gate,
        outcome.toolchain.as_ref(),
        floor_check,
    );

    let manifest = ManifestDto {
        schema_version: MANIFEST_SCHEMA_VERSION,
        timestamp: timestamp_now()?,
        ruleset_fingerprint: fingerprint,
        ran_count: outcome.rust_rule_scan.coverage.ran_count(),
        toolchain_included: outcome.toolchain.is_some(),
        family_counts: vec![
            FamilyCountDto::row(
                DogfoodFamily::RustRulesNewViolations,
                finding_count_of(outcome.rust_rule_scan.gate.errors.len()),
            ),
            FamilyCountDto::row(
                DogfoodFamily::RustRulesBaselinedDebt,
                finding_count_of(outcome.rust_rule_scan.gate.warnings.len()),
            ),
            FamilyCountDto::row(
                DogfoodFamily::LiteralScanHardFindings,
                finding_count_of(floor.hard_findings.get()),
            ),
            FamilyCountDto::row(
                DogfoodFamily::LiteralScanHardFindingsCeiling,
                finding_count_of(ceiling.get()),
            ),
            FamilyCountDto::row(
                DogfoodFamily::LiteralScanRisks,
                finding_count_of(floor.risks.get()),
            ),
            FamilyCountDto::row(DogfoodFamily::PlanStructure, plan_count),
        ],
        // ALLOC-JUSTIFICATION: the manifest owns its verdict token; the
        // Display rendering is the locked wire form.
        verdict,
    };

    persist_manifest(&paths.manifest_file, &manifest)?;
    append_journal_entry(&paths.journal_file, &manifest)?;

    Ok(GateRun {
        verdict,
        floor_check,
        scan: outcome.rust_rule_scan,
    })
}

#[cfg(test)]
mod tests {
    use super::{check_floor, run_gate, CeilingDto, FamilyCountDto, GatePaths, ManifestDto};
    use crate::boundary::testkit::{seed, seed_config, seed_rules_catalog, violating_body};
    use enforcer_domain::xtask_types::{
        DogfoodFamily, DogfoodGateVerdict, LiteralFloorCheck, ToolchainMode,
    };
    use enforcer_domain::{
        scan_types::{LiteralScanCount, ScanTargetCount},
        telemetry_types::{FindingCount, RecordSchemaVersion},
    };
    use std::path::Path;

    fn seed_fixture_repo(root: &Path) -> std::io::Result<()> {
        seed_config(root)?;
        seed_rules_catalog(root)?;
        seed(
            root,
            "crates/sample/src/lib.rs",
            &crate::boundary::testkit::clean_body(),
        )
    }

    #[test]
    fn clean_fixture_repo_passes_and_emits_manifest_and_journal() -> Result<(), std::io::Error> {
        let temp = tempfile::tempdir()?;
        seed_fixture_repo(temp.path())?;
        let paths = GatePaths::under(temp.path());
        let run = run_gate(&paths, ToolchainMode::Skip).map_err(std::io::Error::other)?;
        assert_eq!(run.verdict(), DogfoodGateVerdict::Pass);
        assert!(
            !run.scan().coverage.ran_count().is_zero(),
            "coverage must be nonzero"
        );
        assert!(paths.manifest_file().is_file());
        assert!(paths.journal_file().is_file());

        // The persisted manifest decodes back to the identical record
        // shape (wire-form check the CLI test also leans on).
        let raw = std::fs::read(paths.manifest_file())?;
        let decoded: ManifestDto = serde_json::from_slice(&raw)?;
        assert_eq!(decoded.verdict, DogfoodGateVerdict::Pass);
        Ok(())
    }

    #[test]
    fn seeded_self_violation_fails_the_gate() -> Result<(), std::io::Error> {
        let temp = tempfile::tempdir()?;
        seed_fixture_repo(temp.path())?;
        seed(temp.path(), "crates/sample/src/bad.rs", &violating_body())?;
        let paths = GatePaths::under(temp.path());
        let run = run_gate(&paths, ToolchainMode::Skip).map_err(std::io::Error::other)?;
        assert_eq!(
            run.verdict(),
            DogfoodGateVerdict::Fail,
            "a seeded, unbaselined self-violation must FAIL the gate"
        );
        Ok(())
    }

    #[test]
    fn hollow_repo_with_no_crates_is_an_error_not_a_pass() -> Result<(), std::io::Error> {
        let temp = tempfile::tempdir()?;
        // `**/rules/**` excludes the (required-on-disk) rules catalog from
        // the WALK, so zero crates/** files are dispatched -- the hollow,
        // invalid state -- while the fingerprint (which reads the catalog
        // directly, not through the walk) still resolves.
        seed(
            temp.path(),
            "ocentra-enforcer.config.json",
            r#"{"schemaVersion":2,"profileName":"default","ignoreFileGlobs":["**/rules/**"]}"#,
        )?;
        seed_rules_catalog(temp.path())?;
        let paths = GatePaths::under(temp.path());
        assert!(
            run_gate(&paths, ToolchainMode::Skip).is_err(),
            "a hollow scan (zero crates/** files) must be a hard error, never a silent PASS"
        );
        Ok(())
    }

    #[test]
    fn floor_check_grandfathers_up_to_the_ceiling_and_blocks_growth() {
        assert_eq!(
            check_floor(
                LiteralScanCount::from_count(0),
                LiteralScanCount::from_count(0),
            ),
            LiteralFloorCheck::WithinCeiling
        );
        assert_eq!(
            check_floor(
                LiteralScanCount::from_count(402),
                LiteralScanCount::from_count(402),
            ),
            LiteralFloorCheck::WithinCeiling
        );
        assert_eq!(
            check_floor(
                LiteralScanCount::from_count(403),
                LiteralScanCount::from_count(402),
            ),
            LiteralFloorCheck::ExceedsCeiling
        );
        // A missing/malformed ceiling record loads as zero, so any hard
        // finding blocks (fail-closed, never "unlimited").
        assert_eq!(
            check_floor(
                LiteralScanCount::from_count(1),
                LiteralScanCount::from_count(0),
            ),
            LiteralFloorCheck::ExceedsCeiling
        );
    }

    /// PROPERTY-TEST: over a generated grid of counts and verdict tokens,
    /// the two wire records round-trip byte-stably through serde -- the
    /// round-trip property the committed ceiling and persisted manifest
    /// rely on to be diff-reviewable.
    #[test]
    fn ceiling_and_manifest_wire_records_round_trip() -> Result<(), std::io::Error> {
        let counts = [0_u64, 1, 402, u64::MAX];
        for count in counts {
            let native_count = usize::try_from(count).map_err(std::io::Error::other)?;
            let literal_count = LiteralScanCount::from_count(native_count);
            let record = CeilingDto::from(literal_count);
            let wire = serde_json::to_vec(&record)?;
            let back: CeilingDto = serde_json::from_slice(&wire)?;
            assert_eq!(back, record, "ceiling round-trip diverged at {count}");
            assert_eq!(LiteralScanCount::from(&back), literal_count);

            let manifest = ManifestDto {
                schema_version: RecordSchemaVersion::V1,
                timestamp: String::from("2026-07-12T00:00:00.000Z"),
                ruleset_fingerprint: enforcer_core::hash_chain::link_digest(None, b"test ruleset"),
                ran_count: ScanTargetCount::from_count(native_count),
                toolchain_included: count == 1,
                family_counts: vec![FamilyCountDto::row(
                    DogfoodFamily::RustRulesNewViolations,
                    FindingCount::new(count),
                )],
                verdict: DogfoodGateVerdict::Pass,
            };
            let manifest_wire = serde_json::to_vec(&manifest)?;
            let manifest_back: ManifestDto = serde_json::from_slice(&manifest_wire)?;
            assert_eq!(
                manifest_back, manifest,
                "manifest round-trip diverged at {count}"
            );
        }
        Ok(())
    }
}
