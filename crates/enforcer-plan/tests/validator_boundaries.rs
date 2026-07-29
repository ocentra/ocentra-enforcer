use enforcer_domain::findings::ScanScope;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_plan::validator::{PlanCapsuleValidator, PlanSkeletonValidator};
use enforcer_validator::validator::{ValidationInput, Validator};

#[test]
fn plan_validators_report_truncated_and_empty_documents_without_panicking(
) -> Result<(), Box<dyn std::error::Error>> {
    let file: RelPath = "workpacks/truncated.md".parse()?;
    let capsule = PlanCapsuleValidator::new("PLAN-CAPSULE.1".parse::<RuleId>()?);
    let skeleton = PlanSkeletonValidator::new("PLAN-SKELETON.1".parse::<RuleId>()?);

    let truncated = capsule.validate(ValidationInput {
        file: &file,
        source: ValidationSource::from_text("<!-- agent-capsule -->"),
        scope: ScanScope::Files,
    });
    assert_eq!(truncated.len(), 1);
    assert_eq!(
        truncated.first().map(|finding| finding.title.as_str()),
        Some("truncated agent-capsule block")
    );

    let missing_headings = skeleton.validate(ValidationInput {
        file: &file,
        source: ValidationSource::from_text(""),
        scope: ScanScope::Files,
    });
    assert_eq!(missing_headings.len(), 1);
    assert_eq!(
        missing_headings
            .first()
            .map(|finding| finding.title.as_str()),
        Some("missing or out-of-order required heading")
    );
    Ok(())
}
use enforcer_domain::boundary::validation::ValidationSource;
