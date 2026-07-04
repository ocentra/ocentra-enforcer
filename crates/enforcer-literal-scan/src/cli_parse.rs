use enforcer_literal_scan::CliOptions;

use crate::cli_flag::apply_flag;
use crate::cli_mode::consume_mode;

pub(crate) fn parse_args(args: &[String]) -> Result<CliOptions, String> {
    let mut opts = CliOptions::default();
    let mut index = consume_mode(args, &mut opts);

    while index < args.len() {
        apply_flag(args, &mut index, &mut opts)?;
        index += 1;
    }
    Ok(opts)
}
