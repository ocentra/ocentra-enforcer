use std::path::PathBuf;

use crate::{FileRole, Finding, LanguageSpec, LiteralCandidate};

#[derive(Debug, Clone)]
pub(crate) struct FileJob {
    pub(crate) path: PathBuf,
    pub(crate) rel: String,
    pub(crate) language: LanguageSpec,
    pub(crate) role: FileRole,
}

#[derive(Debug, Clone)]
pub(crate) struct FileResult {
    pub(crate) file: String,
    pub(crate) language: String,
    pub(crate) role: FileRole,
    pub(crate) candidates: Vec<LiteralCandidate>,
    pub(crate) findings: Vec<Finding>,
}
