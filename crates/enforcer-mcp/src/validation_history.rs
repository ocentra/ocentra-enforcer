//! Process-local compatibility history for native MCP validation results.
//!
//! The router decodes the frozen MJS JSON report at the boundary and gives this
//! module a typed snapshot. This keeps process-local state independent of the
//! transport representation while preserving the frozen wire envelope.

use std::collections::{BTreeMap, VecDeque};

use enforcer_domain::boundary::validation::McpReportLabelText;
use enforcer_domain::paths::RepoRoot;

const HISTORY_LIMIT: usize = 20;

/// A validation command whose wire spelling is fixed by the MCP contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationKind {
    Scan,
    Check,
}

/// A string decoded at the MCP boundary for opaque report labels.
/// BRAND-INVARIANT: always owned boundary text; it is never accepted from a
/// domain function signature and is emitted only by the router's JSON encoder.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReportLabel(String);

impl ReportLabel {
    /// Retain label text already validated by the MCP boundary adapter.
    pub(crate) fn try_new(value: McpReportLabelText) -> Self {
        Self(value.into_inner())
    }
}

impl From<&ReportLabel> for String {
    fn from(value: &ReportLabel) -> Self {
        // CLONE-JUSTIFICATION: the router owns a fresh JSON string at the transport boundary.
        value.0.clone()
    }
}

/// A timestamp selected by the router after it has crossed the JSON boundary.
/// BRAND-INVARIANT: this is either the platform UTC representation or the
/// frozen epoch fallback, and is only rendered by the boundary encoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationTimestamp(String);

impl ValidationTimestamp {
    pub(crate) fn parse(value: ReportLabel) -> Self {
        Self(value.0)
    }
}
impl From<&ValidationTimestamp> for String {
    fn from(value: &ValidationTimestamp) -> Self {
        // CLONE-JUSTIFICATION: the router owns a fresh JSON string at the transport boundary.
        value.0.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct FindingCount(pub(crate) usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SeverityCount(pub(crate) u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValidationOutcome {
    Passed,
    Failed,
    Unknown,
}

/// Case-folded index key for a validated repository root.
/// BRAND-INVARIANT: generated exclusively from a `RepoRoot`; the text is its
/// Unicode lowercase form and is used solely for frozen-MJS-compatible lookup.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FoldedRoot(String);

impl From<&RepoRoot> for FoldedRoot {
    fn from(root: &RepoRoot) -> Self {
        Self(root.as_str().to_lowercase())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ValidationCounts {
    pub(crate) findings: FindingCount,
    pub(crate) violations: FindingCount,
    pub(crate) warnings: FindingCount,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CompactScope {
    pub(crate) mode: Option<ReportLabel>,
    pub(crate) crate_name: Option<ReportLabel>,
    pub(crate) base: Option<ReportLabel>,
    pub(crate) head: Option<ReportLabel>,
    pub(crate) file_count: Option<FindingCount>,
    pub(crate) sample_files: Vec<ReportLabel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidationSummary {
    pub(crate) kind: ValidationKind,
    pub(crate) command: Option<ReportLabel>,
    pub(crate) check: Option<ReportLabel>,
    pub(crate) outcome: ValidationOutcome,
    pub(crate) root: RepoRoot,
    pub(crate) profile_name: Option<ReportLabel>,
    pub(crate) at: ValidationTimestamp,
    pub(crate) by_severity: BTreeMap<ReportLabel, SeverityCount>,
    pub(crate) counts: ValidationCounts,
    pub(crate) rule_ids: Vec<ReportLabel>,
    pub(crate) docs: Vec<ReportLabel>,
    pub(crate) scope: Option<CompactScope>,
}

#[derive(Debug, Default)]
pub struct ValidationHistory {
    by_root: BTreeMap<FoldedRoot, VecDeque<ValidationSummary>>,
}

impl ValidationHistory {
    pub(crate) fn record(&mut self, summary: ValidationSummary) {
        let root = FoldedRoot::from(&summary.root);
        let entries = self.by_root.entry(root).or_default();
        entries.push_front(summary);
        entries.truncate(HISTORY_LIMIT);
    }

    pub(crate) fn latest(
        &self,
        root: &RepoRoot,
        filter: Option<ValidationKind>,
    ) -> Option<&ValidationSummary> {
        self.by_root
            .get(&FoldedRoot::from(root))?
            .iter()
            .find(|entry| filter.is_none_or(|expected| entry.kind == expected))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use enforcer_domain::{boundary::validation::McpReportLabelText, paths::RepoRoot};

    use super::{
        HISTORY_LIMIT, ReportLabel, ValidationCounts, ValidationHistory, ValidationKind,
        ValidationSummary, ValidationTimestamp,
    };

    fn root() -> Result<RepoRoot, enforcer_domain::boundary::decode_error::DecodeError> {
        "C:/Repo".parse()
    }

    fn summary(
        kind: ValidationKind,
    ) -> Result<ValidationSummary, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(ValidationSummary {
            kind,
            command: None,
            check: None,
            outcome: super::ValidationOutcome::Failed,
            root: root()?,
            profile_name: None,
            at: ValidationTimestamp::parse(ReportLabel::try_new(McpReportLabelText::try_new(
                "1970-01-01T00:00:00.000Z".to_owned(),
            )?)),
            by_severity: BTreeMap::new(),
            counts: ValidationCounts::default(),
            rule_ids: vec![ReportLabel::try_new(McpReportLabelText::try_new(
                "RR-TEST".to_owned(),
            )?)],
            docs: Vec::new(),
            scope: None,
        })
    }

    #[test]
    fn retains_newest_twenty_case_folded_per_root_and_filters_kind()
    -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        let mut history = ValidationHistory::default();
        history.record(summary(ValidationKind::Scan)?);
        history.record(summary(ValidationKind::Check)?);
        std::iter::repeat_with(|| summary(ValidationKind::Scan))
            .take(HISTORY_LIMIT - 2)
            .try_for_each(|entry| {
                history.record(entry?);
                Ok::<(), enforcer_domain::boundary::decode_error::DecodeError>(())
            })?;

        assert_eq!(
            history
                .latest(&root()?, Some(ValidationKind::Check))
                .map(|entry| entry.kind),
            Some(ValidationKind::Check)
        );
        assert_eq!(
            history
                .latest(&root()?, Some(ValidationKind::Scan))
                .map(|entry| entry.kind),
            Some(ValidationKind::Scan)
        );
        Ok(())
    }

    #[test]
    fn try_new_rejects_invalid_blank_and_control_character_labels()
    -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        for invalid in ["   ", "bad\nlabel"] {
            let error = McpReportLabelText::try_new(invalid.to_owned())
                .err()
                .ok_or_else(|| {
                    enforcer_domain::boundary::decode_error::DecodeError::new(
                        "test.mcpReportLabel",
                        "invalid label was accepted",
                    )
                })?;
            assert_eq!(error.path, "mcpReportLabel");
            assert!(
                matches!(
                    error.reason.as_str(),
                    "label is blank" | "label contains a control character"
                ),
                "unexpected validation reason: {}",
                error.reason
            );
        }
        Ok(())
    }

    #[test]
    fn empty_label_is_rejected_before_epoch_fallback_conversion()
    -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        let error = McpReportLabelText::try_new(String::new())
            .err()
            .ok_or_else(|| {
                enforcer_domain::boundary::decode_error::DecodeError::new(
                    "test.mcpReportLabel",
                    "empty label was accepted",
                )
            })?;
        assert_eq!(error.path, "mcpReportLabel");
        assert_eq!(error.reason, "label is blank");

        let input = McpReportLabelText::try_new("1970-01-01T00:00:00.000Z".to_owned())?;
        let timestamp = ValidationTimestamp::parse(ReportLabel::try_new(input));

        assert_eq!(
            String::from(&timestamp),
            "1970-01-01T00:00:00.000Z".to_owned()
        );
        Ok(())
    }
}
