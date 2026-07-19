use std::path::PathBuf;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_literal_scan::CliOptions;

pub(crate) fn apply_scope_flag(
    args: &[String],
    index: &mut usize,
    opts: &mut CliOptions,
) -> Result<bool, DecodeError> {
    let flag = args
        .get(*index)
        .map(String::as_str)
        .ok_or_else(|| DecodeError::new("cli.scope", "missing scope flag"))?;
    match flag {
        "--root" => {
            let value_index = index
                .checked_add(1)
                .ok_or_else(|| DecodeError::new("cli.root", "argument index overflow"))?;
            let value = args
                .get(value_index)
                .ok_or_else(|| DecodeError::new("cli.root", "--root requires a path"))?;
            opts.root = PathBuf::from(value).into();
            *index = value_index;
            Ok(true)
        }
        "--files" => {
            let mut next_index = index
                .checked_add(1)
                .ok_or_else(|| DecodeError::new("cli.files", "argument index overflow"))?;
            while let Some(value) = args.get(next_index) {
                if value.starts_with('-') {
                    break;
                }
                opts.files.push(PathBuf::from(value));
                next_index = next_index
                    .checked_add(1)
                    .ok_or_else(|| DecodeError::new("cli.files", "argument index overflow"))?;
            }
            *index = next_index.saturating_sub(1);
            Ok(true)
        }
        _ => Ok(false),
    }
}
