use std::path::PathBuf;

use enforcer_literal_scan::CliOptions;

pub(crate) fn apply_scope_flag(
    args: &[String],
    index: &mut usize,
    opts: &mut CliOptions,
) -> Result<bool, String> {
    // ALLOC-JUSTIFICATION: this parser's public error contract owns a
    // diagnostic string after the borrowed argument slice is released.
    let flag = args
        .get(*index)
        .map(String::as_str)
        // ALLOC-JUSTIFICATION: this parser's public error contract owns a
        // diagnostic string after the borrowed argument slice is released.
        .ok_or_else(|| "missing scope flag".to_owned())?;
    match flag {
        "--root" => {
            // ALLOC-JUSTIFICATION: checked parser failures return owned
            // diagnostics through the existing CLI error boundary.
            let value_index = index
                .checked_add(1)
                .ok_or_else(|| "--root index overflow".to_owned())?;
            // ALLOC-JUSTIFICATION: checked parser failures return owned
            // diagnostics through the existing CLI error boundary.
            let value = args
                .get(value_index)
                .ok_or_else(|| "--root requires a path".to_owned())?;
            opts.root = PathBuf::from(value);
            *index = value_index;
            Ok(true)
        }
        "--files" => {
            // ALLOC-JUSTIFICATION: checked parser failures return owned
            // diagnostics through the existing CLI error boundary.
            let mut next_index = index
                .checked_add(1)
                .ok_or_else(|| "--files index overflow".to_owned())?;
            while let Some(value) = args.get(next_index) {
                if value.starts_with('-') {
                    break;
                }
                opts.files.push(PathBuf::from(value));
                // ALLOC-JUSTIFICATION: checked parser failures return owned
                // diagnostics through the existing CLI error boundary.
                next_index = next_index
                    .checked_add(1)
                    .ok_or_else(|| "--files index overflow".to_owned())?;
            }
            *index = next_index.saturating_sub(1);
            Ok(true)
        }
        _ => Ok(false),
    }
}
