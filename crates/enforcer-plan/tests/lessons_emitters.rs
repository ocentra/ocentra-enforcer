//! Consumer-boundary verification for lesson artifact emission.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_plan::lessons::{
    emit_doctrine_block, emit_forest_node, emit_rule_candidate, emit_skill, route, ArtifactRef,
    EmitFs, LessonDomain, LessonRecord, LessonRoute,
};
use enforcer_plan::error::PlanError;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn sample_record(id: &str) -> TestResultRecord {
    Ok(LessonRecord {
        id: id.parse()?,
        date: "2026-07-13".parse()?,
        domain: LessonDomain::Harness,
        observed: "a consumer-visible observation".parse()?,
        lesson: "a consumer-visible lesson".parse()?,
        routes: vec![LessonRoute::DoctrineBlock, LessonRoute::Skill],
        landed_at: Vec::new(),
        supersedes_seq: None,
    })
}

type TestResultRecord = Result<LessonRecord, Box<dyn std::error::Error>>;

#[derive(Default)]
struct FixtureFs {
    files: HashMap<PathBuf, String>,
}

impl FixtureFs {
    fn seed(&mut self, path: &Path, content: &str) {
        self.files.insert(path.to_path_buf(), content.to_owned());
    }

    fn file_count(&self) -> usize {
        self.files.len()
    }

    fn get(&self, path: &Path) -> Option<&str> {
        self.files.get(path).map(String::as_str)
    }
}

impl EmitFs for FixtureFs {
    fn read(&self, path: &Path) -> Result<Option<String>, PlanError> {
        Ok(self.files.get(path).cloned())
    }

    fn write(&mut self, path: &Path, content: &str) -> Result<(), PlanError> {
        self.files.insert(path.to_path_buf(), content.to_owned());
        Ok(())
    }
}

#[test]
fn emitters_render_the_lesson_identity_and_preserve_existing_blocks() -> TestResult {
    let record = sample_record("L1")?;
    let target = PathBuf::from("AGENTS.md");
    let mut fs = FixtureFs::default();
    fs.seed(
        &target,
        "<!-- lesson:L0 -->\nprevious lesson\n<!-- /lesson:L0 -->\n",
    );

    let doctrine = emit_doctrine_block(&mut fs, &record, &target, false)?;
    assert!(doctrine.wrote);
    assert!(doctrine.rendered.contains("L1"));
    let content = fs.get(&target).ok_or("emitter did not write target")?;
    assert!(content.contains("previous lesson"));
    assert!(content.contains("<!-- lesson:L1 -->"));

    let skill = emit_skill(&mut fs, &record, &PathBuf::from("skill.md"), false)?;
    let forest = emit_forest_node(&mut fs, &record, &PathBuf::from("forest.md"), false)?;
    assert!(skill.rendered.contains("L1"));
    assert!(forest.rendered.contains("LEAF -> L1"));
    Ok(())
}

#[test]
fn reemitting_the_same_lesson_replaces_its_managed_block() -> TestResult {
    let record = sample_record("L1")?;
    let target = PathBuf::from("AGENTS.md");
    let mut fs = FixtureFs::default();
    emit_doctrine_block(&mut fs, &record, &target, false)?;
    emit_doctrine_block(&mut fs, &record, &target, false)?;
    let content = fs.get(&target).ok_or("emitter did not write target")?;
    assert_eq!(content.matches("<!-- lesson:L1 -->").count(), 1);
    Ok(())
}

#[test]
fn dry_run_never_mutates_the_injected_filesystem() -> TestResult {
    let record = sample_record("L2")?;
    let target = PathBuf::from("AGENTS.md");
    let mut fs = FixtureFs::default();
    let before = fs.file_count();
    let outcome = emit_doctrine_block(&mut fs, &record, &target, true)?;
    assert!(!outcome.wrote);
    assert_eq!(fs.file_count(), before);
    Ok(())
}

#[test]
fn rule_candidate_output_is_json_and_carries_code_domain_identity() -> TestResult {
    let mut record = sample_record("L9")?;
    record.domain = LessonDomain::Code;
    record.routes = vec![LessonRoute::RuleCandidate];
    let mut fs = FixtureFs::default();
    let outcome = emit_rule_candidate(
        &mut fs,
        &record,
        &PathBuf::from("rule-candidate.json"),
        false,
    )?;
    let value: serde_json::Value = serde_json::from_str(&outcome.rendered)?;
    assert_eq!(value["lessonId"], "L9");
    assert_eq!(value["domain"], "code");
    Ok(())
}

#[test]
fn route_only_emits_declared_routes_that_have_explicit_targets() -> TestResult {
    let record = sample_record("L4")?;
    let mut fs = FixtureFs::default();
    let targets = HashMap::from([(LessonRoute::DoctrineBlock, PathBuf::from("AGENTS.md"))]);
    let outcomes = route(&mut fs, &record, &targets, false)?;
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].path, PathBuf::from("AGENTS.md"));
    Ok(())
}

#[test]
fn route_emits_each_declared_route_at_most_once() -> TestResult {
    let mut record = sample_record("L40")?;
    record.routes = vec![LessonRoute::DoctrineBlock, LessonRoute::DoctrineBlock];
    let mut fs = FixtureFs::default();
    let targets = HashMap::from([(LessonRoute::DoctrineBlock, PathBuf::from("AGENTS.md"))]);

    let outcomes = route(&mut fs, &record, &targets, false)?;

    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].path, PathBuf::from("AGENTS.md"));
    Ok(())
}

#[test]
fn doctor_rejects_unlanded_artifacts_at_the_public_boundary() -> TestResult {
    let record = sample_record("L5")?;
    let rule_id: RuleId = "LESSON-DOCTOR.1".parse()?;
    let findings = enforcer_plan::lessons::run_doctor(
        &rule_id,
        &[record],
        &HashMap::<ArtifactRef, String>::new(),
        &HashMap::new(),
    )?;
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Error);
    assert!(findings[0].detail.contains("L5"));
    Ok(())
}

#[test]
fn doctor_rejects_missing_identity_and_accepts_complete_landing() -> TestResult {
    let mut record = sample_record("L6")?;
    let doctrine: ArtifactRef = "AGENTS.md#L6".parse()?;
    let skill: ArtifactRef = "skill.md#L6".parse()?;
    record.landed_at = vec![doctrine.clone(), skill.clone()];
    let rule_id: RuleId = "LESSON-DOCTOR.1".parse()?;
    let absent_identity = HashMap::from([
        (doctrine.clone(), "not the expected identifier".to_owned()),
        (skill.clone(), "still no identity".to_owned()),
    ]);
    let rejected = enforcer_plan::lessons::run_doctor(
        &rule_id,
        &[record.clone()],
        &absent_identity,
        &HashMap::new(),
    )?;
    assert_eq!(rejected.len(), 1);
    assert!(rejected.iter().all(|finding| finding.severity == Severity::Error));

    let landed = HashMap::from([
        (doctrine, "<!-- lesson:L6 -->".to_owned()),
        (skill, "skill L6 section".to_owned()),
    ]);
    let accepted = enforcer_plan::lessons::run_doctor(
        &rule_id,
        &[record],
        &landed,
        &HashMap::new(),
    )?;
    assert!(accepted.is_empty(), "complete landing must be green: {accepted:?}");
    Ok(())
}

#[test]
fn doctor_marks_plan_doc_only_capture_as_a_transitional_warning() -> TestResult {
    let mut record = sample_record("L7")?;
    record.routes = vec![LessonRoute::PlanDoc];
    let rule_id: RuleId = "LESSON-DOCTOR.1".parse()?;
    let findings = enforcer_plan::lessons::run_doctor(
        &rule_id,
        &[record],
        &HashMap::new(),
        &HashMap::new(),
    )?;
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Warning);
    Ok(())
}

#[test]
fn all_route_outputs_match_the_pinned_golden_artifacts() -> TestResult {
    let mut record = sample_record("L900-GOLDEN")?;
    record.date = "2026-07-04".parse()?;
    record.observed = "golden fixture observation".parse()?;
    record.lesson = "golden fixture lesson text".parse()?;
    record.routes = vec![
        LessonRoute::DoctrineBlock,
        LessonRoute::Skill,
        LessonRoute::RuleCandidate,
        LessonRoute::ForestNode,
    ];
    let mut fs = FixtureFs::default();
    let artifacts = [
        (
            emit_doctrine_block(&mut fs, &record, &PathBuf::from("doctrine.md"), false)?.rendered,
            "doctrine-block.md",
        ),
        (
            emit_skill(&mut fs, &record, &PathBuf::from("skill.md"), false)?.rendered,
            "skill.md",
        ),
        (
            emit_rule_candidate(&mut fs, &record, &PathBuf::from("candidate.json"), false)?.rendered,
            "rule-candidate.json",
        ),
        (
            emit_forest_node(&mut fs, &record, &PathBuf::from("forest.md"), false)?.rendered,
            "forest-node.md",
        ),
    ];
    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lessons/golden");
    for (rendered, fixture_name) in artifacts {
        assert_eq!(rendered, std::fs::read_to_string(fixture_root.join(fixture_name))?);
    }
    Ok(())
}

#[test]
fn doctor_is_green_when_every_declared_route_has_identity_bearing_output() -> TestResult {
    let mut record = sample_record("L8")?;
    record.routes = vec![
        LessonRoute::DoctrineBlock,
        LessonRoute::Skill,
        LessonRoute::RuleCandidate,
        LessonRoute::ForestNode,
    ];
    let mut fs = FixtureFs::default();
    let outputs = [
        emit_doctrine_block(&mut fs, &record, &PathBuf::from("AGENTS.md"), false)?,
        emit_skill(&mut fs, &record, &PathBuf::from("skill.md"), false)?,
        emit_rule_candidate(&mut fs, &record, &PathBuf::from("candidate.json"), false)?,
        emit_forest_node(&mut fs, &record, &PathBuf::from("forest.md"), false)?,
    ];
    let artifacts: Vec<ArtifactRef> = outputs
        .iter()
        .map(|output| format!("{}#L8", output.path.display()).parse())
        .collect::<Result<_, _>>()?;
    record.landed_at = artifacts.clone();
    let contents = artifacts
        .into_iter()
        .zip(outputs.into_iter().map(|output| output.rendered))
        .collect::<HashMap<_, _>>();
    let rule_id: RuleId = "LESSON-DOCTOR.1".parse()?;
    let findings = enforcer_plan::lessons::run_doctor(
        &rule_id,
        &[record],
        &contents,
        &HashMap::new(),
    )?;
    assert!(findings.is_empty(), "fully landed lesson must be green: {findings:?}");
    Ok(())
}
