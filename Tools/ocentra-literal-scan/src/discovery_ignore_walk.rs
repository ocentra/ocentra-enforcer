use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::discovery_ignore::{should_skip_path, IgnoreState};
use crate::{CliOptions, IgnoredSummary};

pub(crate) fn walk(
    root: &Path,
    current: &Path,
    opts: &CliOptions,
    ignore_state: &IgnoreState,
    ignored: &mut IgnoredSummary,
    out: &mut Vec<PathBuf>,
) -> io::Result<()> {
    let metadata = match fs::symlink_metadata(current) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(()),
    };
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    if should_skip_path(root, current, &metadata, opts, ignore_state, ignored) {
        return Ok(());
    }
    push_file_or_walk_dir(root, current, opts, ignore_state, ignored, out, &metadata)
}

fn push_file_or_walk_dir(
    root: &Path,
    current: &Path,
    opts: &CliOptions,
    ignore_state: &IgnoreState,
    ignored: &mut IgnoredSummary,
    out: &mut Vec<PathBuf>,
    metadata: &fs::Metadata,
) -> io::Result<()> {
    if metadata.is_dir() {
        return visit_directory(root, current, opts, ignore_state, ignored, out);
    }
    if metadata.is_file() {
        out.push(current.to_path_buf());
    }
    Ok(())
}

fn visit_directory(
    root: &Path,
    current: &Path,
    opts: &CliOptions,
    ignore_state: &IgnoreState,
    ignored: &mut IgnoredSummary,
    out: &mut Vec<PathBuf>,
) -> io::Result<()> {
    let mut entries = fs::read_dir(current)?.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        walk(root, &entry.path(), opts, ignore_state, ignored, out)?;
    }
    Ok(())
}
