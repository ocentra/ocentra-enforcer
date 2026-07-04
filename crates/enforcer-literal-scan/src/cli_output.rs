use std::process;

use enforcer_literal_scan::{OutputFormat, ScanReport};

// This is the CLI binary's stdout report surface -- printing IS the
// contract here, not a stray debug print. Scoped allow, same rationale as
// the workspace's other narrowly-scoped clippy allows (e.g. unwrap_used in
// enforcer-harness's test-only parsers).
#[allow(clippy::print_stdout)]
pub(crate) fn print_report(report: &ScanReport, format: OutputFormat) {
    match format {
        OutputFormat::Json => println!("{}", report.to_json_pretty()),
        OutputFormat::JsonLines => {
            for line in report.to_json_lines() {
                println!("{line}");
            }
        }
        OutputFormat::Human => println!("{}", report.to_human()),
    }
}

pub(crate) fn exit_if_failed(report: &ScanReport) {
    if !report.ok {
        process::exit(1);
    }
}
