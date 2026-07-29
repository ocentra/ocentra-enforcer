//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use enforcer_domain::scan_types::LiteralScanOutputFormat as OutputFormat;
use enforcer_literal_scan::CliOptions;

pub(crate) fn apply_output_flag(flag: &str, opts: &mut CliOptions) -> bool {
    match flag {
        "--json" => opts.output_format = OutputFormat::Json,
        "--jsonl" => opts.output_format = OutputFormat::JsonLines,
        "--human" => opts.output_format = OutputFormat::Human,
        _ => return false,
    }
    true
}
