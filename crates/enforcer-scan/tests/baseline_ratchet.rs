//! d02 proof: `cargo test -p enforcer-scan` (`baseline-ratchet`) over
//! `tests/fixtures/baseline_ratchet/**`.
//!
//! Covers the five acceptance scenarios named in
//! `docs/plans/enforcer-selfhost-plan/workpacks/d02-baseline-grandfather-
//! ratchet.md`: (a) clean baseline write, (b) unchanged run passes with
//! warnings, (c) one added finding fails, (d) one grown count fails, (e)
//! one removed finding shrinks the allowance. Also proves the persisted
//! record's `Sha256` integrity gate (tamper detection) and `RuleId` parity
//! against the real `enforcer-rules` registry (d01's oracle-backed
//! catalog), per the workpack's requirement checklist.
//!
//! No `unwrap`/`expect`/`panic` (workspace lints): every test returns
//! `Result` and propagates via `?`, using `assert!`/`assert_eq!` (allowed
//! panics, since a failed assertion IS the test failing) for outcomes.

use std::path::{Path, PathBuf};

use enforcer_domain::findings::{FindingLine, ReportOutcome, Violation};
use enforcer_domain::telemetry_types::SourceLine;
use enforcer_scan::boundary::baseline::{BaselineEntryDto, BaselineRecordDto};
use enforcer_scan::rules::baseline_ratchet::{
    load_baseline, write_baseline, Baseline, BaselineLocation, BaselineRatchetValidator,
    BASELINE_RECORD_VERSION,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/baseline_ratchet")
}

fn load_violations(name: &str) -> Result<Vec<Violation>, Box<dyn std::error::Error>> {
    let path = fixtures_dir().join(name);
    let raw = std::fs::read_to_string(&path)?;
    let findings: Vec<enforcer_domain::findings::Finding> = serde_json::from_str(&raw)?;
    let violations = findings
        .into_iter()
        .map(Violation::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(violations)
}

fn baseline_from(violations: &[Violation]) -> Baseline {
    Baseline::from_known(violations.iter().map(BaselineLocation::for_violation))
}

fn temp_baseline_path(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let unique = format!(
        "enforcer-scan-baseline-ratchet-{}-{nanos}-{name}.json",
        std::process::id(),
    );
    Ok(std::env::temp_dir().join(unique))
}

/// (a) `enforcer check --baseline write` records current findings to a
/// stable baseline file as a versioned `serde` record with a `Sha256`
/// integrity hash, and loading it back recovers exactly those entries.
#[test]
fn clean_baseline_write_round_trips_via_persisted_record() -> TestResult {
    let violations = load_violations("clean_write.json")?;
    let baseline = baseline_from(&violations);
    let path = temp_baseline_path("clean-write")?;

    write_baseline(&path, &baseline)?;
    let loaded = load_baseline(&path)?;

    assert_eq!(loaded, baseline, "round-tripped baseline must be identical");
    assert_eq!(loaded.entry_count().get(), 2);

    let raw = std::fs::read_to_string(&path)?;
    let record: BaselineRecordDto = serde_json::from_str(&raw)?;
    assert_eq!(record.version, BASELINE_RECORD_VERSION);
    let round_trip_entry: &BaselineEntryDto = record
        .entries
        .first()
        .ok_or("persisted baseline DTO must contain its first entry")?;
    assert_eq!(round_trip_entry.file.as_str(), "crates/legacy/src/lib.rs");
    let wire = serde_json::to_string(&record)?;
    let restored: BaselineRecordDto = serde_json::from_str(&wire)?;
    assert_eq!(restored, record);
    record.verify()?;

    std::fs::remove_file(&path)?;
    Ok(())
}

/// A hand-edited/corrupted baseline file must fail to load rather than be
/// silently trusted — the integrity hash is a real gate, not decoration.
#[test]
fn tampered_baseline_file_fails_to_load() -> TestResult {
    let violations = load_violations("clean_write.json")?;
    let baseline = baseline_from(&violations);
    let path = temp_baseline_path("tampered")?;

    write_baseline(&path, &baseline)?;
    let raw = std::fs::read_to_string(&path)?;
    let tampered = raw.replace("\"line\": 10", "\"line\": 999");
    std::fs::write(&path, tampered)?;

    let outcome = load_baseline(&path);
    assert!(
        outcome.is_err(),
        "a tampered baseline file must fail integrity verification"
    );

    std::fs::remove_file(&path)?;
    Ok(())
}

/// (b) An unchanged run (current violations == baselined violations)
/// passes, and every violation is demoted to a warning (grandfathered),
/// not silently dropped.
#[test]
fn unchanged_run_passes_with_warnings() -> TestResult {
    let baselined = load_violations("clean_write.json")?;
    let baseline = baseline_from(&baselined);
    let current = load_violations("unchanged_pass.json")?;

    let outcome = BaselineRatchetValidator::gate(&baseline, &current);

    assert_eq!(outcome.passes(), ReportOutcome::Clean);
    assert!(outcome.errors.is_empty());
    assert_eq!(
        outcome.warnings.len(),
        current.len(),
        "every baselined violation must surface as a warning, not vanish"
    );
    assert_eq!(outcome.ratcheted_baseline, baseline);
    Ok(())
}

/// (c) One added finding (not present in the baseline) fails closed, while
/// the previously-baselined findings still just warn.
#[test]
fn one_added_finding_fails() -> TestResult {
    let baselined = load_violations("clean_write.json")?;
    let baseline = baseline_from(&baselined);
    let current = load_violations("added_finding_fail.json")?;

    let outcome = BaselineRatchetValidator::gate(&baseline, &current);

    assert_eq!(outcome.passes(), ReportOutcome::Violations);
    assert_eq!(outcome.errors.len(), 1);
    assert_eq!(
        outcome.errors[0].finding().file.as_str(),
        "crates/legacy/src/fresh.rs"
    );
    assert_eq!(
        outcome.warnings.len(),
        2,
        "the two known findings still just warn"
    );
    assert_eq!(
        outcome.ratcheted_baseline.entry_count().get(),
        3,
        "the ratcheted baseline absorbs the new finding going forward"
    );
    Ok(())
}

/// (d) A grown count — a new occurrence line inside an already-baselined
/// file — fails closed exactly like any other new [`BaselineLocation`]; growth
/// cannot hide behind "the file was already flagged".
#[test]
fn one_grown_count_fails() -> TestResult {
    let baselined = load_violations("clean_write.json")?;
    let baseline = baseline_from(&baselined);
    let current = load_violations("grown_count_fail.json")?;

    let outcome = BaselineRatchetValidator::gate(&baseline, &current);

    assert_eq!(outcome.passes(), ReportOutcome::Violations);
    assert_eq!(outcome.errors.len(), 1);
    assert_eq!(
        outcome.errors[0].finding().line,
        FindingLine::known(SourceLine::try_new(
            std::num::NonZeroU32::new(25)
                .ok_or_else(|| std::io::Error::other("fixture line must be positive"))?,
        ))
    );
    assert_eq!(outcome.warnings.len(), 2);
    Ok(())
}

/// (e) One removed finding shrinks the allowance: the ratcheted baseline
/// drops the fixed occurrence and never carries it forward, proving the
/// ratchet is one-directional (it can shrink, never silently re-expand to
/// cover a since-fixed key on some later run).
#[test]
fn one_removed_finding_shrinks_the_allowance() -> TestResult {
    let baselined = load_violations("clean_write.json")?;
    let baseline = baseline_from(&baselined);
    assert_eq!(baseline.entry_count().get(), 2);
    let current = load_violations("removed_finding_shrink.json")?;

    let outcome = BaselineRatchetValidator::gate(&baseline, &current);

    assert_eq!(outcome.passes(), ReportOutcome::Clean);
    assert_eq!(
        outcome.ratcheted_baseline.entry_count().get(),
        1,
        "the fixed finding must be dropped, shrinking the baseline"
    );

    // Persist the shrunk baseline and load it back: the ratchet-down is
    // durable, not just an in-memory artifact of this one run.
    let path = temp_baseline_path("shrunk")?;
    write_baseline(&path, &outcome.ratcheted_baseline)?;
    let reloaded = load_baseline(&path)?;
    assert_eq!(reloaded.entry_count().get(), 1);

    // A later run against the ORIGINAL (larger) violation set must now
    // re-fail on the dropped key: the ratchet never silently re-grants an
    // allowance it already shrank away.
    let later_outcome = BaselineRatchetValidator::gate(&reloaded, &baselined);
    assert_eq!(later_outcome.passes(), ReportOutcome::Violations);

    std::fs::remove_file(&path)?;
    Ok(())
}

/// Baseline entries reference real registry `RuleId`s — parity enforced
/// via the d01 mechanization oracle's registry loader. Every fixture in
/// this suite uses `T1-NOREEXPORT.1`, which must resolve in the real
/// `enforcer-rules` catalog, not a fixture-only placeholder id.
#[test]
fn baseline_entries_reference_real_registry_rule_ids() -> TestResult {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("enforcer-scan is expected two levels under the repo root")?;
    let catalog_path = repo_root.join("crates/enforcer-rules/rules/no-reexports.json");
    let registry = enforcer_rules::loader::load_registry_from_files(&[catalog_path.as_path()])?;

    let violations = load_violations("clean_write.json")?;
    for violation in &violations {
        let rule_id = &violation.finding().rule_id;
        assert!(
            registry.get(rule_id).is_some(),
            "baseline fixture references `{rule_id}`, which must exist in the real registry"
        );
    }
    Ok(())
}
