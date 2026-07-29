//! Explicit JSON wire decoding for CFML tool integrations.
//!
//! BOUNDARY-INVARIANT: untrusted CFLint JSON and subprocess output are
//! decoded here and converted before validator policy is applied.
//! boundaryOwnerNote: enforcer-lang-cfml owns this CFLint integration boundary.
//! Negative invalid-input coverage is exercised by
//! `malformed_json_is_a_parse_error_not_a_panic`.

use std::path::Path;
use std::process::Command;

use enforcer_domain::boundary::validation::ValidationSource;

#[derive(Debug, Clone, serde::Deserialize)]
/// One issue in the CFLint JSON envelope.
pub(crate) struct CflintIssueEnvelope {
    pub(crate) id: String,
    // DEFAULT-JUSTIFICATION: CFLint omits an issue message for some rule codes.
    #[serde(default)]
    pub(crate) message: String,
    // DEFAULT-JUSTIFICATION: CFLint may report an issue without a source location.
    #[serde(default)]
    pub(crate) locations: Vec<CflintLocationEnvelope>,
}

#[derive(Debug, Clone, serde::Deserialize)]
/// One source location in the CFLint JSON envelope.
pub(crate) struct CflintLocationEnvelope {
    // DEFAULT-JUSTIFICATION: a missing CFLint line is normalized to line one by the adapter.
    #[serde(default)]
    pub(crate) line: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
/// One file report in the CFLint JSON envelope.
pub(crate) struct CflintFileReportEnvelope {
    // DEFAULT-JUSTIFICATION: a CFLint file report with no issues is a valid clean result.
    #[serde(default)]
    pub(crate) issues: Vec<CflintIssueEnvelope>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
/// Root CFLint JSON envelope.
pub(crate) struct CflintReportEnvelope {
    // DEFAULT-JUSTIFICATION: an absent reports collection is treated as an empty report.
    #[serde(default)]
    pub(crate) reports: Vec<CflintFileReportEnvelope>,
}

/// Decode untrusted CFLint JSON into its boundary envelope.
pub(crate) fn decode_cflint_report(
    source: ValidationSource<'_>,
) -> Result<CflintReportEnvelope, serde_json::Error> {
    serde_json::from_str(source.as_str())
}

/// Decode an untrusted JSON configuration document.
pub(crate) fn decode_json(
    source: ValidationSource<'_>,
) -> Result<serde_json::Value, serde_json::Error> {
    serde_json::from_str(source.as_str())
}

/// Invoke CFLint and retain its stdout for boundary decoding.
pub(crate) fn run_cflint(file_path: &Path) -> Option<String> {
    let output = match Command::new("cflint").arg("-json").arg(file_path).output() {
        Ok(output) => output,
        Err(_) => return None,
    };
    // ALLOC-JUSTIFICATION: process stdout must outlive the command output buffer.
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}
