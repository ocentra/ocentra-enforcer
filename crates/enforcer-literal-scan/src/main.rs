#[path = "boundary/cli_flag.rs"]
mod cli_flag;
#[path = "boundary/cli_flag_output.rs"]
mod cli_flag_output;
#[path = "boundary/cli_flag_scope.rs"]
mod cli_flag_scope;
#[path = "boundary/cli_flag_toggle.rs"]
mod cli_flag_toggle;
#[path = "boundary/cli_flag_value.rs"]
mod cli_flag_value;
#[path = "boundary/cli_mode.rs"]
mod cli_mode;
#[path = "boundary/cli_output.rs"]
mod cli_output;
#[path = "boundary/cli_parse.rs"]
mod cli_parse;
#[path = "boundary/cli_usage.rs"]
mod cli_usage;

use std::env;
use std::fmt::Display;
use std::io::{self, Write};
use std::process;

use enforcer_literal_scan::run_scan;

use crate::cli_output::{exit_if_failed, print_report};
use crate::cli_parse::parse_args;
use crate::cli_usage::write_usage;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let opts = match parse_args(&args) {
        Ok(opts) => opts,
        Err(message) => fail_with_usage(&message),
    };
    if opts.help.is_enabled() {
        let _ = write_usage(io::stdout().lock());
        return;
    }
    let report = run_scan_or_exit(&opts);
    if let Err(error) = print_report(&report, opts.output_format) {
        fail_with_usage(error);
    }
    exit_if_failed(&report);
}

fn fail_with_usage(message: impl Display) -> ! {
    let mut stderr = io::stderr().lock();
    let _ = writeln!(stderr, "{message}");
    let _ = write_usage(io::stdout().lock());
    process::exit(2);
}

// PARSER-TEST: cli_usage_reports_help_and_invalid_arguments proves invalid CLI arguments return usage status.
fn run_scan_or_exit(opts: &enforcer_literal_scan::CliOptions) -> enforcer_literal_scan::ScanReport {
    match run_scan(opts) {
        Ok(report) => report,
        Err(error) => {
            let mut stderr = io::stderr().lock();
            let _ = writeln!(stderr, "enforcer-literal-scan failed: {error}");
            process::exit(1);
        }
    }
}
