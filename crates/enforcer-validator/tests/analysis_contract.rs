use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::findings::ScanScope;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::syntax_types::{CapabilitySet, ProviderVersion};
// source owner: crates/enforcer-validator/src/analysis.rs
// generator: cargo test -p enforcer-validator --test analysis_contract
// contractHash: 7129e278e32c8fa681eabf9adf1cd19a53da32362725750b400120a9c992683b
use enforcer_validator::analysis::{
    content_hash, AnalysisOutcome, AnalysisProvider, LegacyAnalysisProvider, PreparedAnalysis,
};
use enforcer_validator::validator::{AnalysisSkip, ValidationDispatch, ValidationInput, Validator};

struct TextValidator {
    rule_id: RuleId,
}

impl Validator for TextValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, _input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        Vec::new()
    }
}

struct FactValidator {
    rule_id: RuleId,
}

impl Validator for FactValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn required_capabilities(&self) -> CapabilitySet {
        CapabilitySet::function_facts()
    }

    fn validate(&self, _input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        Vec::new()
    }
}

#[test]
fn legacy_provider_is_explicitly_non_fact_backed() -> Result<(), DecodeError> {
    let path = RelPath::try_new("src/example.rs")?;
    let provider = LegacyAnalysisProvider;
    let outcome = provider.analyze(
        &path,
        ValidationSource::from_text("fn legacy() {}"),
        ScanScope::Files,
    );
    assert_eq!(outcome, AnalysisOutcome::LegacyText);
    Ok(())
}

#[test]
fn fact_requirement_cannot_fall_through_to_empty_findings() -> Result<(), DecodeError> {
    let path = RelPath::try_new("src/example.rs")?;
    let validator = FactValidator {
        rule_id: "RR-99.1".parse()?,
    };
    let input = ValidationInput {
        file: &path,
        source: ValidationSource::from_text("fn missing() {}"),
        scope: ScanScope::Files,
    };
    assert_eq!(
        validator.validate_with_analysis(input, None),
        ValidationDispatch::Skipped(AnalysisSkip::NotPrepared)
    );
    Ok(())
}

#[test]
fn unavailable_and_parser_failure_are_visible_to_fact_dispatch() -> Result<(), DecodeError> {
    let path = RelPath::try_new("src/example.rs")?;
    let validator = FactValidator {
        rule_id: "RR-99.3".parse()?,
    };
    let input = ValidationInput {
        file: &path,
        source: ValidationSource::from_text("fn unavailable() {}"),
        scope: ScanScope::Files,
    };
    let unavailable = PreparedAnalysis::new(
        content_hash(input.source),
        ProviderVersion::TreeSitter025,
        AnalysisOutcome::ProviderUnavailable,
    );
    assert_eq!(
        validator.validate_with_analysis(input, Some(&unavailable)),
        ValidationDispatch::Skipped(AnalysisSkip::RequirementUnavailable)
    );

    let parser_failure = PreparedAnalysis::new(
        content_hash(input.source),
        ProviderVersion::TreeSitter025,
        AnalysisOutcome::ParserFailure,
    );
    assert_eq!(
        validator.validate_with_analysis(input, Some(&parser_failure)),
        ValidationDispatch::Skipped(AnalysisSkip::RequirementUnavailable)
    );
    Ok(())
}

#[test]
fn legacy_validator_dispatch_remains_running() -> Result<(), DecodeError> {
    let path = RelPath::try_new("src/example.rs")?;
    let validator = TextValidator {
        rule_id: "RR-99.2".parse()?,
    };
    let input = ValidationInput {
        file: &path,
        source: ValidationSource::from_text("plain text"),
        scope: ScanScope::Files,
    };
    assert_eq!(
        validator.validate_with_analysis(input, None),
        ValidationDispatch::Ran(Vec::new())
    );
    Ok(())
}
