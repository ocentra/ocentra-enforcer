use std::io::{self, Write};
use std::process;

use enforcer_domain::findings::ReportOutcome;
use enforcer_domain::scan_types::LiteralScanOutputFormat as OutputFormat;
use enforcer_literal_scan::ScanReport;

pub(crate) fn print_report(report: &ScanReport, format: OutputFormat) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    match format {
        OutputFormat::Json => writeln!(stdout, "{}", report.to_json_pretty())?,
        OutputFormat::JsonLines => {
            for line in report.to_json_lines() {
                writeln!(stdout, "{line}")?;
            }
        }
        OutputFormat::Human => writeln!(stdout, "{}", report.to_human())?,
    }
    Ok(())
}

pub(crate) fn exit_if_failed(report: &ScanReport) {
    if matches!(report.ok, ReportOutcome::Violations) {
        process::exit(1);
    }
}
