use enforcer_domain::xtask_types::{
    DogfoodFamily, DogfoodGateVerdict, ToolchainOutcome, ToolchainStepOutcome, XtaskFailureDetail,
};

#[test]
fn failure_detail_rejects_blank_and_nul_text() -> Result<(), Box<dyn std::error::Error>> {
    let blank_error = match XtaskFailureDetail::try_new(String::new()) {
        Err(error) => error,
        Ok(_) => return Err(std::io::Error::other("blank failure detail was accepted").into()),
    };
    assert_eq!(blank_error.path, "xtaskFailureDetail");
    assert_eq!(blank_error.reason, "must be non-empty and contain no NUL");
    assert_eq!(blank_error.input_hint, None);

    let nul_error = match XtaskFailureDetail::try_new(String::from("bad\0detail")) {
        Err(error) => error,
        Ok(_) => {
            return Err(std::io::Error::other("NUL-bearing failure detail was accepted").into())
        }
    };
    assert_eq!(nul_error.path, "xtaskFailureDetail");
    assert_eq!(nul_error.reason, "must be non-empty and contain no NUL");
    assert_eq!(nul_error.input_hint, None);
    assert_eq!(
        XtaskFailureDetail::try_new(String::from("cargo clippy failed"))?.as_str(),
        "cargo clippy failed"
    );
    Ok(())
}

#[test]
fn verdict_and_family_tokens_round_trip_through_json() -> Result<(), serde_json::Error> {
    let verdict_wire = serde_json::to_string(&DogfoodGateVerdict::Pass)?;
    assert_eq!(verdict_wire, r#""pass""#);
    assert_eq!(
        serde_json::from_str::<DogfoodGateVerdict>(&verdict_wire)?,
        DogfoodGateVerdict::Pass
    );

    let family_wire = serde_json::to_string(&DogfoodFamily::PlanStructure)?;
    assert_eq!(family_wire, r#""plan-structure""#);
    assert_eq!(
        serde_json::from_str::<DogfoodFamily>(&family_wire)?,
        DogfoodFamily::PlanStructure
    );
    Ok(())
}

#[test]
fn only_failed_toolchain_steps_block_the_composed_outcome() -> Result<(), Box<dyn std::error::Error>>
{
    let optional_reason = XtaskFailureDetail::try_new(String::from("optional tool unavailable"))?;
    let green = ToolchainOutcome {
        fmt: ToolchainStepOutcome::Passed,
        clippy: ToolchainStepOutcome::Passed,
        deny: ToolchainStepOutcome::Skipped {
            reason: optional_reason.clone(),
        },
        audit: ToolchainStepOutcome::Skipped {
            reason: optional_reason,
        },
    };
    assert_eq!(green.verdict(), DogfoodGateVerdict::Pass);

    let red = ToolchainOutcome {
        fmt: ToolchainStepOutcome::Failed {
            detail: XtaskFailureDetail::try_new(String::from("formatter failed"))?,
        },
        clippy: ToolchainStepOutcome::Passed,
        deny: ToolchainStepOutcome::Passed,
        audit: ToolchainStepOutcome::Passed,
    };
    assert_eq!(red.verdict(), DogfoodGateVerdict::Fail);
    Ok(())
}
