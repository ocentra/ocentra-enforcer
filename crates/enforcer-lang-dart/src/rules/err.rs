//! Dart error-handling rules `DART-ERR-1.1` / `DART-ERR-2.1` (both
//! scored): typed sealed `Failure` hierarchy instead of raw
//! `throw Exception('msg')`, and never rendering a raw exception to the
//! user.

use enforcer_core::error::DecodeError;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{finding, first_line_containing, FindingSpec};

/// `DART-ERR-1.1` (scored) — raw `throw Exception('msg')` instead of a
/// typed sealed `Failure` subtype.
pub struct RawExceptionThrowValidator {
    rule_id: RuleId,
}

impl RawExceptionThrowValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-ERR-1.1".parse()?,
        })
    }
}

impl Validator for RawExceptionThrowValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(input.source, "throw Exception(") else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "raw Exception thrown instead of a typed Failure (scored)",
            },
            "`throw Exception('...')` throws Dart's untyped built-in `Exception` — throw a \
             typed sealed `Failure` subtype (e.g. `ServerFailure`) instead."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// `DART-ERR-2.1` (scored) — never render a raw caught exception/error
/// object directly to the user (`Text('$error')`-shaped interpolation).
pub struct ExceptionRenderedToUserValidator {
    rule_id: RuleId,
}

impl ExceptionRenderedToUserValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-ERR-2.1".parse()?,
        })
    }
}

impl Validator for ExceptionRenderedToUserValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(input.source, "Text('$error')") else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "raw exception rendered to the user (scored)",
            },
            "a raw error/exception object is interpolated straight into a `Text(...)` widget — \
             show a generic user-facing message and log the raw error separately."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// Build every validator this module registers.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(RawExceptionThrowValidator::new()?),
        Box::new(ExceptionRenderedToUserValidator::new()?),
    ])
}
