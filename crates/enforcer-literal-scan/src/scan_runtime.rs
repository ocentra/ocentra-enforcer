use std::fs;
use std::io;
use std::time::SystemTime;

use crate::discovery_ignore::IgnoreState;
use crate::scan_jobs::build_scan_jobs;
use crate::scan_parallel::scan_jobs_in_parallel;
use crate::scan_results::classify_scan_results;
use crate::{CliOptions, IgnoredSummary, ScanReport, ScanSummary};

pub fn run_scan(opts: &CliOptions) -> io::Result<ScanReport> {
    let started = SystemTime::now();
    let root = fs::canonicalize(&opts.root).unwrap_or_else(|_| opts.root.clone());
    let ignore_state = IgnoreState::load(&root, opts.respect_gitignore);
    let mut ignored = IgnoredSummary::default();
    let (files_discovered, jobs) = build_scan_jobs(&root, opts, &ignore_state, &mut ignored)?;
    let files_scanned = jobs.len();
    let results = scan_jobs_in_parallel(jobs);
    let (literals_found, hard_findings, literal_risks, languages) =
        classify_scan_results(results, opts);

    Ok(ScanReport {
        ok: hard_findings.is_empty(),
        summary: ScanSummary {
            files_discovered,
            files_scanned,
            files_ignored: total_ignored_files(&ignored),
            literals_found,
            literal_risks: literal_risks.len(),
            hard_findings: hard_findings.len(),
            duration_ms: started
                .elapsed()
                .map(|elapsed| elapsed.as_millis())
                .unwrap_or(0),
        },
        ignored,
        hard_findings,
        literal_risks,
        languages,
    })
}

fn total_ignored_files(ignored: &IgnoredSummary) -> usize {
    ignored.gitignore
        + ignored.default_dirs
        + ignored.default_files
        + ignored.binary
        + ignored.too_large
        + ignored.unknown_language
}
