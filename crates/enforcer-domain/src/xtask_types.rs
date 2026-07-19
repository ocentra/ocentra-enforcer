//! Canonical values shared by the workspace dogfood task runner.
//!
//! `xtask` owns process and persistence adapters, but its decisions and
//! diagnostic values live here so callers never exchange raw flags, verdict
//! strings, family labels, or unvalidated failure text.

use crate::boundary::decode_error::DecodeError;

/// Validated rendered detail for an xtask composition or subprocess failure.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for XtaskFailureDetail."]
#[doc = "BRAND-INVARIANT: non-empty text containing no NUL character."]
pub struct XtaskFailureDetail(String);

impl XtaskFailureDetail {
    /// Validate rendered failure evidence at the adapter seam.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        if value.trim().is_empty() || value.contains('\0') {
            return Err(DecodeError::new(
                "xtaskFailureDetail",
                "must be non-empty and contain no NUL",
            ));
        }
        Ok(Self(value))
    }

    /// Stable fallback used only when an upstream `Display` implementation
    /// violates the failure-detail contract.
    #[must_use]
    pub fn invalid_rendering() -> Self {
        Self(String::from(
            "upstream failure rendered invalid diagnostic text",
        ))
    }

    /// View the validated diagnostic.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for XtaskFailureDetail {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Whether dogfood includes the external Rust toolchain subprocesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for ToolchainMode."]
pub enum ToolchainMode {
    Include,
    Skip,
}

/// One external toolchain step's typed result.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ToolchainStepOutcome."]
pub enum ToolchainStepOutcome {
    Passed,
    Skipped { reason: XtaskFailureDetail },
    Failed { detail: XtaskFailureDetail },
}

/// The four standard Rust toolchain outcomes composed by dogfood.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ToolchainOutcome."]
pub struct ToolchainOutcome {
    pub fmt: ToolchainStepOutcome,
    pub clippy: ToolchainStepOutcome,
    pub deny: ToolchainStepOutcome,
    pub audit: ToolchainStepOutcome,
}

impl ToolchainOutcome {
    /// Typed terminal verdict for the composed toolchain.
    #[must_use]
    pub const fn verdict(&self) -> DogfoodGateVerdict {
        if matches!(self.fmt, ToolchainStepOutcome::Failed { .. })
            || matches!(self.clippy, ToolchainStepOutcome::Failed { .. })
            || matches!(self.deny, ToolchainStepOutcome::Failed { .. })
            || matches!(self.audit, ToolchainStepOutcome::Failed { .. })
        {
            DogfoodGateVerdict::Fail
        } else {
            DogfoodGateVerdict::Pass
        }
    }
}

/// Terminal result of the composed dogfood gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for DogfoodGateVerdict."]
pub enum DogfoodGateVerdict {
    Pass,
    Fail,
}

impl DogfoodGateVerdict {
    /// Stable lowercase token used by the proof manifest.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
        }
    }
}

impl std::fmt::Display for DogfoodGateVerdict {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl serde::Serialize for DogfoodGateVerdict {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for DogfoodGateVerdict {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match <String as serde::Deserialize>::deserialize(deserializer)?.as_str() {
            "pass" => Ok(Self::Pass),
            "fail" => Ok(Self::Fail),
            _ => Err(serde::de::Error::custom(
                "dogfood gate verdict must be `pass` or `fail`",
            )),
        }
    }
}

/// Standing of the literal-scan observation against its committed ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for LiteralFloorCheck."]
pub enum LiteralFloorCheck {
    WithinCeiling,
    ExceedsCeiling,
}

impl std::fmt::Display for LiteralFloorCheck {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WithinCeiling => formatter.write_str("within ceiling"),
            Self::ExceedsCeiling => formatter.write_str("exceeds ceiling"),
        }
    }
}

/// Stable family key persisted in the dogfood proof manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for DogfoodFamily."]
pub enum DogfoodFamily {
    RustRulesNewViolations,
    RustRulesBaselinedDebt,
    LiteralScanHardFindings,
    LiteralScanHardFindingsCeiling,
    LiteralScanRisks,
    PlanStructure,
}

impl DogfoodFamily {
    /// Stable manifest token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RustRulesNewViolations => "rust-rules-new-violations",
            Self::RustRulesBaselinedDebt => "rust-rules-baselined-debt",
            Self::LiteralScanHardFindings => "literal-scan-hard-findings",
            Self::LiteralScanHardFindingsCeiling => "literal-scan-hard-findings-ceiling",
            Self::LiteralScanRisks => "literal-scan-risks",
            Self::PlanStructure => "plan-structure",
        }
    }
}

impl serde::Serialize for DogfoodFamily {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for DogfoodFamily {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match <String as serde::Deserialize>::deserialize(deserializer)?.as_str() {
            "rust-rules-new-violations" => Ok(Self::RustRulesNewViolations),
            "rust-rules-baselined-debt" => Ok(Self::RustRulesBaselinedDebt),
            "literal-scan-hard-findings" => Ok(Self::LiteralScanHardFindings),
            "literal-scan-hard-findings-ceiling" => Ok(Self::LiteralScanHardFindingsCeiling),
            "literal-scan-risks" => Ok(Self::LiteralScanRisks),
            "plan-structure" => Ok(Self::PlanStructure),
            _ => Err(serde::de::Error::custom("unknown dogfood manifest family")),
        }
    }
}
