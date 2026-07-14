//! Ledger behavior verification for the lesson-capture loop and seed importer.
//!
//! These tests exercise the public `enforcer_plan::lessons` API from the
//! consumer boundary. They deliberately keep the append/replay/import loop
//! outside the implementation module so observable behavior does not depend
//! on crate-private test access.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use enforcer_plan::error::PlanError;
use enforcer_plan::lessons::{
    add, emit_doctrine_block, import_seed_corpus, list, ArtifactRef, CapturedDate, EmitFs,
    LessonDomain, LessonId, LessonLedger, LessonRecord, LessonRoute,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn temp_ledger_path(name: &str) -> PathBuf {
    static NEXT_TEST_LEDGER: AtomicUsize = AtomicUsize::new(0);
    let unique = format!(
        "enforcer-plan-lessons-external-{}-{}-{name}.ndjson",
        std::process::id(),
        NEXT_TEST_LEDGER.fetch_add(1, Ordering::Relaxed)
    );
    std::env::temp_dir().join(unique)
}

fn sample_record(id: &str) -> TestResultRecord {
    Ok(LessonRecord {
        id: id.parse()?,
        date: "2026-07-04".parse()?,
        domain: LessonDomain::Harness,
        observed: "example observation".parse()?,
        lesson: "example lesson text".parse()?,
        routes: vec![LessonRoute::DoctrineBlock, LessonRoute::Skill],
        landed_at: Vec::new(),
        supersedes_seq: None,
    })
}

type TestResultRecord = Result<LessonRecord, Box<dyn std::error::Error>>;

struct DeniedReadFs;

impl EmitFs for DeniedReadFs {
    fn read(&self, _: &Path) -> Result<Option<String>, PlanError> {
        Err(PlanError::Io {
            path: "denied.md".to_owned(),
            reason: "permission denied".to_owned(),
        })
    }

    fn write(&mut self, _: &Path, _: &str) -> Result<(), PlanError> {
        Ok(())
    }
}

#[test]
fn emitter_fails_closed_when_an_existing_target_cannot_be_read() -> TestResult {
    let mut fs = DeniedReadFs;
    let error = emit_doctrine_block(
        &mut fs,
        &sample_record("L1")?,
        Path::new("denied.md"),
        false,
    )
    .expect_err("unreadable target must not be treated as absent");
    match error {
        PlanError::Io { path, reason } => {
            assert_eq!(path, "denied.md");
            assert_eq!(reason, "permission denied");
        }
        other => return Err(format!("expected I/O error, got {other}").into()),
    }
    Ok(())
}

#[test]
fn lesson_id_rejects_invalid_input() {
    for good in ["L1", "L26", "L11-FILL"] {
        assert_eq!(
            good.parse::<LessonId>().map(|value| value.to_string()),
            Ok(good.to_owned()),
        );
    }
    for bad in ["", "1", "l1", "M1", "Lalpha", "L1x", "L-", "L1-", "L1--FILL"] {
        assert_eq!(
            bad.parse::<LessonId>()
                .err()
                .map(|error| error.to_string()),
            Some("decode/validation failed at `lessonId`: expected `L<number>[-SUFFIX]` (e.g. `L1`, `L26`, `L11-FILL`)".to_owned()),
        );
    }
}

#[test]
fn artifact_ref_rejects_invalid_input() {
    assert_eq!(
        "".parse::<ArtifactRef>()
            .err()
            .map(|error| error.to_string()),
        Some("decode/validation failed at `artifactRef`: expected a non-empty landed-artifact reference (path#anchor or path)".to_owned()),
    );
    assert_eq!(
        "some/path.md#L1"
            .parse::<ArtifactRef>()
            .map(|value| value.to_string()),
        Ok("some/path.md#L1".to_owned()),
    );
}

#[test]
fn captured_date_rejects_invalid_shape_but_preserves_legacy_absence() {
    assert!("2026-07-13".parse::<CapturedDate>().is_ok());
    assert!("".parse::<CapturedDate>().is_ok());
    assert!("2026/07/13".parse::<CapturedDate>().is_err());
}

#[test]
fn ledger_round_trips_and_verifies_from_the_public_api() -> TestResult {
    let path = temp_ledger_path("round-trip");
    {
        let mut ledger = LessonLedger::open(&path)?;
        ledger.append(sample_record("L1")?)?;
        ledger.append(sample_record("L2")?)?;
    }
    let ledger = LessonLedger::open(&path)?;
    assert_eq!(ledger.verify_on_replay()?, 2);
    assert_eq!(ledger.list()?.len(), 2);
    std::fs::remove_file(&path)?;
    Ok(())
}

#[test]
fn rewriting_a_prior_row_is_detected_on_public_open() -> TestResult {
    let path = temp_ledger_path("tamper");
    {
        let mut ledger = LessonLedger::open(&path)?;
        ledger.append(sample_record("L1")?)?;
        ledger.append(sample_record("L2")?)?;
    }
    let content = std::fs::read_to_string(&path)?;
    let mut lines: Vec<String> = content.lines().map(str::to_owned).collect();
    let mut value: serde_json::Value = serde_json::from_str(&lines[0])?;
    value["record"]["lesson"] = serde_json::json!("REWRITTEN");
    lines[0] = value.to_string();
    std::fs::write(&path, lines.join("\n") + "\n")?;

    match LessonLedger::open(&path) {
        Err(PlanError::Io { path, reason }) => {
            assert_eq!(path, "lesson ledger");
            assert_eq!(reason.starts_with("lesson ledger tamper detected at line 0"), true);
        }
        Err(other) => return Err(format!("expected tamper rejection, received {other}").into()),
        Ok(_) => return Err("expected tamper rejection, received open ledger".into()),
    }
    std::fs::remove_file(&path)?;
    Ok(())
}

#[test]
fn supersede_appends_and_folds_latest_state() -> TestResult {
    let path = temp_ledger_path("supersede");
    let mut ledger = LessonLedger::open(&path)?;
    ledger.append(sample_record("L1")?)?;
    let artifact: ArtifactRef = "docs/AGENTS.md#L1".parse()?;
    ledger.supersede(&"L1".parse()?, vec![artifact.clone()])?;

    assert_eq!(ledger.list()?.len(), 2);
    assert_eq!(ledger.verify_on_replay()?, 2);
    let latest = ledger.latest()?;
    assert_eq!(latest.len(), 1);
    assert_eq!(latest[0].landed_at, vec![artifact]);
    std::fs::remove_file(&path)?;
    Ok(())
}

#[test]
fn append_rejects_a_duplicate_id_without_supersede() -> TestResult {
    let path = temp_ledger_path("dup-reject");
    let mut ledger = LessonLedger::open(&path)?;
    ledger.append(sample_record("L1")?)?;
    match ledger.append(sample_record("L1")?) {
        Err(PlanError::Io { reason, .. }) => assert_eq!(
            reason,
            "lesson `L1` already captured; use supersede to fill in landed_at",
        ),
        Err(other) => return Err(format!("expected duplicate rejection, received {other}").into()),
        Ok(()) => return Err("expected duplicate rejection, received append success".into()),
    }
    std::fs::remove_file(&path)?;
    Ok(())
}

#[test]
fn add_then_list_round_trips_through_the_cli_seam() -> TestResult {
    let path = temp_ledger_path("seam");
    add(&path, sample_record("L1")?)?;
    add(&path, sample_record("L2")?)?;
    assert_eq!(list(&path, None, false)?.len(), 2);
    assert_eq!(list(&path, Some(LessonRoute::Skill), false)?.len(), 2);
    assert_eq!(list(&path, None, true)?.len(), 2);
    std::fs::remove_file(&path)?;
    Ok(())
}

#[test]
fn pending_list_keeps_captured_but_unrouted_lessons_visible() -> TestResult {
    let path = temp_ledger_path("unrouted-pending");
    let mut record = sample_record("L3")?;
    record.routes.clear();

    add(&path, record)?;
    let pending = list(&path, None, true)?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id.as_str(), "L3");
    std::fs::remove_file(&path)?;
    Ok(())
}

fn seed_markdown_fixture() -> String {
    r#"
| id | date | observed | lesson | landed-at | ships-via |
|---|---|---|---|---|---|
| L1 | 2026-07-04 | `coordination_init` re-init threw raw `EEXIST` | init must be idempotent | arc-16 finding (this row) | fixed MCP tool behavior (arc-16) |
| L4 | 2026-07-04 | wave-1 workers went silent until done | worker mail lifecycle is started -> progress -> done/blocked | EXECUTION_MODEL section2d | c01 doctrine payload + b06 decision forest |
| L15 | 2026-07-04 | [code] arc-02 dogfood boundary allowlist gap | rule configs must ship boundary-module globs | this row | rules-as-data (arc-04/arc-06) |
"#
    .to_owned()
}

#[test]
fn seed_import_is_idempotent_and_preserves_chain_integrity() -> TestResult {
    let path = temp_ledger_path("import");
    let mut ledger = LessonLedger::open(&path)?;
    let markdown = vec![seed_markdown_fixture()];
    let first = import_seed_corpus(&mut ledger, &markdown, &[])?;
    assert_eq!((first.discovered, first.newly_appended), (3, 3));
    let second = import_seed_corpus(&mut ledger, &markdown, &[])?;
    assert_eq!((second.discovered, second.newly_appended), (3, 0));
    assert_eq!(ledger.verify_on_replay()?, 3);
    std::fs::remove_file(&path)?;
    Ok(())
}

#[test]
fn seed_import_maps_routes_and_domains_from_the_public_record_shape() -> TestResult {
    let path = temp_ledger_path("import-map");
    let mut ledger = LessonLedger::open(&path)?;
    import_seed_corpus(&mut ledger, &[seed_markdown_fixture()], &[])?;
    let records = ledger.latest()?;
    let l1 = records.iter().find(|record| record.id.as_str() == "L1").ok_or("L1")?;
    assert_eq!(l1.domain, LessonDomain::Harness);
    assert_eq!(l1.routes, vec![LessonRoute::PlanDoc]);
    let l4 = records.iter().find(|record| record.id.as_str() == "L4").ok_or("L4")?;
    assert_eq!(
        l4.routes,
        vec![LessonRoute::DoctrineBlock, LessonRoute::ForestNode],
    );
    let l15 = records.iter().find(|record| record.id.as_str() == "L15").ok_or("L15")?;
    assert_eq!(l15.domain, LessonDomain::Code);
    assert_eq!(l15.routes, vec![LessonRoute::RuleCandidate]);
    std::fs::remove_file(&path)?;
    Ok(())
}

#[test]
fn seed_import_accepts_memory_stream_rows_and_is_idempotent() -> TestResult {
    let path = temp_ledger_path("import-memory");
    let mut ledger = LessonLedger::open(&path)?;
    let stream = r#"{"id":"L900","date":"2026-07-04","domain":"code","observed":"[code] example","lesson":"example fix","shipsVia":"rules-as-data","landedAt":"docs/x.md#L900"}
{"id":"status-only","note":"not a lesson row"}
"#
    .to_owned();
    let first = import_seed_corpus(&mut ledger, &[], std::slice::from_ref(&stream))?;
    assert_eq!((first.discovered, first.newly_appended), (1, 1));
    let l900 = ledger
        .latest()?
        .into_iter()
        .find(|record| record.id.as_str() == "L900")
        .ok_or("L900")?;
    assert_eq!(l900.domain, LessonDomain::Code);
    assert_eq!(l900.routes, vec![LessonRoute::RuleCandidate]);
    assert_eq!(import_seed_corpus(&mut ledger, &[], &[stream])?.newly_appended, 0);
    std::fs::remove_file(&path)?;
    Ok(())
}
