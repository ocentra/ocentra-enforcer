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
fn severity_parse_wire_rejects_invalid_empty_oversized_and_malformed_variants(
) -> Result<(), Box<dyn std::error::Error>> {
    let unknown = serde_json::from_str::<Severity>("\"fatal\"")
        .err()
        .ok_or("fatal severity must be rejected")?;
    assert_eq!(unknown.classify(), serde_json::error::Category::Data);
    let wrong_case = serde_json::from_str::<Severity>("\"ERROR\"")
        .err()
        .ok_or("uppercase severity must be rejected")?;
    assert_eq!(wrong_case.classify(), serde_json::error::Category::Data);
    for invalid in ["\"\"", "\"fatal-severity-name-that-is-oversized\"", "[]"] {
        let rejected = match serde_json::from_str::<Severity>(invalid) {
            Err(error) => error,
            Ok(value) => {
                return Err(format!("invalid severity wire input produced {value:?}").into());
            }
        };
        assert_eq!(rejected.classify(), serde_json::error::Category::Data);
    }
    Ok(())
}

#[test]
fn tier_wire_form_round_trips() -> Result<(), serde_json::Error> {
    assert_eq!(serde_json::to_string(&Tier::T1)?, "\"T1\"");
    let parsed: Tier = serde_json::from_str("\"T3\"")?;
    assert_eq!(parsed, Tier::T3);
    let unknown = serde_json::from_str::<Tier>("\"T4\"")
        .err()
        .ok_or_else(|| serde_json::Error::io(std::io::Error::other("T4 tier must be rejected")))?;
    assert_eq!(unknown.classify(), serde_json::error::Category::Data);
    Ok(())
}
