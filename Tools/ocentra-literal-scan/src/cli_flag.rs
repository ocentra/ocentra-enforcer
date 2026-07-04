use ocentra_literal_scan::CliOptions;

use crate::cli_flag_output::apply_output_flag;
use crate::cli_flag_scope::apply_scope_flag;
use crate::cli_flag_toggle::apply_toggle_flag;
use crate::cli_flag_value::apply_value_flag;

pub(crate) fn apply_flag(
    args: &[String],
    index: &mut usize,
    opts: &mut CliOptions,
) -> Result<(), String> {
    if apply_scope_flag(args, index, opts)? {
        return Ok(());
    }
    if apply_output_flag(&args[*index], opts) {
        return Ok(());
    }
    if apply_toggle_flag(&args[*index], opts) {
        return Ok(());
    }
    if apply_value_flag(args, index, opts)? {
        return Ok(());
    }
    Err(format!("unknown argument: {}", args[*index]))
}
