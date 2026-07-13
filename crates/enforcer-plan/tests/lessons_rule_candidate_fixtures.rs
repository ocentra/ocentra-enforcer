use std::collections::HashMap;

use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_plan::lessons::{
    run_doctor, ArtifactRef, LessonDomain, LessonId, LessonRecord, LessonRoute,
    RuleCandidateFixtures,
};

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
    contents.insert(artifact, "lessonId L9".to_owned());
    let rule_id: RuleId = "LESSON-DOCTOR.1".parse()?;

    let findings = run_doctor(&rule_id, &[record.clone()], &contents, &HashMap::new())?;
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Error);

    for incomplete in [
        RuleCandidateFixtures::MissingBoth,
        RuleCandidateFixtures::MissingFail,
        RuleCandidateFixtures::MissingPass,
    ] {
        let findings = run_doctor(
            &rule_id,
            &[record.clone()],
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
