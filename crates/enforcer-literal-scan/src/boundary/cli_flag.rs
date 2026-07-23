//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_literal_scan::CliOptions;

use crate::cli_flag_output::apply_output_flag;
use crate::cli_flag_scope::apply_scope_flag;
use crate::cli_flag_toggle::apply_toggle_flag;
use crate::cli_flag_value::apply_value_flag;

pub(crate) fn apply_flag(
    args: &[String],
    index: &mut usize,
    opts: &mut CliOptions,
) -> Result<(), DecodeError> {
    if apply_scope_flag(args, index, opts)? {
        return Ok(());
    }
    let flag = args
        .get(*index)
        .ok_or_else(|| DecodeError::new("cli.flag", "argument index is out of bounds"))?;
    if apply_output_flag(flag, opts) {
        return Ok(());
    }
    if apply_toggle_flag(flag, opts) {
        return Ok(());
    }
    if apply_value_flag(args, index, opts)? {
        return Ok(());
    }
    Err(DecodeError::new(
        "cli.flag",
        format!("unknown argument `{flag}`"),
    ))
}
