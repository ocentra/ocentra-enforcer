use enforcer_literal_scan::CliOptions;

pub(crate) fn apply_value_flag(
    args: &[String],
    index: &mut usize,
    opts: &mut CliOptions,
) -> Result<bool, String> {
    match args[*index].as_str() {
        "--min-score" => {
            *index += 1;
            opts.min_score = parse_u8(args.get(*index), "--min-score")?;
            Ok(true)
        }
        "--fail-above" => {
            *index += 1;
            opts.fail_above = Some(parse_u8(args.get(*index), "--fail-above")?);
            Ok(true)
        }
        "--max-file-bytes" => {
            *index += 1;
            let raw = args.get(*index).ok_or("--max-file-bytes requires a number")?;
            opts.max_file_bytes = raw
                .parse::<u64>()
                .map_err(|_| "--max-file-bytes must be a positive integer".to_string())?;
            Ok(true)
        }
        "--languages" => {
            *index += 1;
            let raw = args.get(*index).ok_or("--languages requires a comma list")?;
            opts.languages = raw
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(str::to_string)
                .collect();
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn parse_u8(value: Option<&String>, flag: &str) -> Result<u8, String> {
    let raw = value.ok_or_else(|| format!("{flag} requires a number"))?;
    raw.parse::<u8>()
        .map_err(|_| format!("{flag} must be a number from 0 to 100"))
        .and_then(|value| {
            if value <= 100 {
                Ok(value)
            } else {
                Err(format!("{flag} must be between 0 and 100"))
            }
        })
}
