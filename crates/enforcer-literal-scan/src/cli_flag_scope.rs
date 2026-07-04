use std::path::PathBuf;

use enforcer_literal_scan::CliOptions;

pub(crate) fn apply_scope_flag(
    args: &[String],
    index: &mut usize,
    opts: &mut CliOptions,
) -> Result<bool, String> {
    match args[*index].as_str() {
        "--root" => {
            *index += 1;
            let value = args.get(*index).ok_or("--root requires a path")?;
            opts.root = PathBuf::from(value);
            Ok(true)
        }
        "--files" => {
            *index += 1;
            while *index < args.len() && !args[*index].starts_with('-') {
                opts.files.push(PathBuf::from(&args[*index]));
                *index += 1;
            }
            *index = index.saturating_sub(1);
            Ok(true)
        }
        _ => Ok(false),
    }
}
