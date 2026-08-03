//! Canonical values retained after harness adapters decode tool output.
//!
//! Raw JSON, stdout/stderr, and command-line spellings remain in
//! `enforcer-harness` adapter DTOs. These values are the validated runtime
//! representation shared by storage, query, and UI consumers.

use crate::boundary::decode_error::DecodeError;

macro_rules! harness_text_target {
    () => {
        str
    };
}

fn owned_harness_text(value: &str) -> String {
    // ALLOC-JUSTIFICATION: canonical harness text owns adapter data beyond the borrowed wire buffer.
    value.to_owned()
}

macro_rules! harness_text {
    ($(#[$doc:meta])* $name:ident, $field:literal) => {
        $(#[$doc])*
        #[derive(
            Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord,
            serde::Serialize, serde::Deserialize,
        )]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// Validate non-empty, control-free harness text.
            pub fn try_new(value: String) -> Result<Self, DecodeError> {
                if value.trim().is_empty() || value.chars().any(char::is_control) {
                    return Err(DecodeError::new(
                        $field,
                        "invalid harness text: must be non-empty text without control characters",
                    ));
                }
                Ok(Self(value))
            }

            /// View the validated text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = DecodeError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::try_new(value)
            }
        }

        impl std::str::FromStr for $name {
            type Err = DecodeError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::try_new(owned_harness_text(value))
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl std::ops::Deref for $name {
            type Target = harness_text_target!();

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

harness_text!(
    /// Identifier of one persisted harness run.
    HarnessRunId,
    "harnessRunId"
);
harness_text!(
    /// Native tool name retained after command decoding.
    HarnessToolName,
    "harnessToolName"
);
harness_text!(
    /// External diagnostic rule label; tool vocabularies are intentionally open.
    HarnessExternalRuleId,
    "harnessExternalRuleId"
);
harness_text!(
    /// External engine severity label retained until the harness normalizes it.
    HarnessExternalSeverity,
    "harnessExternalSeverity"
);
harness_text!(
    /// Optional external threat taxonomy identifier such as CWE or ATT&CK.
    HarnessThreatId,
    "harnessThreatId"
);
harness_text!(
    /// Persisted seed that reproduces one recorded property or fuzz failure.
    HarnessReproductionSeed,
    "harnessReproductionSeed"
);
harness_text!(
    /// Validated label identifying one recorded observability event.
    HarnessEventLabel,
    "harnessEventLabel"
);
harness_text!(
    /// Redacted diagnostic detail retained after adapter decoding.
    HarnessDiagnosticMessage,
    "harnessDiagnosticMessage"
);
harness_text!(
    /// Stable fingerprint assigned after diagnostic deduplication.
    HarnessDiagnosticFingerprint,
    "harnessDiagnosticFingerprint"
);
harness_text!(
    /// Normalized CI/local parity step name.
    HarnessStepName,
    "harnessStepName"
);
harness_text!(
    /// Tool, action, or toolchain version compared by the parity harness.
    HarnessStepVersion,
    "harnessStepVersion"
);
harness_text!(
    /// RFC3339 or legacy epoch timestamp retained in run storage.
    HarnessTimestamp,
    "harnessTimestamp"
);
harness_text!(
    /// Command argument retained for run audit output.
    HarnessCommandArgument,
    "harnessCommandArgument"
);
harness_text!(
    /// Optional package name retained by the run store.
    HarnessPackageName,
    "harnessPackageName"
);
harness_text!(
    /// Optional logical domain label retained by the run store.
    HarnessDomainName,
    "harnessDomainName"
);
harness_text!(
    /// Query/storage tag attached to a harness run.
    HarnessTag,
    "harnessTag"
);
harness_text!(
    /// Relative run-store artifact that is missing from an incomplete layout.
    HarnessRunFile,
    "harnessRunFile"
);
harness_text!(
    /// Normalized path label reported by a tool; may be absolute before repo relativization.
    HarnessDiagnosticPath,
    "harnessDiagnosticPath"
);

impl HarnessRunId {
    /// Normalize untrusted adapter text into a total runtime identifier.
    #[must_use]
    pub fn from_adapter(value: &str) -> Self {
        Self::try_new(owned_harness_text(value))
            .unwrap_or_else(|_| Self(owned_harness_text("unknown-run")))
    }
}

impl HarnessToolName {
    /// Normalize untrusted adapter text into a total runtime tool label.
    #[must_use]
    pub fn from_adapter(value: &str) -> Self {
        Self::try_new(owned_harness_text(value))
            .unwrap_or_else(|_| Self(owned_harness_text("unknown-tool")))
    }
}

impl HarnessExternalRuleId {
    /// Normalize an open external rule vocabulary without accepting blanks.
    #[must_use]
    pub fn from_adapter(value: &str) -> Self {
        Self::try_new(owned_harness_text(value))
            .unwrap_or_else(|_| Self(owned_harness_text("unknown-rule")))
    }
}

impl HarnessExternalSeverity {
    /// Normalize an open external severity vocabulary without accepting blanks.
    #[must_use]
    pub fn from_adapter(value: &str) -> Self {
        Self::try_new(owned_harness_text(value))
            .unwrap_or_else(|_| Self(owned_harness_text("unknown")))
    }
}

impl HarnessThreatId {
    /// Normalize an optional external threat identifier without accepting blanks.
    #[must_use]
    pub fn from_adapter(value: &str) -> Self {
        Self::try_new(owned_harness_text(value))
            .unwrap_or_else(|_| Self(owned_harness_text("unknown-threat")))
    }
}

impl HarnessReproductionSeed {
    /// Validate a seed decoded from a recorded engine report.
    pub fn from_adapter(value: String) -> Result<Self, DecodeError> {
        Self::try_new(value)
    }
}

impl HarnessEventLabel {
    /// Validate an event label decoded from a recorded observability report.
    pub fn from_adapter(value: String) -> Result<Self, DecodeError> {
        Self::try_new(value)
    }
}

impl HarnessDiagnosticMessage {
    /// Normalize untrusted diagnostic text without accepting blank messages.
    #[must_use]
    pub fn from_adapter(value: &str) -> Self {
        Self::try_new(owned_harness_text(value))
            .unwrap_or_else(|_| Self(owned_harness_text("No diagnostic detail supplied.")))
    }
}

impl HarnessDiagnosticPath {
    /// Normalize path separators and provide a stable unknown location.
    #[must_use]
    pub fn from_adapter(value: &str) -> Self {
        let normalized = value.replace('\\', "/");
        Self::try_new(normalized).unwrap_or_else(|_| Self(owned_harness_text(".")))
    }
}

impl HarnessDiagnosticFingerprint {
    /// Retain the non-empty base64url digest emitted by the harness hasher.
    #[must_use]
    pub fn from_digest(value: String) -> Self {
        Self(value)
    }
}

impl HarnessStepName {
    /// Normalize a manifest step name after trimming.
    #[must_use]
    pub fn from_manifest(value: &str) -> Self {
        match Self::try_new(owned_harness_text(value.trim())) {
            Ok(name) => name,
            Err(_) => Self(owned_harness_text("unnamed-step")),
        }
    }
}

impl HarnessStepVersion {
    /// Normalize an optional manifest version after trimming.
    #[must_use]
    pub fn from_manifest(value: &str) -> Option<Self> {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
            None
        } else {
            Some(Self(owned_harness_text(trimmed)))
        }
    }
}

// BRAND-INVARIANT: every byte of captured process output is valid, including empty text.
/// Captured stdout or stderr retained at the process adapter seam.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HarnessCapturedOutput(Box<str>);

impl HarnessCapturedOutput {
    /// Retain captured process text. Empty output is valid.
    #[must_use]
    pub fn from_owned(value: String) -> Self {
        Self(value.into_boxed_str())
    }

    /// View captured text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether a run is protected from ordinary retention.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HarnessPinned {
    Pinned,
    #[default]
    Unpinned,
}

impl HarnessPinned {
    /// Stable wire representation.
    #[must_use]
    pub const fn as_bool(self) -> bool {
        matches!(self, Self::Pinned)
    }
}

/// Availability state of the optional DuckDB projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarnessDuckDbAvailability {
    Available,
    Deferred,
}

/// Store mode advertised by the deferred DuckDB projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarnessDuckDbMode {
    Optional,
}

impl HarnessDuckDbMode {
    /// Stable wire representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Optional => "optional",
        }
    }
}

impl HarnessDuckDbAvailability {
    /// Stable wire representation.
    #[must_use]
    pub const fn as_bool(self) -> bool {
        matches!(self, Self::Available)
    }
}

/// Closed language classification retained after tool-name inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HarnessLanguage {
    Rust,
    Typescript,
    Python,
    Common,
}

impl HarnessLanguage {
    /// Stable storage spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Typescript => "typescript",
            Self::Python => "python",
            Self::Common => "common",
        }
    }
}

impl std::fmt::Display for HarnessLanguage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

// BRAND-INVARIANT: zero is normalized at the adapter seam; oversized values remain explicit.
/// One-based source line reported by an external tool.
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
// BRAND-INVARIANT: zero is normalized at the adapter seam; oversized values remain explicit.
/// One-based source location retained from an external tool.
pub struct HarnessSourceLine(u64);

impl std::fmt::Debug for HarnessSourceLine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("HarnessSourceLine")
            .field(&self.0)
            .finish()
    }
}

impl HarnessSourceLine {
    /// Validate a one-based external source line without narrowing it.
    pub fn try_new(value: u64) -> Result<Self, DecodeError> {
        if value == 0 {
            return Err(DecodeError::new("harnessSourceLine", "must be one-based"));
        }
        Ok(Self(value))
    }

    /// Normalize a missing/zero external line to the first source line.
    #[must_use]
    pub const fn from_external(value: u64) -> Self {
        Self(if value == 0 { 1 } else { value })
    }

    /// Return the stable wire value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Convert to the narrower finding-line domain.
    ///
    /// Oversized external locations remain explicit as `None`; they are not
    /// silently clamped to a different source line.
    #[must_use]
    pub fn finding_line(self) -> Option<std::num::NonZeroU32> {
        match u32::try_from(self.0) {
            Ok(value) => std::num::NonZeroU32::new(value),
            Err(_) => None,
        }
    }
}

impl std::fmt::Display for HarnessSourceLine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

impl PartialEq<u64> for HarnessSourceLine {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

/// Closed run result stored in summaries and events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarnessRunStatus {
    Passed,
    Failed,
}

/// Closed set of persisted harness artifacts exposed by the query API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarnessArtifactKind {
    Stdout,
    Stderr,
    Diagnostics,
    Events,
}

impl HarnessArtifactKind {
    /// Stable storage key used in a run summary.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
            Self::Diagnostics => "diagnostics",
            Self::Events => "events",
        }
    }
}

impl HarnessRunStatus {
    /// Derive the run status from a process exit code.
    #[must_use]
    pub const fn from_exit_code(code: crate::telemetry_types::ProcessExitCode) -> Self {
        if code.get() == 0 {
            Self::Passed
        } else {
            Self::Failed
        }
    }

    /// Stable storage spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }
}

/// Policy class assigned to an allowlisted external tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HarnessToolRequirement {
    /// A non-available tool prevents the policy gate from passing.
    Required,
    /// A non-available tool is reported as a warning.
    Optional,
    /// A non-available tool is explicitly outside the current run.
    Advisory,
}

impl HarnessToolRequirement {
    /// Stable policy spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Optional => "optional",
            Self::Advisory => "advisory",
        }
    }
}

/// Typed availability result for an allowlisted tool invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HarnessToolAvailability {
    Available,
    Missing,
    VersionMismatch,
    Misconfigured,
    TimedOut,
    Failed,
    MalformedOutput,
}

impl HarnessToolAvailability {
    /// Stable availability spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Missing => "missing",
            Self::VersionMismatch => "version-mismatch",
            Self::Misconfigured => "misconfigured",
            Self::TimedOut => "timed-out",
            Self::Failed => "failed",
            Self::MalformedOutput => "malformed-output",
        }
    }

    /// Derive the non-ambiguous policy action for this availability result.
    #[must_use]
    pub const fn decision(self, requirement: HarnessToolRequirement) -> HarnessToolDecision {
        match (self, requirement) {
            (Self::Available, _) => HarnessToolDecision::Run,
            (_, HarnessToolRequirement::Required) => HarnessToolDecision::Block,
            (_, HarnessToolRequirement::Optional) => HarnessToolDecision::Warn,
            (_, HarnessToolRequirement::Advisory) => HarnessToolDecision::NotApplicable,
        }
    }
}

/// Action a caller must take after resolving allowlisted-tool availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HarnessToolDecision {
    Run,
    Block,
    Warn,
    NotApplicable,
}

/// Closed execution limits required before an allowlisted tool may run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HarnessExecutionLimits {
    max_wall_time_ms: std::num::NonZeroU64,
    max_output_bytes: std::num::NonZeroU64,
    max_files: std::num::NonZeroU32,
}

impl HarnessExecutionLimits {
    /// Construct non-zero wall-time, output, and file-count bounds.
    pub fn try_new(
        max_wall_time_ms: u64,
        max_output_bytes: u64,
        max_files: u32,
    ) -> Result<Self, DecodeError> {
        let max_wall_time_ms = std::num::NonZeroU64::new(max_wall_time_ms)
            .ok_or_else(|| DecodeError::new("maxWallTimeMs", "must be greater than zero"))?;
        let max_output_bytes = std::num::NonZeroU64::new(max_output_bytes)
            .ok_or_else(|| DecodeError::new("maxOutputBytes", "must be greater than zero"))?;
        let max_files = std::num::NonZeroU32::new(max_files)
            .ok_or_else(|| DecodeError::new("maxFiles", "must be greater than zero"))?;
        Ok(Self {
            max_wall_time_ms,
            max_output_bytes,
            max_files,
        })
    }

    /// Maximum wall time in milliseconds.
    #[must_use]
    pub const fn max_wall_time_ms(self) -> u64 {
        self.max_wall_time_ms.get()
    }

    /// Maximum combined captured output in bytes.
    #[must_use]
    pub const fn max_output_bytes(self) -> u64 {
        self.max_output_bytes.get()
    }

    /// Maximum file count a later adapter may inspect.
    #[must_use]
    pub const fn max_files(self) -> u32 {
        self.max_files.get()
    }
}

/// Reviewed command template and policy for one allowlisted tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessToolSpec {
    tool: HarnessToolName,
    command: Vec<HarnessCommandArgument>,
    requirement: HarnessToolRequirement,
    limits: HarnessExecutionLimits,
    expected_version: Option<HarnessStepVersion>,
}

impl HarnessToolSpec {
    /// Construct a spec with a non-empty executable-and-argument template.
    pub fn try_new(
        tool: HarnessToolName,
        command: Vec<HarnessCommandArgument>,
        requirement: HarnessToolRequirement,
        limits: HarnessExecutionLimits,
        expected_version: Option<HarnessStepVersion>,
    ) -> Result<Self, DecodeError> {
        if command.is_empty() {
            return Err(DecodeError::new(
                "command",
                "allowlisted tool command must not be empty",
            ));
        }
        Ok(Self {
            tool,
            command,
            requirement,
            limits,
            expected_version,
        })
    }

    /// Reviewed tool identity.
    #[must_use]
    pub const fn tool(&self) -> &HarnessToolName {
        &self.tool
    }

    /// Reviewed executable-and-argument template.
    #[must_use]
    pub fn command(&self) -> &[HarnessCommandArgument] {
        &self.command
    }

    /// Required/optional/advisory policy class.
    #[must_use]
    pub const fn requirement(&self) -> HarnessToolRequirement {
        self.requirement
    }

    /// Bounded execution limits.
    #[must_use]
    pub const fn limits(&self) -> HarnessExecutionLimits {
        self.limits
    }

    /// Optional version expectation used by a later availability probe.
    #[must_use]
    pub const fn expected_version(&self) -> Option<&HarnessStepVersion> {
        self.expected_version.as_ref()
    }
}
