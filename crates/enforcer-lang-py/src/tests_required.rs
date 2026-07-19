//! `python/tests` validator: PY-6.8 / PY-6.9 / PY-6.10 (3 rules) --
//! "required coverage SHAPE" rules. Each fires when a test file defines a
//! function whose name signals a behavior category (`test_*_rejects_*` /
//! validators, functions calling something that raises, parser/normalizer
//! functions) but the file never demonstrates the matching test pattern
//! (`pytest.raises`, an explicit invalid-input assertion, or a
//! `@given`/Hypothesis property test).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::{Finding, FindingTitle};
use enforcer_domain::ids::{BuiltInPythonRule, RuleId};
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::finding::{static_title, PythonFindingMessage};
use crate::boundary::source::PythonMarkers;

/// Fires when `trigger_markers` are present (the file exercises the
/// behavior category this rule cares about) but none of
/// `satisfying_markers` are present (the file never proves the required
/// coverage shape for that category).
struct RequiredCoverageValidator {
    rule_id: RuleId,
    title: FindingTitle,
    trigger_markers: PythonMarkers,
    satisfying_markers: PythonMarkers,
}

impl Validator for RequiredCoverageValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let has_trigger = self.trigger_markers.any_in(input.source);
        if !has_trigger {
            return Vec::new();
        }
        let has_coverage = self.satisfying_markers.any_in(input.source);
        if has_coverage {
            return Vec::new();
        }
        crate::boundary::finding::from_python_source(
            &self.rule_id,
            Severity::Error,
            input.file,
            1,
            PythonFindingMessage::new(
                self.title.as_str(),
                "required test-coverage shape is missing",
                None,
            ),
        )
        .into_iter()
        .collect()
    }
}

/// Build every `python/tests`-keyed validator this crate registers.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(RequiredCoverageValidator {
            rule_id: BuiltInPythonRule::Py6Rule8.id(),
            title: static_title("Python validators require invalid-input tests")?,
            trigger_markers: PythonMarkers::new(&["def validate_", "def parse_"]),
            satisfying_markers: PythonMarkers::new(&["invalid", "pytest.raises", "rejects"]),
        }),
        Box::new(RequiredCoverageValidator {
            rule_id: BuiltInPythonRule::Py6Rule9.id(),
            title: static_title("Python exception paths require tests")?,
            trigger_markers: PythonMarkers::new(&["raise "]),
            satisfying_markers: PythonMarkers::new(&["pytest.raises"]),
        }),
        Box::new(RequiredCoverageValidator {
            rule_id: BuiltInPythonRule::Py6Rule10.id(),
            title: static_title("Python parsers and normalizers require property tests")?,
            trigger_markers: PythonMarkers::new(&["def parse_", "def normalize_"]),
            satisfying_markers: PythonMarkers::new(&["@given", "hypothesis"]),
        }),
    ])
}
