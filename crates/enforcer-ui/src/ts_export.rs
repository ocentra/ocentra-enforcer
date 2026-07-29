//! The Rust->TS type-generation pipeline (arc-24): `ts_rs::TS::export_
//! all_to` over every UI-facing type, driving both the committed
//! `frontend/src/bindings/*.ts` and the fail-closed drift test in
//! `tests/ts_drift.rs`.
//!
//! Frontend types are DERIVED, never hand-written: `enforcer-domain`'s
//! `Report`/`Finding`/`Violation`/`ScanScope`/`Severity` (arc-02) and this
//! crate's own [`crate::payload::UiReportResponse`]/[`crate::payload::
//! UiFindingRowResponse`] all derive `ts_rs::TS`; [`export_all`] walks their full
//! dependency graph (branded newtypes like `RuleId`/`RelPath` included)
//! and writes one `.ts` file per type into the given directory. camelCase
//! wire casing throughout (locked decision).

use std::{fs, path::Path};

use enforcer_domain::findings::Report;
use ts_rs::{ExportError, TS};

use crate::payload::UiReportResponse;

/// Export every UI-facing type (and its full dependency graph) into
/// `out_dir`. Called by both the `enforcer-ui-export-ts` bin (writes the
/// committed `frontend/src/bindings/`) and the drift test (writes into a
/// scratch `tempfile::TempDir` to byte-compare against committed output).
///
/// `Report::export_all_to` alone pulls in every `enforcer-domain`
/// dependency the frontend needs (`Finding`, `Violation`, `ScanScope`,
/// `Severity`, `RuleId`, `RelPath`, ...); [`UiReportResponse`]'s own call
/// additionally emits the UI-specific row shape (which is not one of
/// `Report`'s dependencies, since [`crate::payload::render_report`]
/// converts rather than reuses the domain type directly).
#[derive(Debug, thiserror::Error)]
pub enum TsExportError {
    #[error("TypeScript export failed: {0}")]
    Export(#[from] ExportError),
    #[error("TypeScript binding normalization failed: {0}")]
    Io(#[from] std::io::Error),
}

pub fn export_all(out_dir: &Path) -> Result<(), TsExportError> {
    Report::export_all_to(out_dir)?;
    UiReportResponse::export_all_to(out_dir)?;
    normalize_bindings(out_dir)?;
    Ok(())
}

fn normalize_bindings(out_dir: &Path) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(out_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("ts") {
            continue;
        }
        let source = fs::read_to_string(&path)?;
        let normalized = source
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        fs::write(path, normalized)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::export_all;

    /// PASS fixture: exporting into a fresh scratch directory succeeds
    /// and produces at least the `Report` and `UiReportResponse` bindings.
    #[test]
    fn export_all_writes_expected_bindings() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        export_all(dir.path())?;
        assert!(dir.path().join("Report.ts").is_file());
        assert!(dir.path().join("UiReportResponse.ts").is_file());
        for entry in std::fs::read_dir(dir.path())? {
            let path = entry?.path();
            if path.extension().and_then(|value| value.to_str()) == Some("ts") {
                let source = std::fs::read_to_string(path)?;
                assert!(source.lines().all(|line| line.trim_end() == line));
            }
        }
        Ok(())
    }
}
