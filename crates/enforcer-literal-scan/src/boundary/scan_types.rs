//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use std::path::PathBuf;

use crate::{FileRole, Finding, LanguageSpec, LiteralCandidate};
use enforcer_domain::scan_types::{LiteralFindingPath, LiteralLanguageId};

#[derive(Debug, Clone)]
pub(crate) struct FileJob {
    pub(crate) path: PathBuf,
    pub(crate) rel: LiteralFindingPath,
    pub(crate) language: LanguageSpec,
    pub(crate) role: FileRole,
}

#[derive(Debug, Clone)]
pub(crate) struct FileResult {
    pub(crate) file: LiteralFindingPath,
    pub(crate) language: LiteralLanguageId,
    pub(crate) role: FileRole,
    pub(crate) candidates: Vec<LiteralCandidate>,
    pub(crate) findings: Vec<Finding>,
}
