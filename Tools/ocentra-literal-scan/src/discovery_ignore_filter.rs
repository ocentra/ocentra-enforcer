use std::fs;
use std::path::Path;

use crate::discovery_ignore::{is_default_ignored_dir, is_default_ignored_file, IgnoreState};
use crate::normalize_path;
use crate::{CliOptions, IgnoredSummary};

pub(crate) fn should_skip_path(
    root: &Path,
    current: &Path,
    metadata: &fs::Metadata,
    opts: &CliOptions,
    ignore_state: &IgnoreState,
    ignored: &mut IgnoredSummary,
) -> bool {
    if opts.include_ignored {
        return false;
    }
    if metadata.is_dir() && is_default_ignored_dir(current) {
        ignored.default_dirs += 1;
        return true;
    }
    if metadata.is_file() && is_default_ignored_file(current) {
        ignored.default_files += 1;
        return true;
    }
    if opts.respect_gitignore {
        let rel = current.strip_prefix(root).unwrap_or(current);
        let rel_norm = normalize_path(rel);
        if ignore_state.matches(&rel_norm, metadata.is_dir()) {
            ignored.gitignore += 1;
            return true;
        }
    }
    false
}
