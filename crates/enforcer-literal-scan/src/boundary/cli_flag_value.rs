//! Decode value-bearing CLI flags into canonical scan values.
//! BOUNDARY-INVARIANT: raw argument text is rejected before it reaches scanner state.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::scan_types::{LiteralFileByteLimit, LiteralRiskScore};
use enforcer_literal_scan::CliOptions;

pub(crate) fn apply_value_flag(
    args: &[String],
    index: &mut usize,
    opts: &mut CliOptions,
) -> Result<bool, DecodeError> {
    let flag = args
        .get(*index)
        .ok_or_else(|| DecodeError::new("cli.flag", "argument index is out of bounds"))?;
    match flag.as_str() {
        "--min-score" => {
            *index += 1;
            opts.min_score = decode_risk_score(args.get(*index), "--min-score")?;
            Ok(true)
        }
        "--fail-above" => {
            *index += 1;
            opts.fail_above = Some(decode_risk_score(args.get(*index), "--fail-above")?);
            Ok(true)
        }
        "--max-file-bytes" => {
            *index += 1;
            let raw = args.get(*index).ok_or_else(|| {
                DecodeError::new("cli.maxFileBytes", "requires a positive integer")
            })?;
            let value = match raw.parse::<u64>() {
                Ok(value) => value,
                Err(_) => return Err(DecodeError::new("cli.maxFileBytes", "must be an integer")),
            };
            let value = std::num::NonZeroU64::new(value)
                .ok_or_else(|| DecodeError::new("cli.maxFileBytes", "must be greater than zero"))?;
            opts.max_file_bytes = LiteralFileByteLimit::try_from_nonzero(value);
            Ok(true)
        }
        "--languages" => {
            *index += 1;
            let raw = args
                .get(*index)
                .ok_or_else(|| DecodeError::new("cli.languages", "requires a comma list"))?;
            opts.languages = raw
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(str::parse)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn decode_percentage(value: Option<&String>, flag: &str) -> Result<u8, DecodeError> {
    let raw = value.ok_or_else(|| DecodeError::new(flag, "requires a number"))?;
    let value = match raw.parse::<u8>() {
        Ok(value) => value,
        Err(_) => {
            return Err(DecodeError::new(
                flag,
                "must be an integer between 0 and 100",
            ))
        }
    };
    if value <= 100 {
        Ok(value)
    } else {
        Err(DecodeError::new(flag, "must be between 0 and 100"))
    }
}

fn decode_risk_score(value: Option<&String>, flag: &str) -> Result<LiteralRiskScore, DecodeError> {
    let value = decode_percentage(value, flag)?;
    match std::num::NonZeroU8::new(value) {
        Some(value) => LiteralRiskScore::try_from(value),
        None => Ok(LiteralRiskScore::ZERO),
    }
}

#[cfg(test)]
mod tests {
    use super::decode_risk_score;

    #[test]
    fn malformed_and_out_of_range_scores_return_specific_decode_errors(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let malformed = String::from("not-a-score");
        let malformed_error = decode_risk_score(Some(&malformed), "--min-score")
            .err()
            .ok_or("malformed score unexpectedly decoded")?;
        assert_eq!(
            malformed_error.to_string(),
            "decode/validation failed at `--min-score`: must be an integer between 0 and 100"
        );

        let oversized = String::from("101");
        let oversized_error = decode_risk_score(Some(&oversized), "--min-score")
            .err()
            .ok_or("oversized score unexpectedly decoded")?;
        assert_eq!(
            oversized_error.to_string(),
            "decode/validation failed at `--min-score`: must be between 0 and 100"
        );
        Ok(())
    }
}
