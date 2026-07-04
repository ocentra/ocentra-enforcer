mod cli_flag;
mod cli_flag_output;
mod cli_flag_scope;
mod cli_flag_toggle;
mod cli_flag_value;
mod cli_mode;
mod cli_output;
mod cli_parse;
mod cli_usage;

use std::env;
use std::process;

use enforcer_literal_scan::run_scan;

use crate::cli_output::{exit_if_failed, print_report};
use crate::cli_parse::parse_args;
use crate::cli_usage::print_usage;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let opts = match parse_args(&args) {
        Ok(opts) => opts,
        Err(message) => fail_with_usage(&message),
    };
    if opts.help {
        print_usage();
        return;
    }
    let report = run_scan_or_exit(&opts);
    print_report(&report, opts.output_format);
    exit_if_failed(&report);
}

#[allow(clippy::print_stderr)]
fn fail_with_usage(message: &str) -> ! {
    eprintln!("{message}");
    print_usage();
    process::exit(2);
}

#[allow(clippy::print_stderr)]
fn run_scan_or_exit(opts: &enforcer_literal_scan::CliOptions) -> enforcer_literal_scan::ScanReport {
    match run_scan(opts) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("enforcer-literal-scan failed: {error}");
            process::exit(1);
        }
    }
}
