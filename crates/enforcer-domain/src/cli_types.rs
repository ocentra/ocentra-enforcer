//! Canonical semantic values shared by CLI lifecycle and command routing.

/// Lifecycle phase selected by a CLI invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for Phase."]
pub enum Phase {
    Plan,
    Implement,
    Check,
    Fix,
    Review,
}

impl Phase {
    #[doc = "The command_name operation for this canonical domain value."]
    pub fn command_name(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Implement => "implement",
            Self::Check => "check",
            Self::Fix => "fix",
            Self::Review => "review",
        }
    }
}

/// Typed reason a lifecycle oracle cannot pass.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for LifecycleReasonText."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct LifecycleReasonText(String);

impl LifecycleReasonText {
    /// Validate lifecycle diagnostic text; invalid blank or NUL-bearing input is rejected.
    pub fn try_new(value: String) -> Result<Self, crate::boundary::decode_error::DecodeError> {
        (!value.trim().is_empty() && !value.contains('\0'))
            .then_some(Self(value))
            .ok_or_else(|| {
                crate::boundary::decode_error::DecodeError::new(
                    "lifecycleReason",
                    "must be non-empty and contain no NUL",
                )
            })
    }
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for LifecycleFailReason."]
pub enum LifecycleFailReason {
    OracleFindings(LifecycleReasonText),
    NotYetWired(LifecycleReasonText),
    Internal(LifecycleReasonText),
}

/// Result of one lifecycle oracle.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for PhaseVerdict."]
pub enum PhaseVerdict {
    Pass,
    Fail(LifecycleFailReason),
}

/// Explicit filesystem paths selected for one CLI check phase.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for CliSelectedPath."]
#[doc = "BRAND-INVARIANT: validated non-empty filesystem path; raw storage remains private."]
pub struct CliSelectedPath(std::path::PathBuf);

impl CliSelectedPath {
    #[doc = "The new operation for this canonical domain value."]
    pub fn new(
        value: std::path::PathBuf,
    ) -> Result<Self, crate::boundary::decode_error::DecodeError> {
        (!value.as_os_str().is_empty())
            .then_some(Self(value))
            .ok_or_else(|| {
                crate::boundary::decode_error::DecodeError::new(
                    "cliSelectedPath",
                    "must not be empty",
                )
            })
    }
    #[doc = "The as_path operation for this canonical domain value."]
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

#[derive(Debug, Clone, Default)]
#[doc = "Canonical domain representation for CheckScope."]
pub struct CheckScope {
    paths: Vec<CliSelectedPath>,
}

impl CheckScope {
    #[doc = "The new operation for this canonical domain value."]
    pub fn new(paths: Vec<CliSelectedPath>) -> Self {
        Self { paths }
    }
    #[doc = "The paths operation for this canonical domain value."]
    pub fn paths(&self) -> &[CliSelectedPath] {
        &self.paths
    }
}
