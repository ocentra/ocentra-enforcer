//! Crate-local error type for the fixture/parity harness.

use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::telemetry_types::FindingCount;

#[derive(Debug, thiserror::Error)]
#[doc = "Failures raised by fixture I/O or validator fail/pass contract violations."]
pub enum HarnessError {
    /// A fixture file could not be read.
    #[error("failed to read fixture `{path}`: {source}")]
    FixtureRead {
        /// Path to the fixture that failed to read.
        path: RelPath,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The validator did not fire on its fail fixture (silent when it must
    /// speak).
    #[error(
        "validator `{rule_id}` did not fire on fail fixture `{fixture}` — expected at least one finding"
    )]
    DidNotFireOnFail {
        /// The rule under test.
        rule_id: RuleId,
        /// The fail-fixture path that should have tripped it.
        fixture: RelPath,
    },

    /// A validator emitted a finding for a different rule.
    #[error(
        "validator `{expected_rule_id}` emitted `{actual_rule_id}` on fail fixture `{fixture}`"
    )]
    MismatchedRule {
        /// The rule owned by the validator.
        expected_rule_id: RuleId,
        /// The rule carried by the unexpected finding.
        actual_rule_id: RuleId,
        /// The fail fixture that produced the mismatch.
        fixture: RelPath,
    },

    /// The validator fired on its pass fixture (speaks when it must stay
    /// silent).
    #[error(
        "validator `{rule_id}` fired on pass fixture `{fixture}` — expected zero findings, got {finding_count}"
    )]
    FiredOnPass {
        /// The rule under test.
        rule_id: RuleId,
        /// The pass-fixture path that should have stayed clean.
        fixture: RelPath,
        /// How many findings the validator actually produced.
        finding_count: FindingCount,
    },

    /// A platform collection length could not be represented in durable
    /// finding-count telemetry.
    #[error("finding count for pass fixture `{fixture}` exceeds the supported telemetry range")]
    FindingCountOverflow {
        /// The pass fixture whose emitted finding collection was too large.
        fixture: RelPath,
        /// Platform integer conversion failure.
        #[source]
        source: std::num::TryFromIntError,
    },
}

/// Convenience alias for harness-fallible operations.
pub type HarnessResult<T> = Result<T, HarnessError>;
