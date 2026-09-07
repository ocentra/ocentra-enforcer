//! Typed wire records for advisory ARCH-1.16 UI/logic coupling analysis.

use crate::boundary::decode_error::DecodeError;
use serde::ser::{SerializeStruct, Serializer};
use serde::Serialize;

/// BRAND-INVARIANT: nonblank text retained as a validated, owned wire value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiLogicText(String);

impl UiLogicText {
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        if value.trim().is_empty() {
            return Err(DecodeError::new("uiLogicText", "must not be empty"));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for UiLogicText {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

/// BRAND-INVARIANT: a non-negative count that has passed checked conversion to u64.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiLogicCount(u64);

impl UiLogicCount {
    pub fn try_new(value: u64) -> Result<Self, DecodeError> {
        Ok(Self(value))
    }

    pub fn try_from_len(value: usize) -> Result<Self, DecodeError> {
        u64::try_from(value)
            .map_err(|_error| DecodeError::new("uiLogicCount", "does not fit the wire count range"))
            .and_then(Self::try_new)
    }
}

impl Serialize for UiLogicCount {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiLogicEvidenceState {
    Present,
    Absent,
}

impl UiLogicEvidenceState {
    #[must_use]
    pub const fn from_bool(value: bool) -> Self {
        if value {
            Self::Present
        } else {
            Self::Absent
        }
    }
}

impl Serialize for UiLogicEvidenceState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(matches!(self, Self::Present))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiLogicFindingKind {
    BusinessLogicImport,
    EventSourceImport,
}

impl Serialize for UiLogicFindingKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::BusinessLogicImport => "business-logic-import",
            Self::EventSourceImport => "event-source-import",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiLogicFindingSeverity {
    Hard,
    Info,
}

impl Serialize for UiLogicFindingSeverity {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::Hard => "hard",
            Self::Info => "info",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiLogicRuleInput {
    id: UiLogicText,
    title: UiLogicText,
    doc: UiLogicText,
    aka: UiLogicText,
    why: UiLogicText,
}

impl UiLogicRuleInput {
    #[must_use]
    pub fn new(
        id: UiLogicText,
        title: UiLogicText,
        doc: UiLogicText,
        aka: UiLogicText,
        why: UiLogicText,
    ) -> Self {
        Self {
            id,
            title,
            doc,
            aka,
            why,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiLogicRule {
    id: UiLogicText,
    title: UiLogicText,
    doc: UiLogicText,
    aka: UiLogicText,
    why: UiLogicText,
}

impl UiLogicRule {
    #[must_use]
    pub fn from_input(input: UiLogicRuleInput) -> Self {
        Self {
            id: input.id,
            title: input.title,
            doc: input.doc,
            aka: input.aka,
            why: input.why,
        }
    }
}

impl Serialize for UiLogicRule {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("UiLogicRule", 5)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("title", &self.title)?;
        state.serialize_field("doc", &self.doc)?;
        state.serialize_field("aka", &self.aka)?;
        state.serialize_field("why", &self.why)?;
        state.end()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiLogicFindingInput {
    file: UiLogicText,
    source: UiLogicText,
    binding: UiLogicText,
    kind: UiLogicFindingKind,
    severity: UiLogicFindingSeverity,
    data_fetch: UiLogicEvidenceState,
}

impl UiLogicFindingInput {
    #[must_use]
    pub fn new(file: UiLogicText, source: UiLogicText, binding: UiLogicText) -> Self {
        Self {
            file,
            source,
            binding,
            kind: UiLogicFindingKind::BusinessLogicImport,
            severity: UiLogicFindingSeverity::Info,
            data_fetch: UiLogicEvidenceState::Absent,
        }
    }

    #[must_use]
    pub const fn with_kind(mut self, kind: UiLogicFindingKind) -> Self {
        self.kind = kind;
        self
    }

    #[must_use]
    pub const fn with_severity(mut self, severity: UiLogicFindingSeverity) -> Self {
        self.severity = severity;
        self
    }

    #[must_use]
    pub const fn with_data_fetch(mut self, data_fetch: UiLogicEvidenceState) -> Self {
        self.data_fetch = data_fetch;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiLogicFinding {
    file: UiLogicText,
    kind: UiLogicFindingKind,
    severity: UiLogicFindingSeverity,
    source: UiLogicText,
    binding: UiLogicText,
    has_data_fetch_primitive: UiLogicEvidenceState,
}

impl UiLogicFinding {
    #[must_use]
    pub fn from_input(input: UiLogicFindingInput) -> Self {
        Self {
            file: input.file,
            source: input.source,
            binding: input.binding,
            kind: input.kind,
            severity: input.severity,
            has_data_fetch_primitive: input.data_fetch,
        }
    }
    #[must_use]
    pub const fn is_hard(&self) -> bool {
        matches!(self.severity, UiLogicFindingSeverity::Hard)
    }
    #[must_use]
    pub fn file(&self) -> &UiLogicText {
        &self.file
    }
}

impl Serialize for UiLogicFinding {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("UiLogicFinding", 6)?;
        state.serialize_field("file", &self.file)?;
        state.serialize_field("kind", &self.kind)?;
        state.serialize_field("severity", &self.severity)?;
        state.serialize_field("source", &self.source)?;
        state.serialize_field("binding", &self.binding)?;
        state.serialize_field("hasDataFetchPrimitive", &self.has_data_fetch_primitive)?;
        state.end()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiLogicSummary {
    total_findings: UiLogicCount,
    hard_findings: UiLogicCount,
    info_findings: UiLogicCount,
    files_with_hard_findings: UiLogicCount,
}

impl UiLogicSummary {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            total_findings: UiLogicCount(0),
            hard_findings: UiLogicCount(0),
            info_findings: UiLogicCount(0),
            files_with_hard_findings: UiLogicCount(0),
        }
    }

    #[must_use]
    pub fn new(
        total_findings: UiLogicCount,
        hard_findings: UiLogicCount,
        info_findings: UiLogicCount,
        files_with_hard_findings: UiLogicCount,
    ) -> Self {
        Self {
            total_findings,
            hard_findings,
            info_findings,
            files_with_hard_findings,
        }
    }
}

impl Serialize for UiLogicSummary {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("UiLogicSummary", 4)?;
        state.serialize_field("totalFindings", &self.total_findings)?;
        state.serialize_field("hardFindings", &self.hard_findings)?;
        state.serialize_field("infoFindings", &self.info_findings)?;
        state.serialize_field("filesWithHardFindings", &self.files_with_hard_findings)?;
        state.end()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiLogicCouplingReportInput {
    root: UiLogicText,
    rule: UiLogicRule,
    caveat: UiLogicText,
    findings: Vec<UiLogicFinding>,
    summary: UiLogicSummary,
    hard: Vec<UiLogicFinding>,
    info: Vec<UiLogicFinding>,
}

impl UiLogicCouplingReportInput {
    #[must_use]
    pub fn new(root: UiLogicText, rule: UiLogicRule, caveat: UiLogicText) -> Self {
        Self {
            root,
            rule,
            caveat,
            findings: Vec::new(),
            summary: UiLogicSummary::empty(),
            hard: Vec::new(),
            info: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_findings(mut self, findings: Vec<UiLogicFinding>) -> Self {
        self.findings = findings;
        self
    }

    #[must_use]
    pub fn with_summary(mut self, summary: UiLogicSummary) -> Self {
        self.summary = summary;
        self
    }

    #[must_use]
    pub fn with_hard(mut self, hard: Vec<UiLogicFinding>) -> Self {
        self.hard = hard;
        self
    }

    #[must_use]
    pub fn with_info(mut self, info: Vec<UiLogicFinding>) -> Self {
        self.info = info;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiLogicCouplingReport {
    root: UiLogicText,
    rule: UiLogicRule,
    caveat: UiLogicText,
    findings: Vec<UiLogicFinding>,
    summary: UiLogicSummary,
    hard: Vec<UiLogicFinding>,
    info: Vec<UiLogicFinding>,
}

impl UiLogicCouplingReport {
    #[must_use]
    pub fn from_input(input: UiLogicCouplingReportInput) -> Self {
        Self {
            root: input.root,
            rule: input.rule,
            caveat: input.caveat,
            findings: input.findings,
            summary: input.summary,
            hard: input.hard,
            info: input.info,
        }
    }
}

impl Serialize for UiLogicCouplingReport {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("UiLogicCouplingReport", 7)?;
        state.serialize_field("root", &self.root)?;
        state.serialize_field("rule", &self.rule)?;
        state.serialize_field("caveat", &self.caveat)?;
        state.serialize_field("findings", &self.findings)?;
        state.serialize_field("summary", &self.summary)?;
        state.serialize_field("hard", &self.hard)?;
        state.serialize_field("info", &self.info)?;
        state.end()
    }
}
