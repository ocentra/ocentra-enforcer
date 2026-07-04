use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::discovery_ignore::{walk, IgnoreState};
use crate::lexer::{lex_literals, line_at};
use super::{CliOptions, FileJob, FileResult, IgnoredSummary};

pub(crate) fn scan_file(job: FileJob) -> io::Result<FileResult> {
    let source = fs::read_to_string(&job.path)?;
    let mut candidates = lex_literals(&source, job.language, &job.rel);
    for candidate in &mut candidates {
        if candidate.context.is_empty() {
            candidate.context = match line_at(&source, candidate.line) {
                Some(line) => line,
                None => String::new(),
            };
        }
    }
    Ok(FileResult {
        file: job.rel,
        language: job.language.id.to_string(),
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
    let starts = if opts.files.is_empty() {
        vec![root.to_path_buf()]
    } else {
        opts.files
            .iter()
            .map(|entry| {
                if entry.is_absolute() {
                    entry.clone()
                } else {
                    root.join(entry)
                }
            })
            .collect()
    };
    for start in starts {
        walk(root, &start, opts, ignore_state, ignored, &mut out)?;
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
        out[index % count].push(job);
    }
    out
}
