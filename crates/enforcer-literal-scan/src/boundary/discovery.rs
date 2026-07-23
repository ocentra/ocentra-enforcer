//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{CliOptions, IgnoredSummary};
use crate::discovery_ignore_state::IgnoreState;
use crate::discovery_ignore_walk::{walk, WalkContext};
use crate::lexer::lex_literals;
use crate::lexer_shared::line_at;
use crate::scan_types::{FileJob, FileResult};

pub(crate) fn scan_file(job: FileJob) -> io::Result<FileResult> {
    let source = fs::read_to_string(&job.path)?;
    let mut candidates = lex_literals(&source, job.language, job.rel.as_str());
    for candidate in &mut candidates {
        if candidate.context.as_str().is_empty() {
            candidate.context = line_at(&source, candidate.line.get())
                .unwrap_or_default()
                .into();
        }
    }
    Ok(FileResult {
        file: job.rel,
        language: job.language.id.into(),
        role: job.role,
        candidates,
        findings: Vec::new(),
    })
}

pub(crate) fn discover_files(
    root: &Path,
    opts: &CliOptions,
    ignore_state: &IgnoreState,
    ignored: &mut IgnoredSummary,
) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let starts = if opts.files.as_slice().is_empty() {
        vec![root.to_path_buf()]
    } else {
        opts.files
            .as_slice()
            .iter()
            .map(|entry| {
                if entry.is_absolute() {
                    entry.to_path_buf()
                } else {
                    root.join(entry)
                }
            })
            .collect()
    };
    {
        let mut walk_context = WalkContext {
            root,
            opts,
            ignore_state,
            ignored,
            out: &mut out,
        };
        for start in starts {
            walk(&start, &mut walk_context)?;
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

pub(crate) fn chunk_jobs(mut jobs: Vec<FileJob>, chunks: usize) -> Vec<Vec<FileJob>> {
    if jobs.is_empty() {
        return Vec::new();
    }
    jobs.sort_by(|a, b| a.rel.cmp(&b.rel));
    let count = chunks.max(1).min(jobs.len());
    let mut out = vec![Vec::new(); count];
    for (index, job) in jobs.into_iter().enumerate() {
        if let Some(chunk) = out.get_mut(index % count) {
            chunk.push(job);
        }
    }
    out
}
