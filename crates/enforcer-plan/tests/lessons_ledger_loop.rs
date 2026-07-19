//! Ledger behavior verification for the lesson-capture loop and seed importer.
//!
//! These tests exercise the public `enforcer_plan::lessons` API from the
//! consumer boundary. They deliberately keep the append/replay/import loop
//! outside the implementation module so observable behavior does not depend
//! on crate-private test access.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use enforcer_core::hash_chain::link_digest;
use enforcer_domain::plan_types::{
    ArtifactRef, CapturedDate, LessonDomain, LessonId, LessonRoute, PlanArtifactPath,
    PlanCondition, PlanDiagnosticDetail, PlanEmissionMode, PlanFileContent, PlanImportCount,
};
use enforcer_plan::error::PlanError;
use enforcer_plan::lessons::{
    add, emit_doctrine_block, import_seed_corpus, list, EmitFs, LessonLedger, LessonRecord,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn diagnostic(raw: &str) -> PlanDiagnosticDetail {
    let mut candidate = raw.to_owned();
    loop {
        if let Ok(value) = PlanDiagnosticDetail::try_new(candidate) {
            return value;
        }
        candidate = "invalid test diagnostic".to_owned();
    }
}

fn temp_ledger_path(name: &str) -> Result<PlanArtifactPath, Box<dyn std::error::Error>> {
    static NEXT_TEST_LEDGER: AtomicUsize = AtomicUsize::new(0);
    let unique = format!(
        "enforcer-plan-lessons-external-{}-{}-{name}.ndjson",
        std::process::id(),
        NEXT_TEST_LEDGER.fetch_add(1, Ordering::Relaxed)
    );
    Ok(PlanArtifactPath::try_new(
        std::env::temp_dir().join(unique),
    )?)
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
    fn read(&self, _: &PlanArtifactPath) -> Result<Option<PlanFileContent>, PlanError> {
        Err(PlanError::Io {
            path: PlanArtifactPath::try_new(PathBuf::from("denied.md")).map_err(|error| {
                PlanError::GraphInvalid {
                    reason: diagnostic(&error.to_string()),
                }
            })?,
            reason: diagnostic("permission denied"),
        })
    }

    fn write(&mut self, _: &PlanArtifactPath, _: &PlanFileContent) -> Result<(), PlanError> {
        Ok(())
    }
}

#[test]
fn emitter_fails_closed_when_an_existing_target_cannot_be_read() -> TestResult {
    let mut fs = DeniedReadFs;
    let target = PlanArtifactPath::try_new(PathBuf::from("denied.md"))?;
    let result = emit_doctrine_block(
        &mut fs,
        &sample_record("L1")?,
        &target,
        PlanEmissionMode::Apply,
    );
    let error = match result {
        Err(error) => error,
        Ok(_) => return Err("unreadable target was treated as absent".into()),
    };
    match error {
        PlanError::Io { path, reason } => {
            assert_eq!(path.as_path(), std::path::Path::new("denied.md"));
            assert_eq!(reason.as_str(), "permission denied");
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
    for bad in [
        "", "1", "l1", "M1", "Lalpha", "L1x", "L-", "L1-", "L1--FILL",
    ] {
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
    assert_eq!(
        "2026-07-13"
            .parse::<CapturedDate>()
            .map(|date| date.to_string()),
        Ok("2026-07-13".to_owned())
    );
    assert_eq!(
        "".parse::<CapturedDate>().map(|date| date.to_string()),
        Ok(String::new())
    );
    assert!(matches!(
        "2026/07/13".parse::<CapturedDate>(),
        Err(error) if error.path == "capturedDate"
    ));
}

#[test]
fn ledger_round_trips_and_verifies_from_the_public_api() -> TestResult {
    let path = temp_ledger_path("round-trip")?;
    {
        let mut ledger = LessonLedger::open(path.clone())?;
        ledger.append(sample_record("L1")?)?;
        ledger.append(sample_record("L2")?)?;
    }
    let ledger = LessonLedger::open(path.clone())?;
    assert_eq!(usize::from(ledger.verify_on_replay()?), 2);
    assert_eq!(ledger.list()?.len(), 2);
    std::fs::remove_file(path.as_path())?;
    Ok(())
}

#[test]
fn rewriting_a_prior_row_is_detected_on_public_open() -> TestResult {
    let path = temp_ledger_path("tamper")?;
    {
        let mut ledger = LessonLedger::open(path.clone())?;
        ledger.append(sample_record("L1")?)?;
        ledger.append(sample_record("L2")?)?;
    }
    let content = std::fs::read_to_string(path.as_path())?;
    let mut lines: Vec<String> = content.lines().map(str::to_owned).collect();
    let mut value: serde_json::Value = serde_json::from_str(&lines[0])?;
    value["record"]["lesson"] = serde_json::json!("REWRITTEN");
    lines[0] = value.to_string();
    std::fs::write(path.as_path(), lines.join("\n") + "\n")?;

    match LessonLedger::open(path.clone()) {
        Err(PlanError::Io { path, reason }) => {
            assert_eq!(path.as_path(), std::path::Path::new("lesson ledger"));
            assert!(reason
                .as_str()
                .starts_with("lesson ledger tamper detected at line 0"));
        }
        Err(other) => return Err(format!("expected tamper rejection, received {other}").into()),
        Ok(_) => return Err("expected tamper rejection, received open ledger".into()),
    }
    std::fs::remove_file(path.as_path())?;
    Ok(())
}

#[test]
fn supersede_appends_and_folds_latest_state() -> TestResult {
    let path = temp_ledger_path("supersede")?;
    let mut ledger = LessonLedger::open(path.clone())?;
    ledger.append(sample_record("L1")?)?;
    let artifact: ArtifactRef = "docs/AGENTS.md#L1".parse()?;
    ledger.supersede(&"L1".parse()?, vec![artifact.clone()])?;

    assert_eq!(ledger.list()?.len(), 2);
    assert_eq!(usize::from(ledger.verify_on_replay()?), 2);
    let latest = ledger.latest()?;
    assert_eq!(latest.len(), 1);
    assert_eq!(latest[0].landed_at, vec![artifact]);
    std::fs::remove_file(path.as_path())?;
    Ok(())
}

#[test]
fn append_rejects_a_duplicate_id_without_supersede() -> TestResult {
    let path = temp_ledger_path("dup-reject")?;
    let mut ledger = LessonLedger::open(path.clone())?;
    ledger.append(sample_record("L1")?)?;
    match ledger.append(sample_record("L1")?) {
        Err(PlanError::Io { reason, .. }) => assert_eq!(
            reason.as_str(),
            "lesson `L1` already captured; use supersede to fill in landed_at",
        ),
        Err(other) => return Err(format!("expected duplicate rejection, received {other}").into()),
        Ok(()) => return Err("expected duplicate rejection, received append success".into()),
    }
    std::fs::remove_file(path.as_path())?;
    Ok(())
}

#[test]
fn append_rejects_a_caller_supplied_supersession_link() -> TestResult {
    let path = temp_ledger_path("forged-supersession")?;
    let mut ledger = LessonLedger::open(path)?;
    let mut record = sample_record("L3")?;
    record.supersedes_seq = Some(PlanImportCount::default().into());

    match ledger.append(record) {
        Err(PlanError::Io { reason, .. }) => assert_eq!(
            reason.as_str(),
            "lesson `L3` declares a supersession; use supersede to create linked ledger rows",
        ),
        Err(other) => {
            return Err(format!("expected supersession rejection, received {other}").into())
        }
        Ok(()) => return Err("expected supersession rejection, received append success".into()),
    }
    assert_eq!(ledger.list()?.len(), 0);
    Ok(())
}

#[test]
fn add_then_list_round_trips_through_the_cli_seam() -> TestResult {
    let path = temp_ledger_path("seam")?;
    add(path.clone(), sample_record("L1")?)?;
    add(path.clone(), sample_record("L2")?)?;
    assert_eq!(
        list(path.clone(), None, PlanCondition::Unsatisfied)?.len(),
        2
    );
    assert_eq!(
        list(
            path.clone(),
            Some(LessonRoute::Skill),
            PlanCondition::Unsatisfied,
        )?
        .len(),
        2
    );
    assert_eq!(list(path.clone(), None, PlanCondition::Satisfied)?.len(), 2);
    std::fs::remove_file(path.as_path())?;
    Ok(())
}

#[test]
fn pending_list_keeps_captured_but_unrouted_lessons_visible() -> TestResult {
    let path = temp_ledger_path("unrouted-pending")?;
    let mut record = sample_record("L3")?;
    record.routes.clear();

    add(path.clone(), record)?;
    let pending = list(path.clone(), None, PlanCondition::Satisfied)?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id.as_str(), "L3");
    std::fs::remove_file(path.as_path())?;
    Ok(())
}

#[test]
fn ledger_rejects_hash_valid_supersession_that_rewrites_lesson_identity() -> TestResult {
    let path = temp_ledger_path("supersession-identity")?;
    let original = sample_record("L42")?;
    let original_bytes = serde_json::to_vec(&original)?;
    let original_digest = link_digest(None, &original_bytes);

    let mut rewritten = original.clone();
    rewritten.lesson = "rewritten lesson must not replace history".parse()?;
    rewritten.supersedes_seq = Some(PlanImportCount::default().into());
    let rewritten_bytes = serde_json::to_vec(&rewritten)?;
    let rewritten_digest = link_digest(Some(&original_digest), &rewritten_bytes);

    let rows = [
        serde_json::json!({"record": original, "digest": original_digest}),
        serde_json::json!({"record": rewritten, "digest": rewritten_digest}),
    ];
    std::fs::write(
        path.as_path(),
        format!(
            "{}\n{}\n",
            serde_json::to_string(&rows[0])?,
            serde_json::to_string(&rows[1])?
        ),
    )?;

    match LessonLedger::open(path.clone()) {
        Err(PlanError::Io { path, reason }) => {
            assert_eq!(path.as_path(), std::path::Path::new("lesson ledger"));
            assert_eq!(
                reason.as_str(),
                "invalid lesson supersession at line 1: changes immutable lesson identity fields"
            );
        }
        Ok(_) => return Err("hash-valid lesson rewrite must fail closed".into()),
        Err(other) => return Err(format!("unexpected ledger error: {other}").into()),
    }
    std::fs::remove_file(path.as_path())?;
    Ok(())
}

fn seed_markdown_fixture() -> Result<PlanFileContent, Box<dyn std::error::Error>> {
    Ok(PlanFileContent::try_new(
        r#"
| id | date | observed | lesson | landed-at | ships-via |
|---|---|---|---|---|---|
| L1 | 2026-07-04 | `coordination_init` re-init threw raw `EEXIST` | init must be idempotent | arc-16 finding (this row) | fixed MCP tool behavior (arc-16) |
| L4 | 2026-07-04 | wave-1 workers went silent until done | worker mail lifecycle is started -> progress -> done/blocked | EXECUTION_MODEL section2d | c01 doctrine payload + b06 decision forest |
| L15 | 2026-07-04 | [code] arc-02 dogfood boundary allowlist gap | rule configs must ship boundary-module globs | this row | rules-as-data (arc-04/arc-06) |
"#
        .to_owned(),
    )?)
}

#[test]
fn seed_import_is_idempotent_and_preserves_chain_integrity() -> TestResult {
    let path = temp_ledger_path("import")?;
    let mut ledger = LessonLedger::open(path.clone())?;
    let markdown = vec![seed_markdown_fixture()?];
    let first = import_seed_corpus(&mut ledger, &markdown, &[])?;
    assert_eq!(
        (
            usize::from(first.discovered),
            usize::from(first.newly_appended)
        ),
        (3, 3)
    );
    let second = import_seed_corpus(&mut ledger, &markdown, &[])?;
    assert_eq!(
        (
            usize::from(second.discovered),
            usize::from(second.newly_appended)
        ),
        (3, 0)
    );
    assert_eq!(usize::from(ledger.verify_on_replay()?), 3);
    std::fs::remove_file(path.as_path())?;
    Ok(())
}

#[test]
fn seed_import_maps_routes_and_domains_from_the_public_record_shape() -> TestResult {
    let path = temp_ledger_path("import-map")?;
    let mut ledger = LessonLedger::open(path.clone())?;
    import_seed_corpus(&mut ledger, &[seed_markdown_fixture()?], &[])?;
    let records = ledger.latest()?;
    let l1 = records
        .iter()
        .find(|record| record.id.as_str() == "L1")
        .ok_or("L1")?;
    assert_eq!(l1.domain, LessonDomain::Harness);
    assert_eq!(l1.routes, vec![LessonRoute::PlanDoc]);
    let l4 = records
        .iter()
        .find(|record| record.id.as_str() == "L4")
        .ok_or("L4")?;
    assert_eq!(
        l4.routes,
        vec![LessonRoute::DoctrineBlock, LessonRoute::ForestNode],
    );
    let l15 = records
        .iter()
        .find(|record| record.id.as_str() == "L15")
        .ok_or("L15")?;
    assert_eq!(l15.domain, LessonDomain::Code);
    assert_eq!(l15.routes, vec![LessonRoute::RuleCandidate]);
    std::fs::remove_file(path.as_path())?;
    Ok(())
}

#[test]
fn seed_import_accepts_memory_stream_rows_and_is_idempotent() -> TestResult {
    let path = temp_ledger_path("import-memory")?;
    let mut ledger = LessonLedger::open(path.clone())?;
    let stream = PlanFileContent::try_new(r#"{"id":"L900","date":"2026-07-04","domain":"code","observed":"[code] example","lesson":"example fix","shipsVia":"rules-as-data","landedAt":"docs/x.md#L900"}
{"id":"truncated"
{"id":"status-only","note":"not a lesson row"}
"#
    .to_owned())?;
    let first = import_seed_corpus(&mut ledger, &[], std::slice::from_ref(&stream))?;
    assert_eq!(
        (
            usize::from(first.discovered),
            usize::from(first.newly_appended)
        ),
        (1, 1)
    );
    let l900 = ledger
        .latest()?
        .into_iter()
        .find(|record| record.id.as_str() == "L900")
        .ok_or("L900")?;
    assert_eq!(l900.domain, LessonDomain::Code);
    assert_eq!(l900.routes, vec![LessonRoute::RuleCandidate]);
    assert_eq!(
        import_seed_corpus(&mut ledger, &[], &[stream])?.newly_appended,
        enforcer_domain::plan_types::PlanImportCount::default()
    );
    std::fs::remove_file(path.as_path())?;
    Ok(())
}
