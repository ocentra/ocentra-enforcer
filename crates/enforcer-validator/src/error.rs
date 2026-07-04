//! Crate-local error type for the fixture/parity harness.

/// Failures the harness can raise: I/O reading a fixture, or a validator
/// that does not behave per the fail/pass contract.
#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    /// A fixture file could not be read.
    #[error("failed to read fixture `{path}`: {source}")]
    FixtureRead {
        /// Path to the fixture that failed to read.
        path: String,
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
        rule_id: String,
        /// The fail-fixture path that should have tripped it.
        fixture: String,
    },

    /// The validator fired on its pass fixture (speaks when it must stay
    /// silent).
    #[error(
        "validator `{rule_id}` fired on pass fixture `{fixture}` — expected zero findings, got {finding_count}"
    )]
    FiredOnPass {
        /// The rule under test.
        rule_id: String,
        /// The pass-fixture path that should have stayed clean.
        fixture: String,
        /// How many findings the validator actually produced.
        finding_count: usize,
    },
}

/// Convenience alias for harness-fallible operations.
pub type HarnessResult<T> = Result<T, HarnessError>;
