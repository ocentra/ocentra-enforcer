//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Regex compilation boundary for security-rule definitions.
//! Malformed patterns return typed errors, with negative coverage in this module's tests.

use enforcer_domain::boundary::decode_error::DecodeError;

/// Preserve a regex parser failure as a typed boundary error.
pub(crate) fn decode(path: &'static str, source: regex::Error) -> DecodeError {
    let message = source.to_string();
    drop(source);
    DecodeError::new(path, message)
}

/// Compile a static rule pattern while preserving parser failures.
pub(crate) fn compile(
    path: &'static str,
    pattern: &'static str,
) -> Result<regex::Regex, DecodeError> {
    regex::Regex::new(pattern).map_err(|source| decode(path, source))
}

/// Compile an owned pattern assembled while decoding a static rule source.
pub(crate) fn compile_owned(
    path: &'static str,
    pattern: &str,
) -> Result<regex::Regex, DecodeError> {
    regex::Regex::new(pattern).map_err(|source| decode(path, source))
}

#[cfg(test)]
mod tests {
    use super::DecodeError;

    #[test]
    fn malformed_pattern_returns_typed_decode_error() -> Result<(), DecodeError> {
        match super::compile("securityPattern", "(") {
            Err(error) => assert_eq!(error.path, "securityPattern"),
            Ok(_) => {
                return Err(DecodeError::new(
                    "securityPattern",
                    "pattern must be rejected",
                ));
            }
        }
        Ok(())
    }
}
