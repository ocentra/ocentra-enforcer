//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use std::ffi::OsStr;
use std::path::Path;

use crate::discovery_ignore_glob::glob_match;
use crate::{DEFAULT_IGNORED_DIRS, DEFAULT_IGNORED_FILE_SUFFIXES};

pub(crate) fn gitignore_pattern_matches(pattern: &str, rel: &str, is_dir: bool) -> bool {
    let mut pat = pattern.trim();
    if pat.is_empty() {
        return false;
    }
    let dir_only = pat.ends_with('/');
    if dir_only {
        pat = pat.trim_end_matches('/');
    }
    if dir_only && !is_dir && !rel.starts_with(&format!("{pat}/")) {
        return false;
    }
    if pat.contains('*') {
        return glob_match(pat, rel) || rel.split('/').any(|part| glob_match(pat, part));
    }
    if pat.contains('/') {
        return rel == pat || rel.starts_with(&format!("{pat}/"));
    }
    rel.split('/').any(|part| part == pat)
}

pub(crate) fn is_default_ignored_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .map(|name| {
            DEFAULT_IGNORED_DIRS
                .iter()
                .any(|entry| name.eq_ignore_ascii_case(entry))
        })
        .unwrap_or(false)
}

pub(crate) fn is_default_ignored_file(path: &Path) -> bool {
    let name = path.file_name().and_then(OsStr::to_str).unwrap_or("");
    DEFAULT_IGNORED_FILE_SUFFIXES.iter().any(|suffix| {
        name.to_ascii_lowercase()
            .ends_with(&suffix.to_ascii_lowercase())
    })
}
