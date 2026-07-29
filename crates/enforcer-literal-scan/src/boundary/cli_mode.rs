//! Decode the optional CLI mode before ordinary flags are processed.
//! BOUNDARY-INVARIANT: malformed explain requests fail before a command is selected.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::scan_types::LiteralScanCommand;
use enforcer_literal_scan::CliOptions;

pub(crate) fn consume_mode(args: &[String], opts: &mut CliOptions) -> Result<usize, DecodeError> {
    if args.first().map(String::as_str) == Some("scan") {
        return Ok(1);
    }
    if args.first().map(String::as_str) == Some("languages") {
        opts.command = LiteralScanCommand::Languages;
        return Ok(args.len());
    }
    if args.first().map(String::as_str) == Some("explain") {
        opts.command = LiteralScanCommand::Explain;
        opts.explain_category = Some(
            args.get(1)
                .ok_or_else(|| DecodeError::new("cli.explain", "requires a category"))?
                .parse()?,
        );
        return Ok(args.len());
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::consume_mode;
    use enforcer_literal_scan::CliOptions;

    #[test]
    fn explain_without_category_returns_specific_decode_error(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let error = consume_mode(&[String::from("explain")], &mut CliOptions::default())
            .err()
            .ok_or("invalid explain request unexpectedly decoded")?;
        assert_eq!(
            error.to_string(),
            "decode/validation failed at `cli.explain`: requires a category"
        );
        Ok(())
    }
}
