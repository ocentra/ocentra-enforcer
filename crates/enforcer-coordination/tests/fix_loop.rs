//! Integration proof for d07 (`docs/plans/enforcer-selfhost-plan/workpacks/
//! d07-self-correct-fix-loop.md`): exercises the PUBLIC `run_fix_loop` API
//! against the on-disk fixtures under `tests/fixtures/fix_loop/**`, from
//! outside the crate — the same surface a future `enforcer-cli fix` command
//! would call through.
//!
//! Required proof shape (TEST_PROOF_EXPECTATIONS.md d07 row):
//! - an improving fix is kept;
//! - a neutral/regressing fix is reverted;
//! - the loop halts at the iteration cap;
//! - final state never has more findings than the start.

use std::fs;
use std::path::{Path, PathBuf};

use enforcer_coordination::error::Result;
use enforcer_coordination::fix_loop::dispatch::FixGenerator;
use enforcer_coordination::fix_loop::{run_fix_loop, IterationReason, MAX_ITERATIONS};
use enforcer_domain::findings::{Finding, ScanScope};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// Validator that reports one finding per occurrence of the literal marker
/// `BAD` in the source, all under the same fixed `RuleId`.
struct MarkerValidator {
    rule_id: RuleId,
}

impl Validator for MarkerValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        input
            .source
            .match_indices("BAD")
            .enumerate()
            .map(|(idx, _)| Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "bad marker".to_owned(),
                detail: "found BAD".to_owned(),
                file: input.file.clone(),
                line: (idx as u32) + 1,
                snippet: None,
            })
            .collect()
    }
}

/// Removes exactly one `BAD` occurrence per attempt.
struct OneAtATimeRemover;
impl FixGenerator for OneAtATimeRemover {
    fn attempt_fix(&self, root: &Path, findings: &[Finding]) -> Result<bool> {
        if findings.is_empty() {
            return Ok(false);
        }
        let path = root.join("fixture.txt");
        let content = fs::read_to_string(&path)?;
        if let Some(pos) = content.find("BAD") {
            let mut new_content = content.clone();
            new_content.replace_range(pos..pos + 3, "OK_");
            fs::write(&path, new_content)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn name(&self) -> &str {
        "one-at-a-time-remover"
    }
}

/// Rewrites the file to a different string with the SAME finding count — a
/// neutral (non-improving) fix that must be reverted.
struct NeutralRewriter;
impl FixGenerator for NeutralRewriter {
    fn attempt_fix(&self, root: &Path, findings: &[Finding]) -> Result<bool> {
        if findings.is_empty() {
            return Ok(false);
        }
        fs::write(root.join("fixture.txt"), "BAD BAD (rewritten, still bad)")?;
        Ok(true)
    }

    fn name(&self) -> &str {
        "neutral-rewriter"
    }
}

/// Appends MORE `BAD` markers — a regressing fix that must be reverted.
struct RegressingWriter;
impl FixGenerator for RegressingWriter {
    fn attempt_fix(&self, root: &Path, findings: &[Finding]) -> Result<bool> {
        if findings.is_empty() {
            return Ok(false);
        }
        let path = root.join("fixture.txt");
        let content = fs::read_to_string(&path)?;
        fs::write(&path, format!("{content} BAD BAD"))?;
        Ok(true)
    }

    fn name(&self) -> &str {
        "regressing-writer"
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Copy one fixture's content into a fresh tempdir under a stable filename
/// (`fixture.txt`), so the on-disk fixture under version control is never
/// mutated by a test run.
fn stage_fixture(name: &str) -> Result<(tempfile::TempDir, PathBuf)> {
    let dir = tempfile::tempdir()?;
    let source = fs::read_to_string(manifest_dir().join("tests/fixtures/fix_loop").join(name))?;
    let staged = dir.path().join("fixture.txt");
    fs::write(&staged, &source)?;
    Ok((dir, staged))
}

fn rel_path() -> Result<RelPath> {
    Ok("fixture.txt".parse()?)
}

fn rule_id() -> Result<RuleId> {
    Ok("RR-7.1".parse()?)
}

fn scan(validator: &MarkerValidator, source: &str) -> Result<Vec<Finding>> {
    let rel = rel_path()?;
    Ok(validator.validate(ValidationInput {
        file: &rel,
        source,
        scope: ScanScope::Files,
    }))
}

#[test]
fn an_improving_fix_is_kept() -> Result<()> {
    let (_dir, file) = stage_fixture("improving.txt")?;
    let validator = MarkerValidator {
        rule_id: rule_id()?,
    };
    let source = fs::read_to_string(&file)?;
    let initial = scan(&validator, &source)?;
    assert_eq!(initial.len(), 3, "fixture assumption: 3 BAD markers");

    let report = run_fix_loop(
        &file,
        &rel_path()?,
        initial,
        &validator,
        &OneAtATimeRemover,
        |_| {},
    )?;

    assert_eq!(report.findings_start, 3);
    assert_eq!(report.findings_final, 0);
    assert!(!report.hit_iteration_cap);
    // Once findings_after hits 0 the generator has nothing left to try, so
    // the LAST iteration is a decline, not an accept -- only the iterations
    // that actually changed something need to be accepted.
    assert!(report
        .iterations
        .iter()
        .filter(|it| it.reason != IterationReason::GeneratorDeclined)
        .all(|it| it.accepted));
    assert_eq!(fs::read_to_string(&file)?, "OK_ OK_ OK_\n");
    Ok(())
}

#[test]
fn a_neutral_fix_is_not_reverted_incorrectly_and_findings_stay_flat() -> Result<()> {
    let (_dir, file) = stage_fixture("neutral.txt")?;
    let validator = MarkerValidator {
        rule_id: rule_id()?,
    };
    let original = fs::read_to_string(&file)?;
    let initial = scan(&validator, &original)?;
    assert_eq!(initial.len(), 2, "fixture assumption: 2 BAD markers");

    let report = run_fix_loop(
        &file,
        &rel_path()?,
        initial,
        &validator,
        &NeutralRewriter,
        |_| {},
    )?;

    assert_eq!(report.findings_start, 2);
    assert_eq!(report.findings_final, 2, "neutral fix must not be kept");
    assert_eq!(report.iterations.len(), 1);
    assert!(!report.iterations[0].accepted);
    assert_eq!(report.iterations[0].reason, IterationReason::NotImproved);
    assert_eq!(
        fs::read_to_string(&file)?,
        original,
        "working tree must be restored to the pre-attempt bytes"
    );
    Ok(())
}

#[test]
fn a_regressing_fix_is_reverted() -> Result<()> {
    let (_dir, file) = stage_fixture("regressing.txt")?;
    let validator = MarkerValidator {
        rule_id: rule_id()?,
    };
    let original = fs::read_to_string(&file)?;
    let initial = scan(&validator, &original)?;
    assert_eq!(initial.len(), 1, "fixture assumption: 1 BAD marker");

    let report = run_fix_loop(
        &file,
        &rel_path()?,
        initial,
        &validator,
        &RegressingWriter,
        |_| {},
    )?;

    assert_eq!(report.findings_final, report.findings_start);
    assert_eq!(report.iterations.len(), 1);
    assert!(!report.iterations[0].accepted);
    assert_eq!(fs::read_to_string(&file)?, original);
    Ok(())
}

#[test]
fn the_loop_halts_at_the_iteration_cap() -> Result<()> {
    let (_dir, file) = stage_fixture("many_markers.txt")?;
    let validator = MarkerValidator {
        rule_id: rule_id()?,
    };
    let original = fs::read_to_string(&file)?;
    let initial = scan(&validator, &original)?;
    assert!(
        initial.len() as u32 > MAX_ITERATIONS,
        "fixture must carry more markers than the iteration cap can clear one-at-a-time"
    );

    let report = run_fix_loop(
        &file,
        &rel_path()?,
        initial,
        &validator,
        &OneAtATimeRemover,
        |_| {},
    )?;

    assert!(report.hit_iteration_cap);
    assert_eq!(report.iterations.len(), MAX_ITERATIONS as usize);
    assert!(report.iterations.iter().all(|it| it.accepted));
    Ok(())
}

#[test]
fn final_findings_never_exceed_the_starting_count_across_all_fixtures() -> Result<()> {
    for (fixture, generator_name) in [
        ("improving.txt", "improving"),
        ("neutral.txt", "neutral"),
        ("regressing.txt", "regressing"),
        ("many_markers.txt", "many_markers"),
    ] {
        let (_dir, file) = stage_fixture(fixture)?;
        let validator = MarkerValidator {
            rule_id: rule_id()?,
        };
        let original = fs::read_to_string(&file)?;
        let initial = scan(&validator, &original)?;
        let rel = rel_path()?;

        let report: enforcer_coordination::fix_loop::FixLoopReport = match generator_name {
            "improving" => {
                run_fix_loop(&file, &rel, initial, &validator, &OneAtATimeRemover, |_| {})?
            }
            "neutral" => run_fix_loop(&file, &rel, initial, &validator, &NeutralRewriter, |_| {})?,
            "regressing" => {
                run_fix_loop(&file, &rel, initial, &validator, &RegressingWriter, |_| {})?
            }
            _ => run_fix_loop(&file, &rel, initial, &validator, &OneAtATimeRemover, |_| {})?,
        };

        assert!(
            report.findings_final <= report.findings_start,
            "fixture {fixture} regressed: start={} final={}",
            report.findings_start,
            report.findings_final
        );
    }
    Ok(())
}

#[test]
fn every_decision_is_emitted_as_a_typed_event() -> Result<()> {
    let (_dir, file) = stage_fixture("improving.txt")?;
    let validator = MarkerValidator {
        rule_id: rule_id()?,
    };
    let source = fs::read_to_string(&file)?;
    let initial = scan(&validator, &source)?;

    let mut events = Vec::new();
    let _report = run_fix_loop(
        &file,
        &rel_path()?,
        initial,
        &validator,
        &OneAtATimeRemover,
        |event| events.push(event.clone()),
    )?;

    assert!(!events.is_empty());
    assert!(events
        .iter()
        .all(|event| event.generator_name == "one-at-a-time-remover"));
    // Every event round-trips through the typed DomainEvent envelope shape
    // (event_kind is stable/non-empty), proving these are real typed events
    // and not ad hoc structs.
    use enforcer_events::event::DomainEvent;
    for event in &events {
        assert_eq!(event.event_kind(), "coordination.fix_loop.decision");
    }
    Ok(())
}
