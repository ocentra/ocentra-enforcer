use enforcer_domain::severity::{Severity, Tier};

#[test]
fn severity_wire_form_is_lowercase() -> Result<(), serde_json::Error> {
    assert_eq!(serde_json::to_string(&Severity::Error)?, "\"error\"");
    assert_eq!(serde_json::to_string(&Severity::Warning)?, "\"warning\"");
    assert_eq!(serde_json::to_string(&Severity::Info)?, "\"info\"");
    let parsed: Severity = serde_json::from_str("\"error\"")?;
    assert_eq!(parsed, Severity::Error);
    Ok(())
}

#[test]
fn severity_rejects_unknown_variants() {
    let unknown = serde_json::from_str::<Severity>("\"fatal\"").unwrap_err();
    assert_eq!(unknown.classify(), serde_json::error::Category::Data);
    let wrong_case = serde_json::from_str::<Severity>("\"ERROR\"").unwrap_err();
    assert_eq!(wrong_case.classify(), serde_json::error::Category::Data);
}

#[test]
fn tier_wire_form_round_trips() -> Result<(), serde_json::Error> {
    assert_eq!(serde_json::to_string(&Tier::T1)?, "\"T1\"");
    let parsed: Tier = serde_json::from_str("\"T3\"")?;
    assert_eq!(parsed, Tier::T3);
    let unknown = serde_json::from_str::<Tier>("\"T4\"").unwrap_err();
    assert_eq!(unknown.classify(), serde_json::error::Category::Data);
    Ok(())
}
