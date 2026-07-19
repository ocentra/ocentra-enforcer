//! Canonical typed values carried by proof definitions, proof runs, and
//! proof-claim records.
//!
//! The proof harness parses these values at its JSON/process boundaries. Once
//! constructed, cross-crate APIs carry the semantic type instead of an
//! ambiguous `String`.

use crate::boundary::decode_error::DecodeError;

macro_rules! proof_string {
    ($(#[$doc:meta])* $name:ident, $field:literal, $validate:ident) => {
        $(#[$doc])*
        #[derive(
            Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord,
            serde::Serialize, serde::Deserialize,
        )]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// View the validated value.
            pub fn as_str(&self) -> &str { &self.0 }
        }

        impl TryFrom<String> for $name {
            type Error = DecodeError;
            fn try_from(value: String) -> Result<Self, DecodeError> {
                $validate(&value)?;
                Ok(Self(value))
            }
        }

        impl std::str::FromStr for $name {
            type Err = DecodeError;
            fn from_str(value: &str) -> Result<Self, DecodeError> {
                // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
                Self::try_from(value.to_owned())
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self { value.0 }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

fn validate_nonempty(value: &str, field: &str) -> Result<(), DecodeError> {
    if value.trim().is_empty() {
        Err(DecodeError::new(field, "must not be empty"))
    } else {
        Ok(())
    }
}

fn validate_proof_identifier(value: &str, field: &str) -> Result<(), DecodeError> {
    let valid = !value.is_empty()
        && value.len() <= 256
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_' | ':' | '/')
        });
    if valid {
        Ok(())
    } else {
        Err(DecodeError::new(
            field,
            "expected 1..=256 ASCII alphanumeric, dot, dash, underscore, colon, or slash characters",
        ))
    }
}

fn validate_proof_id(value: &str) -> Result<(), DecodeError> {
    validate_proof_identifier(value, "proofId")
}

fn validate_proof_run_id(value: &str) -> Result<(), DecodeError> {
    validate_proof_identifier(value, "proofRunId")
}

fn validate_claim_id(value: &str) -> Result<(), DecodeError> {
    validate_proof_identifier(value, "claimId")
}

fn validate_proof_family(value: &str) -> Result<(), DecodeError> {
    validate_nonempty(value, "proofFamily")
}

fn validate_proof_capability(value: &str) -> Result<(), DecodeError> {
    validate_nonempty(value, "proofCapability")
}

fn validate_proof_collector(value: &str) -> Result<(), DecodeError> {
    validate_nonempty(value, "proofCollector")
}

fn validate_journal_event_type(value: &str) -> Result<(), DecodeError> {
    validate_nonempty(value, "journalEventType")
}

fn validate_git_commit(value: &str) -> Result<(), DecodeError> {
    let valid = value.len() >= 7
        && value.len() <= 64
        && value.chars().all(|character| character.is_ascii_hexdigit());
    if valid {
        Ok(())
    } else {
        Err(DecodeError::new(
            "gitCommit",
            "expected a 7..=64 character hexadecimal Git object id",
        ))
    }
}

fn validate_git_ref_name(value: &str) -> Result<(), DecodeError> {
    validate_nonempty(value, "gitRefName")
}

proof_string!(
    /// Stable proof definition identity.
    ProofId,
    "proofId",
    validate_proof_id
);

proof_string!(
    /// Unique execution identity for one proof invocation.
    ProofRunId,
    "proofRunId",
    validate_proof_run_id
);

proof_string!(
    /// Identity of a PR-ready or completion claim.
    ClaimId,
    "claimId",
    validate_claim_id
);

proof_string!(
    /// Registry routing family for a proof definition.
    ProofFamily,
    "proofFamily",
    validate_proof_family
);

proof_string!(
    /// Execution capability required by a proof, such as `ci` or `local`.
    ProofCapability,
    "proofCapability",
    validate_proof_capability
);

proof_string!(
    /// Collector mechanism declared by a proof definition.
    ProofCollector,
    "proofCollector",
    validate_proof_collector
);

proof_string!(
    /// Extensible lifecycle event name recorded in the proof journal.
    JournalEventType,
    "journalEventType",
    validate_journal_event_type
);

proof_string!(
    /// Resolved Git commit object identifier captured with a proof run.
    GitCommit,
    "gitCommit",
    validate_git_commit
);

proof_string!(
    /// Resolved Git branch or detached-head reference captured with a proof run.
    GitRefName,
    "gitRefName",
    validate_git_ref_name
);

/// Terminal state of one proof run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for ProofStatus."]
pub enum ProofStatus {
    Passed,
    Failed,
    ManualRequired,
    Unavailable,
}

impl ProofStatus {
    /// Whether this state blocks a proof claim.
    pub fn is_failure(self) -> bool {
        matches!(
            self,
            Self::Failed | Self::ManualRequired | Self::Unavailable
        )
    }
}

/// Machine-readable reason a proof claim is rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for ClaimViolationCode."]
pub enum ClaimViolationCode {
    MissingProofRun,
    ProofNotPassed,
    StaleCommit,
    DirtyWorktree,
    MissingArtifact,
    DeletedRequiredPath,
}

/// Integrity state of a project's proof journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for JournalState."]
pub enum JournalState {
    Missing,
    Verified,
    Invalid,
}

/// Parity classification between imported legacy evidence and a proof run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for ProofCoverage."]
pub enum ProofCoverage {
    Equivalent,
    Weaker,
    NotComparable,
}

/// Inspection outcome for an imported legacy artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for LegacyArtifactStatus."]
pub enum LegacyArtifactStatus {
    Present,
    Passed,
    Failed,
}

/// Freshness of a stored proof run relative to the current Git revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for ProofFreshness."]
pub enum ProofFreshness {
    Current,
    Stale,
    Unavailable,
    Invalid,
}

/// Read-model state of a project's PR-ready proof claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for ProjectClaimState."]
pub enum ProjectClaimState {
    Unconfigured,
    InvalidRegistry,
    NoRequiredProofs,
    Ready,
    Blocked,
}

macro_rules! serde_unit_enum {
    ($name:ty, {$($variant:path => $wire:literal),+ $(,)?}) => {
        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let wire = match self {
                    $($variant => $wire),+
                };
                serializer.serialize_str(wire)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let wire = <String as serde::Deserialize>::deserialize(deserializer)?;
                match wire.as_str() {
                    $($wire => Ok($variant)),+,
                    _ => Err(serde::de::Error::unknown_variant(
                        &wire,
                        &[$($wire),+],
                    )),
                }
            }
        }
    };
}

serde_unit_enum!(ProofStatus, {
    ProofStatus::Passed => "passed",
    ProofStatus::Failed => "failed",
    ProofStatus::ManualRequired => "manual-required",
    ProofStatus::Unavailable => "unavailable",
});
serde_unit_enum!(ClaimViolationCode, {
    ClaimViolationCode::MissingProofRun => "missing-proof-run",
    ClaimViolationCode::ProofNotPassed => "proof-not-passed",
    ClaimViolationCode::StaleCommit => "stale-commit",
    ClaimViolationCode::DirtyWorktree => "dirty-worktree",
    ClaimViolationCode::MissingArtifact => "missing-artifact",
    ClaimViolationCode::DeletedRequiredPath => "deleted-required-path",
});
serde_unit_enum!(JournalState, {
    JournalState::Missing => "missing",
    JournalState::Verified => "verified",
    JournalState::Invalid => "invalid",
});
serde_unit_enum!(ProofCoverage, {
    ProofCoverage::Equivalent => "equivalent",
    ProofCoverage::Weaker => "weaker",
    ProofCoverage::NotComparable => "not-comparable",
});
serde_unit_enum!(LegacyArtifactStatus, {
    LegacyArtifactStatus::Present => "present",
    LegacyArtifactStatus::Passed => "passed",
    LegacyArtifactStatus::Failed => "failed",
});
serde_unit_enum!(ProofFreshness, {
    ProofFreshness::Current => "current",
    ProofFreshness::Stale => "stale",
    ProofFreshness::Unavailable => "unavailable",
    ProofFreshness::Invalid => "invalid",
});
serde_unit_enum!(ProjectClaimState, {
    ProjectClaimState::Unconfigured => "unconfigured",
    ProjectClaimState::InvalidRegistry => "invalid-registry",
    ProjectClaimState::NoRequiredProofs => "no-required-proofs",
    ProjectClaimState::Ready => "ready",
    ProjectClaimState::Blocked => "blocked",
});
