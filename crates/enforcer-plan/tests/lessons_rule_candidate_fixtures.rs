use std::collections::HashMap;

use enforcer_domain::ids::RuleId;
use enforcer_domain::plan_types::{
    ArtifactRef, LessonDomain, LessonId, LessonRoute, PlanCondition, PlanFileContent,
    RuleCandidateFixtures,
};
use enforcer_domain::severity::Severity;
use enforcer_plan::lessons::{run_doctor, LessonRecord};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn doctor_requires_complete_fixture_parity_for_code_rule_candidates() -> TestResult {
    let id: LessonId = "L9".parse()?;
    let artifact: ArtifactRef = "rule-candidate.json#L9".parse()?;
    let record = LessonRecord {
        id: id.clone(),
        date: "2026-07-13".parse()?,
        domain: LessonDomain::Code,
        observed: "rule candidate needs fixture parity".parse()?,
        lesson: "ship both fail and pass fixtures".parse()?,
        routes: vec![LessonRoute::RuleCandidate],
        landed_at: vec![artifact.clone()],
        supersedes_seq: None,
    };
    let mut contents = HashMap::new();
    contents.insert(
        artifact,
        PlanFileContent::try_new("lessonId L9".to_owned())?,
    );
    let rule_id: RuleId = "LESSON-DOCTOR.1".parse()?;

    let findings = run_doctor(
        &rule_id,
        std::slice::from_ref(&record),
        &contents,
        &HashMap::new(),
    )?;
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Error);

    for incomplete in [
        RuleCandidateFixtures::MissingBoth,
        RuleCandidateFixtures::MissingFail,
        RuleCandidateFixtures::MissingPass,
    ] {
        let findings = run_doctor(
            &rule_id,
            std::slice::from_ref(&record),
            &contents,
            &HashMap::from([(id.clone(), incomplete)]),
        )?;
        assert_eq!(findings.len(), 1, "{incomplete:?} must fail closed");
        assert_eq!(findings[0].severity, Severity::Error);
    }

    let findings = run_doctor(
        &rule_id,
        &[record],
        &contents,
        &HashMap::from([(id, RuleCandidateFixtures::Complete)]),
    )?;
    assert!(
        findings.is_empty(),
        "expected green with complete parity: {findings:?}"
    );
    Ok(())
}

#[test]
fn doctor_rejects_duplicate_artifact_references_for_distinct_required_routes() -> TestResult {
    let id: LessonId = "L10".parse()?;
    let artifact: ArtifactRef = "rule-candidate.json#L10".parse()?;
    let record = LessonRecord {
        id,
        date: "2026-07-13".parse()?,
        domain: LessonDomain::Harness,
        observed: "one artifact was repeated in persisted input".parse()?,
        lesson: "each declared route needs its own landed artifact reference".parse()?,
        routes: vec![LessonRoute::DoctrineBlock, LessonRoute::Skill],
        landed_at: vec![artifact.clone(), artifact.clone()],
        supersedes_seq: None,
    };
    let contents = HashMap::from([(
        artifact,
        PlanFileContent::try_new("lessonId L10".to_owned())?,
    )]);
    let rule_id: RuleId = "LESSON-DOCTOR.1".parse()?;

    assert_eq!(record.landing_condition(), PlanCondition::Unsatisfied);
    let findings = run_doctor(&rule_id, &[record], &contents, &HashMap::new())?;
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Error);
    assert!(findings[0]
        .detail
        .as_str()
        .contains("only 1 landed artifact(s) verified"));
    Ok(())
}

#[test]
fn duplicate_route_entries_do_not_inflate_required_landing_count() -> TestResult {
    let artifact: ArtifactRef = "doctrine.md#L11".parse()?;
    let record = LessonRecord {
        id: "L11".parse()?,
        date: "2026-07-13".parse()?,
        domain: LessonDomain::Harness,
        observed: "legacy input repeated one route".parse()?,
        lesson: "route identity is a set, not an inflated count".parse()?,
        routes: vec![LessonRoute::DoctrineBlock, LessonRoute::DoctrineBlock],
        landed_at: vec![artifact],
        supersedes_seq: None,
    };

    assert_eq!(record.landing_condition(), PlanCondition::Satisfied);
    Ok(())
}
