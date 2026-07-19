//! Crate-local error type for the scaffolder and the fail-closed parity
//! oracle.

/// Failures the scaffolder or the oracle can raise. Fail-closed: every
/// variant here means "this rule is NOT accepted", never a partial success.
#[derive(Debug, thiserror::Error)]
pub enum MechanizationError {
    /// A canonical domain value rejected raw input at a real boundary.
    #[error("invalid mechanization domain input: {0}")]
    Decode(#[from] enforcer_domain::boundary::decode_error::DecodeError),
    /// A [`crate::scaffold::ScaffoldSpec`] field was structurally empty or
    /// otherwise malformed before a [`enforcer_rules::registry::RuleRecord`]
    /// could even be built.
    #[error("scaffold spec rejected: {reason}")]
    InvalidSpec {
        /// Human-readable reason the spec was rejected.
        reason: enforcer_domain::rules_types::RuleFailureReason,
    },

    /// The rule record the scaffolder produced did not pass
    /// `enforcer_rules::registry::RuleRegistry` shape validation (should be
    /// unreachable given [`crate::scaffold::scaffold_rule`]'s own checks,
    /// but the oracle re-verifies rather than trusting the scaffolder).
    #[error("scaffolded record `{rule_id}` failed registry validation: {source}")]
    RecordRejected {
        /// The rule id under scaffold.
        rule_id: enforcer_domain::ids::RuleId,
        /// Underlying registry load error.
        #[source]
        source: enforcer_rules::RuleLoadError,
    },

    /// The oracle was not given a validator implementation at all —
    /// distinct from "validator exists but does not fire", this is "no
    /// validator to test". Fails closed the same as any other missing
    /// parity artifact.
    #[error("no validator supplied for rule `{rule_id}` — a rule record without a validator implementation is never accepted")]
    MissingValidator {
        /// The rule id under scaffold.
        rule_id: enforcer_domain::ids::RuleId,
    },

    /// The supplied validator belongs to a different canonical rule id.
    #[error("record declares rule `{declared}` but validator implements `{implemented}`")]
    ValidatorRuleMismatch {
        /// Rule declared by the candidate record.
        declared: enforcer_domain::ids::RuleId,
        /// Rule implemented by the supplied validator.
        implemented: enforcer_domain::ids::RuleId,
    },

    /// The fail-closed fixture/parity harness (`enforcer_validator::harness`)
    /// rejected the validator against its declared fixtures.
    #[error("rule `{rule_id}` failed the fixture/parity oracle: {source}")]
    ParityFailed {
        /// The rule id under scaffold.
        rule_id: enforcer_domain::ids::RuleId,
        /// Underlying harness failure (did-not-fire-on-fail / fired-on-pass
        /// / fixture-read).
        #[source]
        source: enforcer_validator::error::HarnessError,
    },
}

/// Convenience alias for mechanization-fallible operations.
pub type MechanizationResult<T> = Result<T, MechanizationError>;
