//! `typescript/tests` — TS-8.10 (decoder and schema tests require negative
//! cases). A test file that exercises a `Schema`/decoder MUST also contain
//! at least one negative-case assertion (`toThrow`, `.rejects`, or an
//! "invalid"/"malformed" test title) — a decoder test suite that only
//! proves the happy path is incomplete.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

const RULE_ID: &str = "TS-8.10";

fn exercises_a_decoder(source: &str) -> bool {
    source.contains("Schema.decode")
        || source.contains("decodeUnknown")
        || source.contains("Schema.Struct")
}

fn has_negative_case(source: &str) -> bool {
    source.contains("toThrow")
        || source.contains(".rejects")
        || source.contains("invalid")
        || source.contains("malformed")
}

/// `typescript/tests` validator for TS-8.10.
pub struct DecoderNegativeCaseValidator {
    rule_id: RuleId,
}

impl DecoderNegativeCaseValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: RULE_ID.parse()?,
        })
    }
}

impl Validator for DecoderNegativeCaseValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if exercises_a_decoder(input.source) && !has_negative_case(input.source) {
            return vec![Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "Decoder and schema tests require negative cases".to_owned(),
                detail: "test file exercises a decoder/schema but has no invalid-input case"
                    .to_owned(),
                file: input.file.clone(),
                line: 1,
                snippet: None,
            }];
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::DecoderNegativeCaseValidator;
    use enforcer_validator::harness::run_fixture_parity;
    use std::path::PathBuf;

    #[test]
    fn requires_a_negative_case_when_a_decoder_is_exercised(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = DecoderNegativeCaseValidator::new()?;
        run_fixture_parity(
            &validator,
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            "fixtures/tests-family/ts-8-10/fail.test.ts",
            "fixtures/tests-family/ts-8-10/pass.test.ts",
        )?;
        Ok(())
    }
}
