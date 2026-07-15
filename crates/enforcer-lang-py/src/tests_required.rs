//! `python/tests` validator: PY-6.8 / PY-6.9 / PY-6.10 (3 rules) --
//! "required coverage SHAPE" rules. Each fires when a test file defines a
//! function whose name signals a behavior category (`test_*_rejects_*` /
//! validators, functions calling something that raises, parser/normalizer
//! functions) but the file never demonstrates the matching test pattern
//! (`pytest.raises`, an explicit invalid-input assertion, or a
//! `@given`/Hypothesis property test).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// Fires when `trigger_markers` are present (the file exercises the
/// behavior category this rule cares about) but none of
/// `satisfying_markers` are present (the file never proves the required
/// coverage shape for that category).
struct RequiredCoverageValidator {
    rule_id: RuleId,
    title: &'static str,
    trigger_markers: &'static [&'static str],
    satisfying_markers: &'static [&'static str],
}

impl Validator for RequiredCoverageValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let has_trigger = self
            .trigger_markers
            .iter()
            .any(|marker| input.source.contains(marker));
        if !has_trigger {
            return Vec::new();
        }
        let has_coverage = self
            .satisfying_markers
            .iter()
            .any(|marker| input.source.contains(marker));
        if has_coverage {
            return Vec::new();
        }
        vec![Finding {
            rule_id: self.rule_id.clone(),
            severity: Severity::Error,
            title: self.title.to_owned(),
            detail: "required test-coverage shape is missing".to_owned(),
            file: input.file.clone(),
            line: 1,
            snippet: None,
        }]
    }
}

/// Build every `python/tests`-keyed validator this crate registers.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(RequiredCoverageValidator {
            rule_id: "PY-6.8".parse()?,
            title: "Python validators require invalid-input tests",
            trigger_markers: &["def validate_", "def parse_"],
            satisfying_markers: &["invalid", "pytest.raises", "rejects"],
        }),
        Box::new(RequiredCoverageValidator {
            rule_id: "PY-6.9".parse()?,
            title: "Python exception paths require tests",
            trigger_markers: &["raise "],
            satisfying_markers: &["pytest.raises"],
        }),
        Box::new(RequiredCoverageValidator {
            rule_id: "PY-6.10".parse()?,
            title: "Python parsers and normalizers require property tests",
            trigger_markers: &["def parse_", "def normalize_"],
            satisfying_markers: &["@given", "hypothesis"],
        }),
    ])
}
