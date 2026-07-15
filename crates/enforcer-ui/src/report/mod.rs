//! g02 — scan report view: renders an `enforcer-domain::findings::Report`
//! (via arc-24's [`crate::payload::render_report`]) into a violation-
//! matrix payload, mounted into g01's `"report"` view slug.
//!
//! # Charter
//!
//! This module never re-runs the scanner (f01/arc-15 owns producing the
//! `Report`; the caller loads it from `.enforce/` and hands it here) and
//! never re-derives the row shape arc-24 already built —
//! [`crate::payload::render_report`] is the single source of the base
//! `UiFindingRow` fields (`ruleId`/severity/title/detail/file/line/
//! snippet). This module's job is the ENRICHMENT + GROUPING layer on top:
//! joining each row against the typed [`enforcer_rules::registry::RuleRegistry`]
//! to attach the rule's `tier` and `docAnchor` (the WHY), plus a
//! `crateName` derived from the row's own file path, then bucketing the
//! enriched rows by severity / tier / file / crate so the frontend never
//! groups client-side.
//!
//! Row-level ACTIONS (waive, etc.) are g03's scope: [`ReportRow`]
//! deliberately carries no action field so g03 can attach its own
//! action payload alongside `ruleId`+`file`+`line` (the natural join key)
//! without this module changing shape underneath it.
//!
//! # Silent-mode (f04 seam)
//!
//! Mirrors [`crate::explorer::RunMode`]'s pattern exactly (same seam,
//! `enforcer-core`'s formal run-context gate has not landed): every entry
//! point takes an explicit [`RunMode`], and [`RunMode::Silent`] returns
//! the empty [`ReportViewPayload`] before touching the registry or doing
//! any grouping work — mechanically silent-safe by construction, no UI
//! emitted during inline agent runs.

use std::collections::BTreeMap;

use enforcer_domain::findings::Report;
use enforcer_rules::registry::RuleRegistry;

use crate::payload::{render_report, UiFindingRow};

/// Whether the caller is a human-invoked UI surface or a silent inline
/// agent run. Mirrors [`crate::explorer::RunMode`] for the same f04 seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    /// Human-invoked: render normally.
    Human,
    /// Inline agent run: no report render, no UI output.
    Silent,
}

/// One violation-matrix row: the base [`UiFindingRow`] fields plus the
/// enrichment this module adds (`tier`, `docAnchor`, `crateName`). No
/// action field on purpose — g03 attaches actions alongside the
/// `ruleId`/`file`/`line` join key rather than this module carrying one.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ReportRow {
    /// Rule that fired, as its wire string (e.g. `"RR-6.1"`).
    pub rule_id: String,
    /// Severity, lowercase wire form (`"error"`/`"warning"`).
    pub severity: String,
    /// Short human title of the rule.
    pub title: String,
    /// Occurrence-specific detail.
    pub detail: String,
    /// Repo-relative file the finding points at.
    pub file: String,
    /// 1-based line number.
    pub line: u32,
    /// Optional offending source excerpt (already redacted upstream).
    pub snippet: Option<String>,
    /// Mechanical-enforcement tier, wire string (`"T1"`/`"T2"`/`"T3"`),
    /// resolved from the `enforcer-rules` record. Empty when the rule id
    /// is not found in the registry (never fabricated).
    pub tier: String,
    /// Repo-relative doc anchor a human can open for WHY this rule
    /// exists, resolved from the `enforcer-rules` record. Empty when the
    /// rule id is not found in the registry.
    pub doc_anchor: String,
    /// Owning crate name, derived from `file` (`crates/<name>/...` ->
    /// `<name>`), or the file's parent directory when it does not follow
    /// that layout. Never empty for a non-empty `file`.
    pub crate_name: String,
}

/// A single named group in the violation matrix: a grouping key value
/// (e.g. `"error"` for a severity group, `"T1"` for a tier group) plus
/// the row indices (into [`ReportViewPayload::rows`]) that belong to it.
/// Indices, not cloned rows, so each row is stored exactly once.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct RowGroup {
    /// The grouping key value, e.g. `"error"`, `"T1"`, a file path, or a
    /// crate name.
    pub key: String,
    /// Indices into [`ReportViewPayload::rows`] belonging to this group,
    /// input order preserved.
    pub row_indices: Vec<u32>,
}

/// The four grouping axes the requirement checklist names, each a list of
/// [`RowGroup`]s covering every row exactly once (a row missing its
/// `tier`/`docAnchor` still lands in a group -- keyed on the empty string
/// -- rather than being dropped, so an incomplete rule is visible, not
/// silently excluded).
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ReportGroups {
    pub by_severity: Vec<RowGroup>,
    pub by_tier: Vec<RowGroup>,
    pub by_file: Vec<RowGroup>,
    pub by_crate: Vec<RowGroup>,
}

/// The full report-view payload: the base [`crate::payload::UiReportPayload`]
/// summary fields (`ok`/`scope`/`totalCount`) plus the flattened,
/// enriched, grouped violation matrix. `rows` is the single flat list
/// every group indexes into -- violations, warnings, and waived findings
/// all appear here (each row's `severity`/membership in `waived` is
/// still recoverable from the base fields if a caller needs the split;
/// the matrix itself groups the union, per the requirement checklist).
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ReportViewPayload {
    /// True when no blocking violations were found.
    pub ok: bool,
    /// What the run covered, as its wire string (e.g. `"workspace"`).
    pub scope: String,
    /// Total finding count across violations + warnings + waived.
    pub total_count: u32,
    /// Every row (violations, then warnings, then waived), enriched.
    pub rows: Vec<ReportRow>,
    /// Rows waived, by index into `rows` (a waived row is NOT removed
    /// from `rows` -- it stays visible, per the doctrine that waivers are
    /// named/visible, never a silent suppression; see g03).
    pub waived_row_indices: Vec<u32>,
    /// The four grouping axes.
    pub groups: ReportGroups,
}

/// Derive the owning crate name from a repo-relative file path:
/// `crates/<name>/...` -> `<name>`; otherwise the path's parent directory
/// (or the whole path, if it has none), so this never returns empty for a
/// non-empty `file`.
#[must_use]
pub fn crate_name_from_file(file: &str) -> String {
    let normalized = file.replace('\\', "/");
    let mut parts = normalized.split('/');
    if parts.next() == Some("crates") {
        if let Some(name) = parts.next() {
            if !name.is_empty() {
                return name.to_owned();
            }
        }
    }
    match normalized.rsplit_once('/') {
        Some((parent, _)) if !parent.is_empty() => parent.to_owned(),
        _ => normalized,
    }
}

/// Enrich one base [`UiFindingRow`] with tier/doc-anchor (looked up in
/// `registry`) and crate name (derived from `file`).
#[must_use]
pub fn enrich_row(row: &UiFindingRow, registry: &RuleRegistry) -> ReportRow {
    let (tier, doc_anchor) = row
        .rule_id
        .parse()
        .ok()
        .and_then(|rule_id| registry.get(&rule_id))
        .map(|record| {
            let tier = serde_json::to_value(record.tier)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            (tier, record.doc_anchor.clone())
        })
        .unwrap_or_default();

    ReportRow {
        rule_id: row.rule_id.clone(),
        severity: row.severity.clone(),
        title: row.title.clone(),
        detail: row.detail.clone(),
        file: row.file.clone(),
        line: row.line,
        snippet: row.snippet.clone(),
        tier,
        doc_anchor,
        crate_name: crate_name_from_file(&row.file),
    }
}

/// Build one grouping axis: a stable (BTreeMap-ordered) list of
/// [`RowGroup`]s over `rows`, keyed by `key_fn`.
fn group_by(rows: &[ReportRow], key_fn: impl Fn(&ReportRow) -> String) -> Vec<RowGroup> {
    let mut groups: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        groups.entry(key_fn(row)).or_default().push(index as u32);
    }
    groups
        .into_iter()
        .map(|(key, row_indices)| RowGroup { key, row_indices })
        .collect()
}

/// Build all four grouping axes over `rows`.
#[must_use]
pub fn build_groups(rows: &[ReportRow]) -> ReportGroups {
    ReportGroups {
        by_severity: group_by(rows, |r| r.severity.clone()),
        by_tier: group_by(rows, |r| r.tier.clone()),
        by_file: group_by(rows, |r| r.file.clone()),
        by_crate: group_by(rows, |r| r.crate_name.clone()),
    }
}

/// Render a [`Report`] into the full [`ReportViewPayload`]: base fields
/// via [`render_report`], every row enriched via `registry`, four
/// grouping axes built over the flattened row list. Honors [`RunMode`]:
/// [`RunMode::Silent`] returns the empty payload before touching the
/// registry or the report at all -- no UI emitted during inline agent
/// runs (mirrors [`crate::explorer::render_explorer`]'s contract).
#[must_use]
pub fn render_report_view(
    mode: RunMode,
    report: &Report,
    registry: &RuleRegistry,
) -> ReportViewPayload {
    if mode == RunMode::Silent {
        return ReportViewPayload::default();
    }

    let base = render_report(report);
    let waived_count = base.waived.len();

    let mut rows: Vec<ReportRow> = Vec::with_capacity(base.total_count as usize);
    rows.extend(base.violations.iter().map(|r| enrich_row(r, registry)));
    rows.extend(base.warnings.iter().map(|r| enrich_row(r, registry)));
    let waived_start = rows.len() as u32;
    rows.extend(base.waived.iter().map(|r| enrich_row(r, registry)));
    let waived_row_indices: Vec<u32> = (waived_start..waived_start + waived_count as u32).collect();

    let groups = build_groups(&rows);

    ReportViewPayload {
        ok: base.ok,
        scope: base.scope,
        total_count: base.total_count,
        rows,
        waived_row_indices,
        groups,
    }
}

#[cfg(test)]
mod tests {
    use enforcer_domain::boundary::decode_error::DecodeError;
    use enforcer_domain::findings::{Finding, Report, ScanScope, Violation};
    use enforcer_domain::ids::RuleId;
    use enforcer_domain::paths::RelPath;
    use enforcer_domain::severity::{Severity, Tier};
    use enforcer_rules::registry::{FixtureRef, RuleRecord, RuleRegistry, ValidatorRef};

    use super::{
        build_groups, crate_name_from_file, enrich_row, render_report_view, ReportRow, RowGroup,
        RunMode,
    };

    fn rule_record(rule_id: &str, tier: Tier, doc_anchor: &str) -> Result<RuleRecord, DecodeError> {
        Ok(RuleRecord {
            rule_id: rule_id.parse()?,
            version: 1,
            title: "No raw string types".to_owned(),
            tier,
            validator: ValidatorRef {
                crate_name: "enforcer-lang-rust".to_owned(),
                path: "no_reexports::NoReexportsValidator".to_owned(),
            },
            fixtures: FixtureRef {
                fail: "crates/x/fixtures/sample/fail.rs".to_owned(),
                pass: "crates/x/fixtures/sample/pass.rs".to_owned(),
            },
            doc_anchor: doc_anchor.to_owned(),
            tags: vec!["rust".to_owned()],
            params: serde_json::Value::Null,
        })
    }

    fn sample_finding(
        rule_id: &str,
        severity: Severity,
        file: &str,
    ) -> Result<Finding, DecodeError> {
        Ok(Finding {
            rule_id: rule_id.parse::<RuleId>()?,
            severity,
            title: "No raw string types".to_owned(),
            detail: "Raw string in signature.".to_owned(),
            file: file.parse::<RelPath>()?,
            line: 12,
            snippet: None,
        })
    }

    /// `crate_name_from_file`: standard `crates/<name>/...` layout.
    #[test]
    fn crate_name_from_standard_layout() {
        assert_eq!(
            crate_name_from_file("crates/enforcer-ui/src/report/mod.rs"),
            "enforcer-ui"
        );
    }

    /// `crate_name_from_file`: non-`crates/` path falls back to parent
    /// directory, never empty.
    #[test]
    fn crate_name_from_non_standard_layout_uses_parent_dir() {
        assert_eq!(
            crate_name_from_file("skills/enforcer/SKILL.md"),
            "skills/enforcer"
        );
        assert_eq!(crate_name_from_file("README.md"), "README.md");
    }

    /// PASS fixture `report-matrix-render`: a mixed-severity fixture
    /// `Report` renders into a matrix where every row exposes `ruleId` +
    /// why-anchor (`docAnchor`) + location (`file`:`line`), and grouping
    /// keys resolve for all four axes.
    #[test]
    fn report_matrix_render_groups_and_enriches_every_row() -> Result<(), Box<dyn std::error::Error>>
    {
        let registry = RuleRegistry::from_records(vec![
            rule_record("RR-6.1", Tier::T1, "docs/rules/RR-6.md#RR-6.1")?,
            rule_record("RR-7.1", Tier::T2, "docs/rules/RR-7.md#RR-7.1")?,
        ])?;

        let violation = Violation::try_from(sample_finding(
            "RR-6.1",
            Severity::Error,
            "crates/enforcer-ui/src/lib.rs",
        )?)?;
        let warning = sample_finding(
            "RR-7.1",
            Severity::Warning,
            "crates/enforcer-core/src/lib.rs",
        )?;
        let waived = sample_finding("RR-6.1", Severity::Warning, "crates/enforcer-ui/src/lib.rs")?;

        let report = Report {
            ok: false,
            scope: ScanScope::Workspace,
            violations: vec![violation],
            warnings: vec![warning],
            waived: vec![waived],
            findings: vec![],
        };

        let payload = render_report_view(RunMode::Human, &report, &registry);

        assert_eq!(payload.rows.len(), 3);
        assert_eq!(payload.total_count, 3);
        assert!(!payload.ok);
        assert_eq!(payload.scope, "workspace");

        for row in &payload.rows {
            assert!(!row.rule_id.is_empty(), "row missing ruleId");
            assert!(
                !row.doc_anchor.is_empty(),
                "row missing docAnchor/why-anchor"
            );
            assert!(!row.file.is_empty(), "row missing location file");
        }

        assert_eq!(payload.rows[0].tier, "T1");
        assert_eq!(payload.rows[0].doc_anchor, "docs/rules/RR-6.md#RR-6.1");
        assert_eq!(payload.rows[0].crate_name, "enforcer-ui");
        assert_eq!(payload.rows[1].tier, "T2");
        assert_eq!(payload.rows[1].crate_name, "enforcer-core");

        // Waived row stays visible in `rows`, indexed (not silently
        // dropped from the matrix).
        assert_eq!(payload.waived_row_indices, vec![2]);

        // Grouping keys resolve for all four axes and cover every row.
        let total_in_groups =
            |groups: &[RowGroup]| groups.iter().map(|g| g.row_indices.len()).sum::<usize>();
        assert_eq!(total_in_groups(&payload.groups.by_severity), 3);
        assert_eq!(total_in_groups(&payload.groups.by_tier), 3);
        assert_eq!(total_in_groups(&payload.groups.by_file), 3);
        assert_eq!(total_in_groups(&payload.groups.by_crate), 3);

        assert!(payload.groups.by_severity.iter().any(|g| g.key == "error"));
        assert!(payload
            .groups
            .by_severity
            .iter()
            .any(|g| g.key == "warning"));
        assert!(payload.groups.by_tier.iter().any(|g| g.key == "T1"));
        assert!(payload.groups.by_tier.iter().any(|g| g.key == "T2"));
        assert!(payload
            .groups
            .by_crate
            .iter()
            .any(|g| g.key == "enforcer-ui"));
        assert!(payload
            .groups
            .by_crate
            .iter()
            .any(|g| g.key == "enforcer-core"));
        Ok(())
    }

    /// A row whose rule id is not present in the registry is enriched
    /// with empty tier/doc-anchor (never fabricated) but still grouped
    /// (under the empty-string key), never dropped.
    #[test]
    fn row_missing_from_registry_gets_empty_enrichment_not_dropped(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let registry = RuleRegistry::from_records(vec![])?;
        let base_row = crate::payload::UiFindingRow {
            rule_id: "RR-9.9".to_owned(),
            severity: "error".to_owned(),
            title: "t".to_owned(),
            detail: "d".to_owned(),
            file: "crates/x/src/lib.rs".to_owned(),
            line: 1,
            snippet: None,
        };
        let enriched = enrich_row(&base_row, &registry);
        assert!(enriched.tier.is_empty());
        assert!(enriched.doc_anchor.is_empty());
        assert_eq!(enriched.crate_name, "x");

        let groups = build_groups(&[enriched]);
        assert!(groups.by_tier.iter().any(|g| g.key.is_empty()));
        Ok(())
    }

    /// PASS fixture: an empty/clean `Report` renders an honest-empty
    /// matrix -- zero rows, zero groups, `ok: true` -- not a
    /// special-cased/fabricated variant.
    #[test]
    fn empty_report_renders_honest_empty_matrix() -> Result<(), Box<dyn std::error::Error>> {
        let registry = RuleRegistry::from_records(vec![])?;
        let report = Report {
            ok: true,
            scope: ScanScope::Workspace,
            violations: vec![],
            warnings: vec![],
            waived: vec![],
            findings: vec![],
        };
        let payload = render_report_view(RunMode::Human, &report, &registry);
        assert!(payload.ok);
        assert!(payload.rows.is_empty());
        assert_eq!(payload.total_count, 0);
        assert!(payload.waived_row_indices.is_empty());
        assert!(payload.groups.by_severity.is_empty());
        assert!(payload.groups.by_tier.is_empty());
        assert!(payload.groups.by_file.is_empty());
        assert!(payload.groups.by_crate.is_empty());
        Ok(())
    }

    /// FAIL fixture `report-silent-mode-suppressed`: silent mode (f04
    /// active) renders zero UI output -- the empty payload -- even for a
    /// non-empty `Report`, and never touches the registry/report data
    /// (verified structurally: the empty payload is indistinguishable
    /// from "nothing happened").
    #[test]
    fn silent_mode_suppresses_all_output() -> Result<(), Box<dyn std::error::Error>> {
        let registry = RuleRegistry::from_records(vec![rule_record(
            "RR-6.1",
            Tier::T1,
            "docs/rules/RR-6.md#RR-6.1",
        )?])?;
        let violation = Violation::try_from(sample_finding(
            "RR-6.1",
            Severity::Error,
            "crates/enforcer-ui/src/lib.rs",
        )?)?;
        let report = Report {
            ok: false,
            scope: ScanScope::Workspace,
            violations: vec![violation],
            warnings: vec![],
            waived: vec![],
            findings: vec![],
        };

        let payload = render_report_view(RunMode::Silent, &report, &registry);
        assert!(payload.rows.is_empty());
        assert_eq!(payload.total_count, 0);
        assert!(payload.groups.by_severity.is_empty());
        Ok(())
    }

    /// `report-view-contract`: no external asset is referenced by any
    /// row -- every row's fields are plain data (strings/numbers), never
    /// a URL/asset reference, so the served-HTML fallback can render
    /// this payload with zero external fetches.
    #[test]
    fn rows_carry_no_external_asset_references() -> Result<(), Box<dyn std::error::Error>> {
        let registry = RuleRegistry::from_records(vec![rule_record(
            "RR-6.1",
            Tier::T1,
            "docs/rules/RR-6.md#RR-6.1",
        )?])?;
        let row = enrich_row(
            &crate::payload::UiFindingRow {
                rule_id: "RR-6.1".to_owned(),
                severity: "error".to_owned(),
                title: "t".to_owned(),
                detail: "d".to_owned(),
                file: "crates/enforcer-ui/src/lib.rs".to_owned(),
                line: 1,
                snippet: None,
            },
            &registry,
        );
        let ReportRow {
            rule_id,
            severity,
            title,
            detail,
            file,
            snippet,
            doc_anchor,
            crate_name,
            ..
        } = &row;
        for field in [
            rule_id, severity, title, detail, file, doc_anchor, crate_name,
        ] {
            assert!(!field.contains("http://") && !field.contains("https://"));
        }
        assert!(snippet.is_none());
        Ok(())
    }

    /// `report-view-contract`: mounts into g01's registry -- the
    /// `"report"` slug is present in [`crate::serve::VIEW_MOUNTS`].
    #[test]
    fn mounts_into_g01_view_registry() {
        assert!(crate::serve::VIEW_MOUNTS
            .iter()
            .any(|mount| mount.slug == "report"));
    }
}
