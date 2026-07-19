//! Decoding boundary for raw values entering Plan domain logic.

use enforcer_domain::paths::RelPath;
use enforcer_domain::plan_types::{
    PlanArtifactPath, PlanBudgetBytes, PlanClaimBlockReason, PlanCurrentState,
    PlanDiagnosticDetail, PlanDocumentText, PlanFileContent, PlanResumeAnchor,
};
use std::path::PathBuf;

pub(crate) fn diagnostic_detail(value: String) -> PlanDiagnosticDetail {
    decode_with_fallback(
        value,
        "invalid Plan diagnostic detail",
        PlanDiagnosticDetail::try_new,
    )
}

pub(crate) fn document_text(value: String) -> PlanDocumentText {
    decode_with_fallback(
        value,
        "invalid rendered Plan document",
        PlanDocumentText::try_new,
    )
}

pub(crate) fn file_content(value: String) -> PlanFileContent {
    PlanFileContent::try_new(value).unwrap_or_else(|_| file_content(String::new()))
}

pub(crate) fn resume_anchor(value: String) -> PlanResumeAnchor {
    decode_with_fallback(
        value,
        "invalid Plan resume anchor",
        PlanResumeAnchor::try_new,
    )
}

pub(crate) fn current_state(value: String) -> PlanCurrentState {
    decode_with_fallback(
        value,
        "invalid Plan current state",
        PlanCurrentState::try_new,
    )
}

pub(crate) fn claim_block_reason(value: String) -> PlanClaimBlockReason {
    decode_with_fallback(
        value,
        "invalid Plan claim block reason",
        PlanClaimBlockReason::try_new,
    )
}

pub(crate) fn artifact_path(value: PathBuf) -> PlanArtifactPath {
    PlanArtifactPath::try_new(value)
        .or_else(|_| PlanArtifactPath::try_new(PathBuf::from(".")))
        .unwrap_or_else(|_| artifact_path(PathBuf::from("plan-artifact")))
}

pub(crate) fn rel_path(value: String) -> RelPath {
    RelPath::try_from(value).unwrap_or_else(|_| rel_path("plan-artifact".to_owned()))
}

pub(crate) fn budget_bytes(value: usize) -> PlanBudgetBytes {
    PlanBudgetBytes::try_new(value).unwrap_or(PlanBudgetBytes::DEFAULT)
}

fn decode_with_fallback<T>(
    value: String,
    fallback: &str,
    decode: impl Fn(String) -> Result<T, enforcer_domain::boundary::decode_error::DecodeError>,
) -> T {
    let mut candidate = value;
    loop {
        match decode(candidate) {
            Ok(decoded) => return decoded,
            Err(_) => candidate = fallback.to_owned(),
        }
    }
}
