//! Compose CLI mode and flag decoders into one canonical scan request.
//! BOUNDARY-INVARIANT: invalid or malformed arguments fail closed with a DecodeError.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_literal_scan::CliOptions;

use crate::cli_flag::apply_flag;
use crate::cli_mode::consume_mode;

pub(crate) fn parse_args(args: &[String]) -> Result<CliOptions, DecodeError> {
    let mut opts = CliOptions::default();
    let mut index = consume_mode(args, &mut opts)?;

    while index < args.len() {
        apply_flag(args, &mut index, &mut opts)?;
        index += 1;
    }
    Ok(opts)
}

#[cfg(test)]
mod tests {
    use super::parse_args;

    fn arguments(values: &[&str]) -> Vec<String> {
        values.iter().copied().map(String::from).collect()
    }

    #[test]
    fn invalid_empty_oversized_and_malformed_inputs_are_handled() {
        assert!(
            parse_args(&arguments(&["--unknown"])).is_err(),
            "invalid input must be rejected"
        );
        assert!(
            parse_args(&arguments(&["--min-score"])).is_err(),
            "empty value must be rejected"
        );
        assert!(
            parse_args(&arguments(&["--min-score", "101"])).is_err(),
            "oversized value must be rejected"
        );
        assert!(
            parse_args(&arguments(&["--min-score", "not-a-number"])).is_err(),
            "malformed value must be rejected"
        );
        assert!(
            parse_args(&arguments(&["explain", "not-a-category"])).is_err(),
            "invalid explain category must be rejected"
        );
    }
}
