//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Caller-facing scan request DTO.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::scan_types::ScanMode;
use enforcer_domain::scan_types::{ResolvedScanPlan, ScanModeError};

/// Raw external scan request decoded before scope and commit values are
/// converted into canonical domain types by `crate::modes`.
/// ROUNDTRIP-TEST: `tests/modes.rs::scan_mode_and_request_round_trip_through_the_external_wire_contract`
/// proves this caller-facing wire contract preserves the request exactly.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRequest {
    /// The named mode selected by the caller.
    pub mode: ScanMode,
    /// Optional crate/folder/plan scope; validated at the request boundary.
    // SERDE-DEFAULT-JUSTIFICATION: omission preserves a caller-selected absence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Older commit-ish endpoint, required only for `diff` mode.
    // SERDE-DEFAULT-JUSTIFICATION: `diff` mode rejects an absent base endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    /// Newer commit-ish endpoint, required only for `diff` mode.
    // SERDE-DEFAULT-JUSTIFICATION: `diff` mode rejects an absent head endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
}

/// Decode a scan mode at the external JSON boundary.
pub fn decode_scan_mode_json(payload: &str) -> Result<ScanMode, serde_json::Error> {
    serde_json::from_str(payload)
}

/// Decode a scan request at the external JSON boundary.
pub fn decode_scan_request_json(payload: &str) -> Result<ScanRequest, serde_json::Error> {
    serde_json::from_str(payload)
}

impl Default for ScanRequest {
    fn default() -> Self {
        Self {
            mode: ScanMode::Scoped,
            scope: None,
            base: None,
            head: None,
        }
    }
}

impl ScanRequest {
    /// Convert the decoded request into the canonical execution plan.
    pub fn into_domain(
        self,
        repo_root: &RepoRoot,
        cwd_scope: &RelPath,
    ) -> Result<ResolvedScanPlan, ScanModeError> {
        self.resolve(repo_root, cwd_scope)
    }
}

/// Validate a raw caller scope before it is converted into a `PathBuf`.
pub(crate) fn validate_scope_input(raw: &str) -> Result<(), DecodeError> {
    let normalized = enforcer_core::platform::normalize_separators(raw);
    let trimmed = normalized.trim_start_matches('/');
    if trimmed.is_empty() {
        return Err(DecodeError::new("scanRequest.scope", "must not be empty"));
    }
    if is_drive_or_unc_absolute(trimmed) {
        return Ok(());
    }
    let mut depth: i32 = 0;
    for segment in trimmed.split('/') {
        match segment {
            ".." => {
                depth -= 1;
                if depth < 0 {
                    return Err(DecodeError::new(
                        "scanRequest.scope",
                        "`..` segment escapes the repository root",
                    ));
                }
            }
            "" | "." => {}
            _ => depth += 1,
        }
    }
    Ok(())
}

fn is_drive_or_unc_absolute(normalized: &str) -> bool {
    normalized.starts_with("//")
        || (normalized.len() >= 3
            && normalized.as_bytes()[0].is_ascii_alphabetic()
            && normalized.as_bytes()[1] == b':'
            && normalized.as_bytes()[2] == b'/')
}
