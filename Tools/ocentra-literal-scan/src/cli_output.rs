use std::process;

use ocentra_literal_scan::{OutputFormat, ScanReport};

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
