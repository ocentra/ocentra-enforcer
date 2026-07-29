//! Finding / Violation / Report / ScanScope DTOs — the record shapes the
//! validators emit and the MCP/UI surfaces render. camelCase wire casing
//! (locked decision); `ts_rs::TS` derives feed the arc-24 Rust->TS
//! pipeline.

use crate::boundary::decode_error::DecodeError;

use crate::ids::RuleId;
use crate::paths::RelPath;
use crate::severity::Severity;
use crate::telemetry_types::SourceLine;

macro_rules! finding_text {
    ($(#[$doc:meta])* $name:ident, $field:literal) => {
        $(#[$doc])*
        // SERIALIZATION-DOC: this stable wire representation is consumed by durable adapters.
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, ts_rs::TS)]
        #[serde(transparent)]
        #[ts(type = "string")]
        pub struct $name(String);

        impl $name {
            #[doc = "The new operation for this canonical domain value."]
            pub fn new(value: String) -> Result<Self, DecodeError> {
                if value.trim().is_empty() || value.chars().any(char::is_control) {
                    return Err(DecodeError::new($field, "must be non-empty printable text"));
                }
                Ok(Self(value))
            }

            #[doc = "The as_str operation for this canonical domain value."]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = DecodeError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl std::str::FromStr for $name {
            type Err = DecodeError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
                Self::new(value.to_owned())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = <String as serde::Deserialize>::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

finding_text!(
    #[doc = "Validated short human title for a finding."]
    FindingTitle,
    "finding.title"
);
finding_text!(
    #[doc = "Validated occurrence-specific finding detail."]
    FindingDetail,
    "finding.detail"
);
finding_text!(
    #[doc = "Validated, pre-redacted offending source excerpt."]
    FindingSnippet,
    "finding.snippet"
);

/// Whether a report contains no blocking violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ts_rs::TS)]
#[ts(type = "boolean")]
#[doc = "Canonical domain representation for ReportOutcome."]
pub enum ReportOutcome {
    Clean,
    Violations,
}

impl serde::Serialize for ReportOutcome {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bool(matches!(self, Self::Clean))
    }
}

impl<'de> serde::Deserialize<'de> for ReportOutcome {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(
            if <bool as serde::Deserialize>::deserialize(deserializer)? {
                Self::Clean
            } else {
                Self::Violations
            },
        )
    }
}

/// Source line attached to a finding, including aggregate findings that have
/// no single source line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, ts_rs::TS)]
#[ts(type = "number")]
#[doc = "Canonical domain representation for FindingLine."]
pub enum FindingLine {
    Known(SourceLine),
    Unspecified,
}

impl FindingLine {
    pub const fn known(line: SourceLine) -> Self {
        Self::Known(line)
    }

    pub const fn source_line(self) -> Option<SourceLine> {
        match self {
            Self::Known(line) => Some(line),
            Self::Unspecified => None,
        }
    }
}

impl From<SourceLine> for FindingLine {
    fn from(line: SourceLine) -> Self {
        Self::Known(line)
    }
}

impl serde::Serialize for FindingLine {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Known(line) => line.serialize(serializer),
            Self::Unspecified => serializer.serialize_u32(0),
        }
    }
}

impl<'de> serde::Deserialize<'de> for FindingLine {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <u32 as serde::Deserialize>::deserialize(deserializer)?;
        if value == 0 {
            Ok(Self::Unspecified)
        } else {
            std::num::NonZeroU32::new(value)
                .map(SourceLine::try_new)
                .map(Self::Known)
                .ok_or_else(|| serde::de::Error::custom("finding line must be positive"))
        }
    }
}

/// What a validation run covered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ts_rs::TS)]
#[doc = "Canonical domain representation for ScanScope."]
pub enum ScanScope {
    /// Whole workspace.
    Workspace,
    /// An explicit file list.
    Files,
    /// One Cargo crate.
    Crate,
    /// A git diff range.
    Diff,
}

impl serde::Serialize for ScanScope {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            Self::Workspace => "workspace",
            Self::Files => "files",
            Self::Crate => "crate",
            Self::Diff => "diff",
        })
    }
}

impl<'de> serde::Deserialize<'de> for ScanScope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match <String as serde::Deserialize>::deserialize(deserializer)?.as_str() {
            "workspace" => Ok(Self::Workspace),
            "files" => Ok(Self::Files),
            "crate" => Ok(Self::Crate),
            "diff" => Ok(Self::Diff),
            _ => Err(serde::de::Error::custom("invalid scan scope")),
        }
    }
}

/// One finding produced by a rule against a file location.
// SERIALIZATION-DOC: this stable wire representation is consumed by durable adapters.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[doc = "Canonical domain representation for Finding."]
pub struct Finding {
    /// Rule that fired.
    pub rule_id: RuleId,
    /// Severity of this occurrence.
    pub severity: Severity,
    /// Short human title of the rule.
    pub title: FindingTitle,
    /// Occurrence-specific detail.
    pub detail: FindingDetail,
    /// Repo-relative file the finding points at.
    pub file: RelPath,
    /// 1-based line number.
    pub line: FindingLine,
    /// Optional offending source excerpt (already redacted upstream).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<FindingSnippet>,
}

/// A BLOCKING finding. Invariant: severity is [`Severity::Error`], enforced
/// at construction — a non-error finding cannot become a `Violation`.
// SERIALIZATION-DOC: deserialization revalidates the blocking severity through TryFrom.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(into = "Finding")]
#[doc = "Canonical domain representation for Violation."]
pub struct Violation(Finding);

impl Violation {
    /// View the underlying finding.
    pub fn finding(&self) -> &Finding {
        &self.0
    }
}

impl TryFrom<Finding> for Violation {
    type Error = DecodeError;

    /// Reject an invalid non-error finding before it can become a blocking violation.
    fn try_from(finding: Finding) -> Result<Self, DecodeError> {
        if finding.severity == Severity::Error {
            Ok(Self(finding))
        } else {
            Err(DecodeError::new(
                "violation.severity",
                "a violation must carry severity `error`",
            ))
        }
    }
}

impl From<Violation> for Finding {
    fn from(violation: Violation) -> Finding {
        violation.0
    }
}

/// The report a check/scan run returns to callers (CLI, MCP, CI).
// SERIALIZATION-DOC: this stable wire representation is consumed by CLI and MCP adapters.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[doc = "Canonical domain representation for Report."]
pub struct Report {
    /// True when no blocking violations were found.
    pub ok: ReportOutcome,
    /// What the run covered.
    pub scope: ScanScope,
    /// Blocking violations.
    pub violations: Vec<Violation>,
    /// Non-blocking warnings.
    pub warnings: Vec<Finding>,
    /// Findings suppressed by an explicit waiver.
    pub waived: Vec<Finding>,
    /// Every finding (violations + warnings + waived, denormalized for
    /// consumers that want one list).
    pub findings: Vec<Finding>,
}
