use ocentra_literal_scan::{CliOptions, OutputFormat};

pub(crate) fn apply_output_flag(flag: &str, opts: &mut CliOptions) -> bool {
    match flag {
        "--json" => opts.output_format = OutputFormat::Json,
        "--jsonl" => opts.output_format = OutputFormat::JsonLines,
        "--human" => opts.output_format = OutputFormat::Human,
        _ => return false,
    }
    true
}
