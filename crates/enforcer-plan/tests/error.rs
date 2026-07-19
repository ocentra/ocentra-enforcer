use enforcer_coordination::error::CoordinationError;
use enforcer_domain::coordination_types::CoordinationRejection;
use enforcer_plan::error::PlanError;

#[test]
fn coordination_error_converts_to_the_single_plan_error_surface(
) -> Result<(), Box<dyn std::error::Error>> {
    let rejection = CoordinationRejection::try_from("claim conflict".to_owned())?;
    let error: PlanError = CoordinationError::rejected(rejection).into();

    assert_eq!(error.to_string(), "coordination error: claim conflict");
    Ok(())
}
