//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.

use enforcer_domain::scan_types::LiteralFindingPath;

use crate::{FileRole, LanguageSpec};

pub(crate) fn classify_file_role(rel: &LiteralFindingPath, language: LanguageSpec) -> FileRole {
    crate::domain::file_role::classify_file_role(rel, language)
}
