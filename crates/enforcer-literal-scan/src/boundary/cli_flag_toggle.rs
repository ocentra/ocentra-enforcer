//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use enforcer_domain::scan_types::LiteralScanToggle;
use enforcer_literal_scan::CliOptions;

pub(crate) fn apply_toggle_flag(flag: &str, opts: &mut CliOptions) -> bool {
    match flag {
        "--include-low" => opts.include_low = LiteralScanToggle::Enabled,
        "--include-ignored" => opts.include_ignored = LiteralScanToggle::Enabled,
        "--include-unknown-code" => opts.include_unknown_code = LiteralScanToggle::Enabled,
        "--no-respect-gitignore" => opts.respect_gitignore = LiteralScanToggle::Disabled,
        "--help" | "-h" => opts.help = LiteralScanToggle::Enabled,
        _ => return false,
    }
    true
}
