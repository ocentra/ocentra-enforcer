//! Severity and enforcement-tier enums (closed sets; serde rejects unknown
//! variants at the boundary by construction).

/// Finding severity, lowercase on the wire (`"error"`, `"warning"`,
/// `"info"`) to match the legacy `.mjs` report shape.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    ts_rs::TS,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Blocking violation.
    Error,
    /// Non-blocking warning.
    Warning,
    /// Informational note.
    Info,
}

/// Mechanical-enforcement tier (doctrine: T1 typed/compile-time, T2 scored
/// scan, T3 review-assist). Wire form is `"T1"`/`"T2"`/`"T3"`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    ts_rs::TS,
)]
pub enum Tier {
    /// Typed / compile-time / hard-gate enforcement.
    T1,
    /// Scored scan enforcement.
    T2,
    /// Review-assist enforcement.
    T3,
}

#[cfg(test)]
mod tests {
    use super::{Severity, Tier};

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
        assert!(serde_json::from_str::<Severity>("\"fatal\"").is_err());
        assert!(serde_json::from_str::<Severity>("\"ERROR\"").is_err());
    }

    #[test]
    fn tier_wire_form_round_trips() -> Result<(), serde_json::Error> {
        assert_eq!(serde_json::to_string(&Tier::T1)?, "\"T1\"");
        let parsed: Tier = serde_json::from_str("\"T3\"")?;
        assert_eq!(parsed, Tier::T3);
        assert!(serde_json::from_str::<Tier>("\"T4\"").is_err());
        Ok(())
    }
}
