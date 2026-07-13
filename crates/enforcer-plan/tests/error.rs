use enforcer_coordination::error::CoordinationError;
use enforcer_plan::error::PlanError;

#[test]
fn coordination_error_converts_to_the_single_plan_error_surface() {
    let error: PlanError = CoordinationError::rejected("claim conflict").into();

    assert_eq!(error.to_string(), "coordination error: claim conflict");
}
