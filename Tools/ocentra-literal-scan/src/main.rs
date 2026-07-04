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

use ocentra_literal_scan::run_scan;

use crate::cli_output::{exit_if_failed, print_report};
use crate::cli_parse::parse_args;
use crate::cli_usage::print_usage;

fn main() {
    let opts = match parse_args(env::args().skip(1).collect()) {
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

fn fail_with_usage(message: &str) -> ! {
    eprintln!("{message}");
    print_usage();
    process::exit(2);
}

fn run_scan_or_exit(opts: &ocentra_literal_scan::CliOptions) -> ocentra_literal_scan::ScanReport {
    match run_scan(opts) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("ocentra-literal-scan failed: {error}");
            process::exit(1);
        }
    }
}
