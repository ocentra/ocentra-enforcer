//! The Rust->TS type-generation pipeline (arc-24): `ts_rs::TS::export_
//! all_to` over every UI-facing type, driving both the committed
//! `frontend/src/bindings/*.ts` and the fail-closed drift test in
//! `tests/ts_drift.rs`.
//!
//! Frontend types are DERIVED, never hand-written: `enforcer-domain`'s
//! `Report`/`Finding`/`Violation`/`ScanScope`/`Severity` (arc-02) and this
//! crate's own [`crate::payload::UiReportPayload`]/[`crate::payload::
//! UiFindingRow`] all derive `ts_rs::TS`; [`export_all`] walks their full
//! dependency graph (branded newtypes like `RuleId`/`RelPath` included)
//! and writes one `.ts` file per type into the given directory. camelCase
//! wire casing throughout (locked decision).

use std::path::Path;

use enforcer_domain::findings::Report;
use ts_rs::{ExportError, TS};

use crate::payload::UiReportPayload;

/// Export every UI-facing type (and its full dependency graph) into
/// `out_dir`. Called by both the `enforcer-ui-export-ts` bin (writes the
/// committed `frontend/src/bindings/`) and the drift test (writes into a
/// scratch `tempfile::TempDir` to byte-compare against committed output).
///
/// `Report::export_all_to` alone pulls in every `enforcer-domain`
/// dependency the frontend needs (`Finding`, `Violation`, `ScanScope`,
/// `Severity`, `RuleId`, `RelPath`, ...); [`UiReportPayload`]'s own call
/// additionally emits the UI-specific row shape (which is not one of
/// `Report`'s dependencies, since [`crate::payload::render_report`]
/// converts rather than reuses the domain type directly).
pub fn export_all(out_dir: &Path) -> Result<(), ExportError> {
    Report::export_all_to(out_dir)?;
    UiReportPayload::export_all_to(out_dir)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::export_all;

    /// PASS fixture: exporting into a fresh scratch directory succeeds
    /// and produces at least the `Report` and `UiReportPayload` bindings.
    #[test]
    fn export_all_writes_expected_bindings() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        export_all(dir.path())?;
        assert!(dir.path().join("Report.ts").is_file());
        assert!(dir.path().join("UiReportPayload.ts").is_file());
        Ok(())
    }
}
