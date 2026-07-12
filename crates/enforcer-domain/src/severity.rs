//! Severity and enforcement-tier enums (closed sets; serde rejects unknown
//! variants at the boundary by construction).

/// Finding severity, lowercase on the wire (`"error"`, `"warning"`,
/// `"info"`) to match the legacy `.mjs` report shape.
// SERDE-TAG-JUSTIFICATION: this closed scalar enum is deliberately a JSON
// string; object tagging would change the public wire contract.
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
#[doc = "SERDE-TAG-JUSTIFICATION: scalar JSON string contract; object tagging is inapplicable."]
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
// SERDE-TAG-JUSTIFICATION: this closed scalar enum is deliberately a JSON
// string; object tagging would change the public wire contract.
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
#[doc = "SERDE-TAG-JUSTIFICATION: scalar JSON string contract; object tagging is inapplicable."]
pub enum Tier {
    /// Typed / compile-time / hard-gate enforcement.
    T1,
    /// Scored scan enforcement.
    T2,
    /// Review-assist enforcement.
    T3,
}
