//! CFML error-handling rules: typed throws (`CF-ERR-1.1`) and no swallowed
//! catch (`CF-ERR-2.1`).

use std::fmt;

use enforcer_core::error::DecodeError;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{finding, first_line_containing, FindingSpec};

/// `CF-ERR-1.1` -- typed throw: `throw(...)` must carry a namespaced
/// `type=` argument, not a bare `message=` throw.
pub struct TypedThrowValidator {
    rule_id: RuleId,
}

impl fmt::Debug for TypedThrowValidator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("TypedThrowValidator").finish()
    }
}

impl TypedThrowValidator {
    /// Creates the validator with its fixed, registered CFML rule identifier.
    ///
    /// Invalid-input proof is kept in `tests/err_rules.rs`, alongside the
    /// behavioral pass/fail cases for this validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-ERR-1.1".parse()?,
        })
    }
}

impl Validator for TypedThrowValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        for (idx, line) in input.source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("throw(") && !trimmed.contains("type=") {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "throw() with no typed/namespaced type= argument",
                    },
                    // ALLOC-JUSTIFICATION: a finding owns its explanatory detail after this
                    // borrowed source line is no longer available to the caller.
                    format!(
                        "`{trimmed}` throws with a bare `message=` argument -- throw a typed/\
                         namespaced error instead (`throw(type=\"app.validation.invalidOrder\", \
                         message=\"...\");`)."
                    ),
                    &input,
                    // CAST-JUSTIFICATION: enumerate indices are non-negative and saturating
                    // conversion preserves the one-based source-line contract on huge inputs.
                    (idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// `CF-ERR-2.1` -- empty catch swallow: an empty `catch(...) {}` block, or
/// one that unconditionally `return true`s, is a violation.
pub struct EmptyCatchSwallowValidator {
    rule_id: RuleId,
}

impl fmt::Debug for EmptyCatchSwallowValidator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("EmptyCatchSwallowValidator").finish()
    }
}

impl EmptyCatchSwallowValidator {
    /// Creates the validator with its fixed, registered CFML rule identifier.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-ERR-2.1".parse()?,
        })
    }
}

impl Validator for EmptyCatchSwallowValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        for (idx, line) in input.source.lines().enumerate() {
            let trimmed = line.trim();
            let is_empty_catch =
                trimmed.starts_with("catch(") && trimmed.trim_end().ends_with("{}");
            if is_empty_catch {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "empty catch block swallows the exception",
                    },
                    // ALLOC-JUSTIFICATION: `Finding` owns a human-readable remediation after
                    // validation returns.
                    "An empty `catch(...) {}` block swallows the exception -- log it and \
                     `rethrow`, or handle it explicitly, at a boundary."
                        .to_owned(),
                    &input,
                    // CAST-JUSTIFICATION: enumerate indices are non-negative and saturating
                    // conversion preserves the one-based source-line contract on huge inputs.
                    (idx as u32).saturating_add(1),
                )];
            }
        }
        if let Some(line) = first_line_containing(input.source, "catch(") {
            let Some((_, after_catch)) = input.source.split_once("catch(") else {
                return Vec::new();
            };
            let block_swallows =
                after_catch.contains("return true;") && !after_catch.contains("rethrow");
            if block_swallows {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "catch block swallows the exception via return true",
                    },
                    // ALLOC-JUSTIFICATION: `Finding` owns a human-readable remediation after
                    // validation returns.
                    "A `catch(...)` block unconditionally `return true`s instead of logging/\
                     rethrowing -- never swallow an exception silently."
                        .to_owned(),
                    &input,
                    line,
                )];
            }
        }
        Vec::new()
    }
}

/// Build every validator this module registers.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(TypedThrowValidator::new()?),
        Box::new(EmptyCatchSwallowValidator::new()?),
    ])
}
