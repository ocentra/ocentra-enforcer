//! Raw toolchain-policy spellings and source predicates.
//!
//! BOUNDARY-INVARIANT: parse raw configuration text into a closed
//! toolchain-rule classification before findings are emitted.
//! boundaryOwnerNote: enforcer-lang-ts owns toolchain input classification.
//! Negative invalid and malformed configuration coverage is fixture-backed.

#[derive(Debug, Clone, Copy)]
/// Closed set of TypeScript toolchain policies.
pub(crate) enum ToolchainRule {
    Ts5_1,
    Ts7_1,
    Ts7_12,
    Ts7_13,
}

impl ToolchainRule {
    /// Static rule identifier spelling.
    pub(crate) const fn rule_id(self) -> &'static str {
        match self {
            Self::Ts5_1 => "TS-5.1",
            Self::Ts7_1 => "TS-7.1",
            Self::Ts7_12 => "TS-7.12",
            Self::Ts7_13 => "TS-7.13",
        }
    }

    /// Human-readable policy title.
    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::Ts5_1 => "TypeScript compiler checks must pass",
            Self::Ts7_1 => "TypeScript strict mode is required",
            Self::Ts7_12 => "npm ci is required in CI",
            Self::Ts7_13 => "ESLint must enforce high-risk TypeScript rules",
        }
    }

    /// Decide whether this policy fires for raw configuration source.
    pub(crate) fn fires(self, source: &str) -> bool {
        match self {
            Self::Ts5_1 => !source.contains("tsc --noEmit"),
            Self::Ts7_1 => source.contains("strict: false"),
            Self::Ts7_12 => !source.contains("npm ci"),
            Self::Ts7_13 => !source.contains("no-floating-promises"),
        }
    }
}
