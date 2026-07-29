//! `typescript/tests` — TS-8.10 (decoder and schema tests require negative
//! cases). A test file that exercises a `Schema`/decoder MUST also contain
//! at least one negative-case assertion (`toThrow`, `.rejects`, or an
//! "invalid"/"malformed" test title) — a decoder test suite that only
//! proves the happy path is incomplete.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::finding::{from_source, SourceFinding, FIRST_SOURCE_LINE};
use crate::boundary::source_analysis::{exercises_a_decoder, has_negative_case};

const RULE_ID: &str = "TS-8.10";

/// `typescript/tests` validator for TS-8.10.
#[derive(Debug)]
#[doc = "Decoder negative-case test validator."]
pub struct DecoderNegativeCaseValidator {
    rule_id: RuleId,
}

impl DecoderNegativeCaseValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id(RULE_ID)?,
        })
    }
}

impl Validator for DecoderNegativeCaseValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if exercises_a_decoder(input.source.as_str()) && !has_negative_case(input.source.as_str()) {
            return from_source(
                &self.rule_id,
                input.file,
                SourceFinding {
                    severity: Severity::Error,
                    title: "Decoder and schema tests require negative cases",
                    // ALLOC-JUSTIFICATION: the canonical Finding owns diagnostic
                    // detail after this borrowed validator invocation returns.
                    detail: "test file exercises a decoder/schema but has no invalid-input case"
                        .to_owned(),
                    line: FIRST_SOURCE_LINE,
                    snippet: None,
                },
            )
            .into_iter()
            .collect();
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::DecoderNegativeCaseValidator;
    use crate::boundary::test_fixtures::run_fixture_parity;

    #[test]
    fn requires_a_negative_case_when_a_decoder_is_exercised(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = DecoderNegativeCaseValidator::new()?;
        run_fixture_parity(
            &validator,
            "fixtures/tests-family/ts-8-10/fail.test.ts",
            "fixtures/tests-family/ts-8-10/pass.test.ts",
        )?;
        Ok(())
    }
}
