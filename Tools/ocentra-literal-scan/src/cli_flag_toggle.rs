use ocentra_literal_scan::CliOptions;

pub(crate) fn apply_toggle_flag(flag: &str, opts: &mut CliOptions) -> bool {
    match flag {
        "--include-low" => opts.include_low = true,
        "--include-ignored" => opts.include_ignored = true,
        "--include-unknown-code" => opts.include_unknown_code = true,
        "--no-respect-gitignore" => opts.respect_gitignore = false,
        "--help" | "-h" => opts.help = true,
        _ => return false,
    }
    true
}
