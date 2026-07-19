use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::discovery_ignore_filter::{should_skip_path, SkipPathContext};
use crate::discovery_ignore_state::IgnoreState;
use crate::{CliOptions, IgnoredSummary};

pub(crate) struct WalkContext<'a> {
    pub(crate) root: &'a Path,
    pub(crate) opts: &'a CliOptions,
    pub(crate) ignore_state: &'a IgnoreState,
    pub(crate) ignored: &'a mut IgnoredSummary,
    pub(crate) out: &'a mut Vec<PathBuf>,
}

pub(crate) fn walk(current: &Path, context: &mut WalkContext<'_>) -> io::Result<()> {
    let metadata = match fs::symlink_metadata(current) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(()),
    };
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    let mut skip_context = SkipPathContext {
        root: context.root,
        opts: context.opts,
        ignore_state: context.ignore_state,
        ignored: context.ignored,
    };
    if should_skip_path(current, &metadata, &mut skip_context) {
        return Ok(());
    }
    push_file_or_walk_dir(current, &metadata, context)
}

fn push_file_or_walk_dir(
    current: &Path,
    metadata: &fs::Metadata,
    context: &mut WalkContext<'_>,
) -> io::Result<()> {
    if metadata.is_dir() {
        return visit_directory(current, context);
    }
    if metadata.is_file() {
        context.out.push(current.to_path_buf());
    }
    Ok(())
}

fn visit_directory(current: &Path, context: &mut WalkContext<'_>) -> io::Result<()> {
    let mut entries = fs::read_dir(current)?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        walk(&entry.path(), context)?;
    }
    Ok(())
}
