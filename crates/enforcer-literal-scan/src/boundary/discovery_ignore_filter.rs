//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use std::fs;
use std::path::Path;

use crate::discovery_ignore_match::{is_default_ignored_dir, is_default_ignored_file};
use crate::discovery_ignore_state::IgnoreState;
use crate::path_normalization::normalize_path;
use crate::{CliOptions, IgnoredSummary};

pub(crate) struct SkipPathContext<'a> {
    pub(crate) root: &'a Path,
    pub(crate) opts: &'a CliOptions,
    pub(crate) ignore_state: &'a IgnoreState,
    pub(crate) ignored: &'a mut IgnoredSummary,
}

pub(crate) fn should_skip_path(
    current: &Path,
    metadata: &fs::Metadata,
    context: &mut SkipPathContext<'_>,
) -> bool {
    if context.opts.include_ignored.is_enabled() {
        return false;
    }
    if metadata.is_dir() && is_default_ignored_dir(current) {
        context.ignored.default_dirs += 1;
        return true;
    }
    if metadata.is_file() && is_default_ignored_file(current) {
        context.ignored.default_files += 1;
        return true;
    }
    if context.opts.respect_gitignore.is_enabled() {
        let rel = current.strip_prefix(context.root).unwrap_or(current);
        let rel_norm = normalize_path(rel);
        if context.ignore_state.matches(&rel_norm, metadata.is_dir()) {
            context.ignored.gitignore += 1;
            return true;
        }
    }
    false
}
