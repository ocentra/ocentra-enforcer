use enforcer_domain::harness_types::{HarnessLanguage, HarnessRunId, HarnessSourceLine};

#[test]
fn harness_values_validate_and_normalize() -> Result<(), Box<dyn std::error::Error>> {
    let blank_error = match " ".parse::<HarnessRunId>() {
        Err(error) => error,
        Ok(_) => return Err(std::io::Error::other("blank harness id was accepted").into()),
    };
    assert_eq!(blank_error.path, "harnessRunId");
    assert_eq!(
        blank_error.reason,
        "invalid harness text: must be non-empty text without control characters"
    );
    assert_eq!("run-1".parse::<HarnessRunId>()?.as_str(), "run-1");
    assert_eq!(HarnessSourceLine::from_external(0).get(), 1);
    assert_eq!(
        HarnessSourceLine::from_external(u64::from(u32::MAX) + 1).finding_line(),
        None
    );
    assert_eq!(HarnessLanguage::Typescript.as_str(), "typescript");
    Ok(())
}
