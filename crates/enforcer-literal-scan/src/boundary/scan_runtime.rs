// BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
// Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use std::fs;
use std::io;
use std::time::SystemTime;

use crate::discovery_ignore_state::IgnoreState;
use crate::scan_jobs::build_scan_jobs;
use crate::scan_parallel::scan_jobs_in_parallel;
use crate::scan_results::classify_scan_results;
/// Execute one fully decoded literal scan request.
pub fn run_scan(opts: &CliOptions) -> io::Result<ScanReport> {
    let started = SystemTime::now();
    let root = match fs::canonicalize(&opts.root) {
        Ok(root) => root,
        Err(_) => opts.root.to_path_buf(),
    };
    let ignore_state = IgnoreState::load(&root, opts.respect_gitignore.is_enabled());
    let mut ignored = IgnoredSummary::default();
    let (files_discovered, jobs) = build_scan_jobs(&root, opts, &ignore_state, &mut ignored)?;
    let files_scanned = jobs.len();
    let results = scan_jobs_in_parallel(jobs);
    let (literals_found, hard_findings, literal_risks, languages) =
        classify_scan_results(results, opts);

    Ok(ScanReport {
        ok: if hard_findings.is_empty() {
            ReportOutcome::Clean
        } else {
            ReportOutcome::Violations
        },
        summary: ScanSummary {
            files_discovered: files_discovered.into(),
            files_scanned: files_scanned.into(),
            files_ignored: total_ignored_files(&ignored),
            literals_found: literals_found.into(),
            literal_risks: literal_risks.len().into(),
            hard_findings: hard_findings.len().into(),
            duration_ms: LiteralScanDurationMillis::from_millis(
                started
                    .elapsed()
                    .map(|elapsed| elapsed.as_millis())
                    .unwrap_or(0),
            ),
        },
        ignored,
        hard_findings,
        literal_risks,
        languages,
    })
}

fn total_ignored_files(ignored: &IgnoredSummary) -> LiteralScanCount {
    ignored.gitignore
        + ignored.default_dirs
        + ignored.default_files
        + ignored.binary
        + ignored.too_large
        + ignored.unknown_language
}
