//! Canonical value types used by plan scaffolding, validation, and orchestration.

use crate::boundary::decode_error::DecodeError;
use std::path::{Path, PathBuf};

macro_rules! plan_text_type {
    ($name:ident, $field:literal, $maximum:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[doc = concat!("Canonical validated Plan text for ", $field, ".")]
        #[doc = "BRAND-INVARIANT: non-empty printable text is validated before private storage."]
        pub struct $name(String);

        impl $name {
            pub fn try_new(value: String) -> Result<Self, DecodeError> {
                let valid = !value.trim().is_empty()
                    && value.len() <= $maximum
                    && value.chars().all(|character| !character.is_control());
                valid
                    .then_some(Self(value))
                    .ok_or_else(|| DecodeError::new($field, "must be non-empty printable text"))
            }

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
                // ALLOC-JUSTIFICATION: the canonical Plan value owns boundary text beyond the caller lifetime.
                Self::try_new(value.to_owned())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

macro_rules! plan_count_type {
    ($name:ident, $primitive:ty, $field:literal) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(transparent)]
        #[doc = concat!("Canonical non-zero Plan count for ", $field, ".")]
        #[doc = "BRAND-INVARIANT: zero is rejected before private numeric storage."]
        pub struct $name($primitive);

        impl $name {
            pub fn try_new(value: $primitive) -> Result<Self, DecodeError> {
                (value > 0)
                    .then_some(Self(value))
                    .ok_or_else(|| DecodeError::new($field, "must be greater than zero"))
            }

            #[must_use]
            pub const fn get(self) -> $primitive {
                self.0
            }
        }

        impl TryFrom<$primitive> for $name {
            type Error = DecodeError;

            fn try_from(value: $primitive) -> Result<Self, Self::Error> {
                Self::try_new(value)
            }
        }
    };
}

macro_rules! plan_index_type {
    ($name:ident, $field:literal) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(transparent)]
        #[doc = concat!("Canonical zero-based Plan index for ", $field, ".")]
        #[doc = "BRAND-INVARIANT: zero-based index meaning is fixed by the owning ledger contract."]
        pub struct $name(PlanImportCount);

        impl From<PlanImportCount> for $name {
            fn from(value: PlanImportCount) -> Self {
                Self(value)
            }
        }

        impl From<$name> for PlanImportCount {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

macro_rules! lesson_text_type {
    ($name:ident, $field:literal, $validate:path) => {
        #[doc = "SERIALIZATION-DOC: serialized as the validated canonical string value."]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
        #[serde(transparent)]
        #[doc = concat!("Canonical validated lesson value for ", $field, ".")]
        #[doc = "BRAND-INVARIANT: validation occurs before private text storage."]
        pub struct $name(String);

        impl $name {
            pub fn try_new(value: String) -> Result<Self, DecodeError> {
                $validate(&value)?;
                Ok(Self(value))
            }

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
                // ALLOC-JUSTIFICATION: the canonical lesson value owns boundary text beyond the caller lifetime.
                Self::try_new(value.to_owned())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = <String as serde::Deserialize>::deserialize(deserializer)?;
                Self::try_new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

fn validate_lesson_id(value: &str) -> Result<(), DecodeError> {
    const EXPECTED: &str = "expected `L<number>[-SUFFIX]` (e.g. `L1`, `L26`, `L11-FILL`)";
    let valid = value
        .strip_prefix('L')
        .and_then(|rest| {
            let (number, suffix) = rest
                .split_once('-')
                .map_or((rest, None), |(number, suffix)| (number, Some(suffix)));
            let number_is_valid =
                !number.is_empty() && number.chars().all(|character| character.is_ascii_digit());
            let suffix_is_valid = suffix.is_none_or(|suffix| {
                !suffix.is_empty()
                    && suffix.split('-').all(|segment| {
                        !segment.is_empty()
                            && segment
                                .chars()
                                .all(|character| character.is_ascii_alphanumeric())
                    })
            });
            (number_is_valid && suffix_is_valid).then_some(())
        })
        .is_some();
    if valid {
        Ok(())
    } else {
        Err(DecodeError::new("lessonId", EXPECTED))
    }
}

fn validate_artifact_ref(value: &str) -> Result<(), DecodeError> {
    if !value.is_empty() && value.len() <= 512 {
        Ok(())
    } else {
        Err(DecodeError::new(
            "artifactRef",
            "expected a non-empty landed-artifact reference (path#anchor or path)",
        ))
    }
}

fn validate_captured_date(value: &str) -> Result<(), DecodeError> {
    let is_iso_date = value.len() == 10
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
        && value
            .chars()
            .enumerate()
            .all(|(index, character)| matches!(index, 4 | 7) || character.is_ascii_digit());
    if value.is_empty() || is_iso_date {
        Ok(())
    } else {
        Err(DecodeError::new(
            "capturedDate",
            "expected YYYY-MM-DD or an empty legacy date",
        ))
    }
}

fn validate_lesson_text(value: &str) -> Result<(), DecodeError> {
    if value.trim().is_empty() {
        Err(DecodeError::new("lessonText", "must not be empty"))
    } else {
        Ok(())
    }
}

fn validate_observed_evidence(_: &str) -> Result<(), DecodeError> {
    Ok(())
}

lesson_text_type!(LessonId, "lessonId", validate_lesson_id);
lesson_text_type!(ArtifactRef, "artifactRef", validate_artifact_ref);
lesson_text_type!(CapturedDate, "capturedDate", validate_captured_date);
lesson_text_type!(LessonText, "lessonText", validate_lesson_text);
lesson_text_type!(
    ObservedEvidence,
    "observedEvidence",
    validate_observed_evidence
);

plan_text_type!(PlanWorkspaceName, "plan.workspaceName", 256);
plan_text_type!(PlanProjectName, "plan.projectName", 256);
plan_text_type!(PlanResumeAnchor, "plan.resumeAnchor", 1024);
plan_text_type!(PlanStatement, "plan.statement", 4096);
plan_text_type!(PlanCurrentState, "plan.currentState", 16_384);
plan_text_type!(PlanDiagnosticDetail, "plan.diagnosticDetail", 16_384);
plan_text_type!(PlanClaimBlockReason, "plan.claimBlockReason", 4096);

/// Non-empty Plan document text that preserves safe Markdown layout.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: bounded non-empty document text permits layout controls but rejects other control bytes."]
pub struct PlanDocumentText(String);

impl PlanDocumentText {
    /// Validate a complete Plan document while preserving line and tab layout.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        let valid = !value.trim().is_empty()
            && value.len() <= 1_048_576
            && value.chars().all(|character| {
                !character.is_control() || matches!(character, '\n' | '\r' | '\t')
            });
        valid.then_some(Self(value)).ok_or_else(|| {
            DecodeError::new(
                "plan.documentText",
                "must be non-empty bounded text without disallowed control bytes",
            )
        })
    }

    /// Borrow the validated document text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for PlanDocumentText {
    type Error = DecodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl std::str::FromStr for PlanDocumentText {
    type Err = DecodeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // ALLOC-JUSTIFICATION: the canonical Plan document owns boundary text beyond the caller lifetime.
        Self::try_new(value.to_owned())
    }
}

impl std::fmt::Display for PlanDocumentText {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Existing Plan artifact content, including an explicitly empty file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[doc = "BRAND-INVARIANT: bounded artifact content permits layout controls but rejects other control bytes."]
pub struct PlanFileContent(String);

impl PlanFileContent {
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        let valid = value.len() <= 1_048_576
            && value.chars().all(|character| {
                !character.is_control() || matches!(character, '\n' | '\r' | '\t')
            });
        valid
            .then_some(Self(value))
            .ok_or_else(|| DecodeError::new("plan.fileContent", "contains invalid control text"))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for PlanFileContent {
    type Error = DecodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

plan_count_type!(PlanBudgetLines, usize, "plan.budgetLines");
plan_count_type!(PlanBudgetBytes, usize, "plan.budgetBytes");
plan_index_type!(LessonSequence, "plan.lessonSequence");
plan_index_type!(LedgerLineIndex, "plan.ledgerLineIndex");

/// Zero-inclusive count of records handled by a Plan import.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "BRAND-INVARIANT: count is produced only by importer enumeration."]
pub struct PlanImportCount(usize);

impl PlanImportCount {
    pub fn increment(&mut self) {
        self.0 = self.0.saturating_add(1);
    }
}

impl From<usize> for PlanImportCount {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<PlanImportCount> for usize {
    fn from(value: PlanImportCount) -> Self {
        value.0
    }
}

impl std::fmt::Display for PlanImportCount {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
/// Zero-based mutable tick/dispatch counter used by Plan orchestration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: counter arithmetic is centralized and saturating."]
pub struct OrchestratorTickCount(PlanImportCount);

impl OrchestratorTickCount {
    pub const ZERO: Self = Self(PlanImportCount(0));

    #[must_use]
    pub const fn from_count(value: PlanImportCount) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> PlanImportCount {
        self.0
    }

    pub fn increment(&mut self) {
        self.0.increment();
    }
}

impl PlanBudgetLines {
    pub const DEFAULT: Self = Self(40);
}

impl PlanBudgetBytes {
    pub const DEFAULT: Self = Self(2048);
}

/// Whether a plan-domain condition is satisfied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanCondition {
    Satisfied,
    Unsatisfied,
}

/// Whether an emitter wrote its rendered plan artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanWriteOutcome {
    Written,
    DryRun,
}

/// Requested execution mode for a Plan artifact emitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanEmissionMode {
    Apply,
    DryRun,
}

/// Whether scaffolding may replace an existing plan directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanOverwriteMode {
    RefuseExisting,
    ReplaceExisting,
}

/// Filesystem path of a rendered or persisted Plan artifact.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[doc = "BRAND-INVARIANT: non-empty filesystem path stored privately after boundary validation."]
pub struct PlanArtifactPath(PathBuf);

impl PlanArtifactPath {
    pub fn try_new(value: PathBuf) -> Result<Self, DecodeError> {
        (!value.as_os_str().is_empty())
            .then_some(Self(value))
            .ok_or_else(|| DecodeError::new("plan.artifactPath", "must not be empty"))
    }

    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    #[must_use]
    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }
}

impl TryFrom<PathBuf> for PlanArtifactPath {
    type Error = DecodeError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl std::fmt::Display for PlanArtifactPath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.display().fmt(formatter)
    }
}

/// Validated lowercase kebab-case plan directory name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for PlanName."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct PlanName(String);

impl PlanName {
    /// Validate a lowercase kebab-case name, rejecting invalid or oversized input.
    pub fn try_new(raw: &str) -> Result<Self, DecodeError> {
        let valid = !raw.is_empty()
            && raw.len() <= 128
            && raw.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
            })
            && !raw.starts_with('-')
            && !raw.ends_with('-')
            && !raw.contains("--");
        if valid {
            // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
            Ok(Self(raw.to_owned()))
        } else {
            Err(DecodeError::new(
                "planName",
                "expected lowercase kebab-case plan name",
            ))
        }
    }

    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for PlanName {
    type Err = DecodeError;
    fn from_str(raw: &str) -> Result<Self, DecodeError> {
        Self::try_new(raw)
    }
}

impl std::fmt::Display for PlanName {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Validated wildcard pattern used by plan-frontmatter ownership declarations.
///
/// This is deliberately distinct from configuration globs: it identifies a
/// plan-owned source surface in frontmatter, not a scanner configuration rule.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for PlanOwnershipPattern."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct PlanOwnershipPattern(String);

impl PlanOwnershipPattern {
    /// Validate a relative ownership pattern, rejecting invalid traversal and separators.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        let valid = !value.trim().is_empty()
            && value.len() <= 512
            && !value.contains('\0')
            && !value.contains('\\')
            && !value.starts_with('/')
            && !value.split('/').any(|segment| segment == "..");
        valid.then_some(Self(value)).ok_or_else(|| {
            DecodeError::new(
                "planOwnershipPattern",
                "must be a non-empty relative forward-slash wildcard pattern without parent traversal",
            )
        })
    }

    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for PlanOwnershipPattern {
    type Error = DecodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl std::str::FromStr for PlanOwnershipPattern {
    type Err = DecodeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
        Self::try_from(value.to_owned())
    }
}

impl std::fmt::Display for PlanOwnershipPattern {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Read order of a generated AGENTS decision forest tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for ForestTier."]
pub enum ForestTier {
    Global,
    Project,
    Plan,
}

impl ForestTier {
    #[doc = "The marker operation for this canonical domain value."]
    pub fn marker(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Project => "project",
            Self::Plan => "plan",
        }
    }
}

/// Liveness state of an orchestrated work lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for LaneStatus."]
pub enum LaneStatus {
    InFlight,
    ReportedDone,
    Dead,
}

/// Durable domain category of a captured lesson.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for LessonDomain."]
pub enum LessonDomain {
    Harness,
    Code,
}

/// Durable landing route for a captured lesson.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for LessonRoute."]
pub enum LessonRoute {
    DoctrineBlock,
    Skill,
    RuleCandidate,
    ForestNode,
    PlanDoc,
}

impl LessonRoute {
    #[doc = "The template_name operation for this canonical domain value."]
    pub fn template_name(self) -> Option<&'static str> {
        match self {
            Self::DoctrineBlock => Some("lesson-doctrine-block.tpl"),
            Self::Skill => Some("lesson-skill.tpl"),
            Self::RuleCandidate => Some("lesson-rule-candidate.tpl"),
            Self::ForestNode => Some("lesson-forest-node.tpl"),
            Self::PlanDoc => None,
        }
    }
}

/// Required fixture-pair state for a lesson routed to a rule candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for RuleCandidateFixtures."]
pub enum RuleCandidateFixtures {
    Complete,
    MissingBoth,
    MissingPass,
    MissingFail,
}
