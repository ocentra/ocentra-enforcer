//! Typed wire records for the native test-doctrine analysis.
//!
//! Analysis belongs to `enforcer-scan`; this leaf crate owns the stable
//! report shape. Raw JSON scalars remain private behind the small canonical
//! value objects below, so callers cannot construct malformed report state.

use std::collections::BTreeMap;

use serde::ser::{Error as _, Serializer};
use serde::Serialize;

/// Opaque human-readable report text produced only by the analyzer.
/// BRAND-INVARIANT: non-empty, NUL-free analyzer-owned display text.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TestDoctrineText(String);
impl TestDoctrineText {
    pub fn try_new(value: String) -> Result<Self, crate::boundary::decode_error::DecodeError> {
        if value.trim().is_empty() || value.contains('\0') {
            return Err(crate::boundary::decode_error::DecodeError::new(
                "testDoctrineText",
                "must be non-empty and NUL-free",
            ));
        }
        Ok(Self(value))
    }
    #[must_use]
    pub fn from_owned(value: String) -> Self {
        Self(value)
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl Serialize for TestDoctrineText {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

/// Opaque count calculated by the analyzer, never decoded from a caller.
/// BRAND-INVARIANT: non-negative analyzer-computed cardinality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TestDoctrineCount(usize);
impl TestDoctrineCount {
    pub fn try_new(value: usize) -> Result<Self, crate::boundary::decode_error::DecodeError> {
        Ok(Self(value))
    }
    #[must_use]
    pub const fn from_usize(value: usize) -> Self {
        Self(value)
    }
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0
    }
}
impl Serialize for TestDoctrineCount {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match u64::try_from(self.0) {
            Ok(value) => serializer.serialize_u64(value),
            Err(_) => Err(S::Error::custom("test doctrine count exceeds u64")),
        }
    }
}

/// Explicit binary evidence state, serialized as the frozen report's boolean.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestDoctrineEvidenceState {
    Present,
    Absent,
}
impl TestDoctrineEvidenceState {
    #[must_use]
    pub const fn from_bool(value: bool) -> Self {
        if value {
            Self::Present
        } else {
            Self::Absent
        }
    }
    #[must_use]
    pub const fn is_present(self) -> bool {
        matches!(self, Self::Present)
    }
}
impl Serialize for TestDoctrineEvidenceState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(self.is_present())
    }
}

/// CI gate state: a category is not invoked, invoked non-blockingly, or blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestDoctrineBlockingState {
    NotInvoked,
    NonBlocking,
    Blocking,
}
impl TestDoctrineBlockingState {
    #[must_use]
    pub const fn from_evidence(wired: bool, blocking: bool) -> Self {
        if !wired {
            Self::NotInvoked
        } else if blocking {
            Self::Blocking
        } else {
            Self::NonBlocking
        }
    }
    #[must_use]
    pub const fn is_blocking(self) -> bool {
        matches!(self, Self::Blocking)
    }
}
impl Serialize for TestDoctrineBlockingState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::NotInvoked => serializer.serialize_none(),
            Self::NonBlocking => serializer.serialize_some(&false),
            Self::Blocking => serializer.serialize_some(&true),
        }
    }
}

/// The priority implied by a detected project surface.
/// SERIALIZATION-DOC: encoded as `core`, `suggested`, or `optional` in MCP/UI reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestDoctrineTier {
    Core,
    Suggested,
    Optional,
}
impl Serialize for TestDoctrineTier {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::Core => "core",
            Self::Suggested => "suggested",
            Self::Optional => "optional",
        })
    }
}

/// One test practice recognised by the doctrine analyzer.
/// SERIALIZATION-DOC: encoded as the frozen MCP category key; enum is manually serialized because it is a closed wire vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TestDoctrineCategory {
    Unit,
    Integration,
    E2e,
    Contract,
    Mutation,
    PropertyFuzzing,
    Security,
    Snapshot,
    LoadPerformance,
    CoverageTooling,
    ConcurrencyRaceTests,
    IdempotencyReplayTests,
    RollbackCompensationTests,
    TimeClockTests,
    EconomicInvariantTests,
    KillSwitchTests,
}
impl Serialize for TestDoctrineCategory {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::Unit => "unit",
            Self::Integration => "integration",
            Self::E2e => "e2e",
            Self::Contract => "contract",
            Self::Mutation => "mutation",
            Self::PropertyFuzzing => "propertyFuzzing",
            Self::Security => "security",
            Self::Snapshot => "snapshot",
            Self::LoadPerformance => "loadPerformance",
            Self::CoverageTooling => "coverageTooling",
            Self::ConcurrencyRaceTests => "concurrencyRaceTests",
            Self::IdempotencyReplayTests => "idempotencyReplayTests",
            Self::RollbackCompensationTests => "rollbackCompensationTests",
            Self::TimeClockTests => "timeClockTests",
            Self::EconomicInvariantTests => "economicInvariantTests",
            Self::KillSwitchTests => "killSwitchTests",
        })
    }
}

/// SERIALIZATION-DOC: camelCase project-nature evidence object emitted by MCP/UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestDoctrineNature {
    languages: BTreeMap<TestDoctrineText, TestDoctrineCount>,
    is_web_api: TestDoctrineEvidenceState,
    has_open_api_spec: TestDoctrineEvidenceState,
    has_frontend_ui: TestDoctrineEvidenceState,
    has_async_workers: TestDoctrineEvidenceState,
    has_money_critical_surface: TestDoctrineEvidenceState,
    money_critical_files: Vec<TestDoctrineText>,
    has_multi_service_boundary: TestDoctrineEvidenceState,
    multi_service_client_files: Vec<TestDoctrineText>,
}
impl TestDoctrineNature {
    #[must_use]
    pub fn new(languages: BTreeMap<TestDoctrineText, TestDoctrineCount>) -> Self {
        Self {
            languages,
            is_web_api: TestDoctrineEvidenceState::Absent,
            has_open_api_spec: TestDoctrineEvidenceState::Absent,
            has_frontend_ui: TestDoctrineEvidenceState::Absent,
            has_async_workers: TestDoctrineEvidenceState::Absent,
            has_money_critical_surface: TestDoctrineEvidenceState::Absent,
            money_critical_files: Vec::new(),
            has_multi_service_boundary: TestDoctrineEvidenceState::Absent,
            multi_service_client_files: Vec::new(),
        }
    }
    #[must_use]
    pub fn with_web_api(mut self, value: TestDoctrineEvidenceState) -> Self {
        self.is_web_api = value;
        self
    }
    #[must_use]
    pub fn with_open_api(mut self, value: TestDoctrineEvidenceState) -> Self {
        self.has_open_api_spec = value;
        self
    }
    #[must_use]
    pub fn with_frontend(mut self, value: TestDoctrineEvidenceState) -> Self {
        self.has_frontend_ui = value;
        self
    }
    #[must_use]
    pub fn with_async_workers(mut self, value: TestDoctrineEvidenceState) -> Self {
        self.has_async_workers = value;
        self
    }
    #[must_use]
    pub fn with_money_surface(
        mut self,
        value: TestDoctrineEvidenceState,
        files: Vec<TestDoctrineText>,
    ) -> Self {
        self.has_money_critical_surface = value;
        self.money_critical_files = files;
        self
    }
    #[must_use]
    pub fn with_service_boundary(
        mut self,
        value: TestDoctrineEvidenceState,
        files: Vec<TestDoctrineText>,
    ) -> Self {
        self.has_multi_service_boundary = value;
        self.multi_service_client_files = files;
        self
    }
    #[must_use]
    pub const fn is_web_api(&self) -> bool {
        self.is_web_api.is_present()
    }
    #[must_use]
    pub const fn has_frontend_ui(&self) -> bool {
        self.has_frontend_ui.is_present()
    }
    #[must_use]
    pub const fn has_async_workers(&self) -> bool {
        self.has_async_workers.is_present()
    }
    #[must_use]
    pub const fn has_money_critical_surface(&self) -> bool {
        self.has_money_critical_surface.is_present()
    }
    #[must_use]
    pub const fn has_multi_service_boundary(&self) -> bool {
        self.has_multi_service_boundary.is_present()
    }
    #[must_use]
    pub fn money_critical_files(&self) -> &[TestDoctrineText] {
        &self.money_critical_files
    }
}

/// SERIALIZATION-DOC: a CI step's display text and blocking verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestDoctrineCiEvidence {
    step: TestDoctrineText,
    blocking: TestDoctrineEvidenceState,
}
impl TestDoctrineCiEvidence {
    #[must_use]
    pub fn new(step: TestDoctrineText, blocking: TestDoctrineEvidenceState) -> Self {
        Self { step, blocking }
    }
    #[must_use]
    pub const fn is_blocking(&self) -> bool {
        self.blocking.is_present()
    }
}

/// SERIALIZATION-DOC: native CI wiring state; `blocking` preserves the frozen bool-or-null wire contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestDoctrineCiState {
    wired: TestDoctrineEvidenceState,
    blocking: TestDoctrineBlockingState,
    evidence: Vec<TestDoctrineCiEvidence>,
}
impl TestDoctrineCiState {
    #[must_use]
    pub fn empty() -> Self {
        Self::new(
            TestDoctrineEvidenceState::Absent,
            TestDoctrineBlockingState::NotInvoked,
            Vec::new(),
        )
    }
    #[must_use]
    pub fn new(
        wired: TestDoctrineEvidenceState,
        blocking: TestDoctrineBlockingState,
        evidence: Vec<TestDoctrineCiEvidence>,
    ) -> Self {
        Self {
            wired,
            blocking,
            evidence,
        }
    }
    #[must_use]
    pub const fn is_wired(&self) -> bool {
        self.wired.is_present()
    }
    #[must_use]
    pub const fn is_blocking(&self) -> bool {
        self.blocking.is_blocking()
    }
    #[must_use]
    pub fn evidence(&self) -> &[TestDoctrineCiEvidence] {
        &self.evidence
    }
}

/// SERIALIZATION-DOC: category evidence/relevance record emitted under `detected`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestDoctrineDetection {
    label: TestDoctrineText,
    present: TestDoctrineEvidenceState,
    evidence: Vec<TestDoctrineText>,
    relevant: TestDoctrineEvidenceState,
    ci: TestDoctrineCiState,
    ci_including_untracked: Option<TestDoctrineCiState>,
}
impl TestDoctrineDetection {
    #[must_use]
    pub fn new(label: TestDoctrineText) -> Self {
        Self {
            label,
            present: TestDoctrineEvidenceState::Absent,
            evidence: Vec::new(),
            relevant: TestDoctrineEvidenceState::Absent,
            ci: TestDoctrineCiState::empty(),
            ci_including_untracked: None,
        }
    }
    #[must_use]
    pub fn with_evidence(
        mut self,
        present: TestDoctrineEvidenceState,
        evidence: Vec<TestDoctrineText>,
    ) -> Self {
        self.present = present;
        self.evidence = evidence;
        self
    }
    #[must_use]
    pub fn with_relevance(mut self, relevant: TestDoctrineEvidenceState) -> Self {
        self.relevant = relevant;
        self
    }
    #[must_use]
    pub fn with_ci(
        mut self,
        ci: TestDoctrineCiState,
        including_untracked: Option<TestDoctrineCiState>,
    ) -> Self {
        self.ci = ci;
        self.ci_including_untracked = including_untracked;
        self
    }
    #[must_use]
    pub const fn is_present(&self) -> bool {
        self.present.is_present()
    }
    #[must_use]
    pub const fn is_relevant(&self) -> bool {
        self.relevant.is_present()
    }
    #[must_use]
    pub fn ci(&self) -> &TestDoctrineCiState {
        &self.ci
    }
}

/// SERIALIZATION-DOC: relevant category absent from the project, with tier and explanation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestDoctrineMissing {
    category: TestDoctrineCategory,
    label: TestDoctrineText,
    tier: TestDoctrineTier,
    reason: TestDoctrineText,
}
impl TestDoctrineMissing {
    #[must_use]
    pub fn new(
        category: TestDoctrineCategory,
        label: TestDoctrineText,
        tier: TestDoctrineTier,
        reason: TestDoctrineText,
    ) -> Self {
        Self {
            category,
            label,
            tier,
            reason,
        }
    }
    #[must_use]
    pub const fn category(&self) -> TestDoctrineCategory {
        self.category
    }
    #[must_use]
    pub const fn tier(&self) -> TestDoctrineTier {
        self.tier
    }
}

/// SERIALIZATION-DOC: detected local category not protected by a blocking CI gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestDoctrineCiGap {
    category: TestDoctrineCategory,
    label: TestDoctrineText,
    reason: TestDoctrineText,
    ci_evidence: Vec<TestDoctrineCiEvidence>,
}
impl TestDoctrineCiGap {
    #[must_use]
    pub fn new(
        category: TestDoctrineCategory,
        label: TestDoctrineText,
        reason: TestDoctrineText,
        evidence: Vec<TestDoctrineCiEvidence>,
    ) -> Self {
        Self {
            category,
            label,
            reason,
            ci_evidence: evidence,
        }
    }
    #[must_use]
    pub const fn category(&self) -> TestDoctrineCategory {
        self.category
    }
}

/// SERIALIZATION-DOC: report totals used by MCP/UI presentation only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestDoctrineSummary {
    categories_relevant: TestDoctrineCount,
    categories_present: TestDoctrineCount,
    categories_missing: TestDoctrineCount,
    core_missing: TestDoctrineCount,
    ci_gaps: TestDoctrineCount,
}
impl TestDoctrineSummary {
    #[must_use]
    pub fn new(
        relevant: TestDoctrineCount,
        present: TestDoctrineCount,
        missing: TestDoctrineCount,
        core_missing: TestDoctrineCount,
        ci_gaps: TestDoctrineCount,
    ) -> Self {
        Self {
            categories_relevant: relevant,
            categories_present: present,
            categories_missing: missing,
            core_missing,
            ci_gaps,
        }
    }
}

/// SERIALIZATION-DOC: top-level native test-doctrine report mirrors frozen MCP keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestDoctrineReport {
    root: TestDoctrineText,
    caveat: TestDoctrineText,
    nature: TestDoctrineNature,
    ci_config_files_found: Vec<TestDoctrineCiConfigFile>,
    has_untracked_ci_files: TestDoctrineEvidenceState,
    detected: BTreeMap<TestDoctrineCategory, TestDoctrineDetection>,
    missing: Vec<TestDoctrineMissing>,
    ci_gaps: Vec<TestDoctrineCiGap>,
    summary: TestDoctrineSummary,
}
impl TestDoctrineReport {
    #[must_use]
    pub fn new(
        root: TestDoctrineText,
        caveat: TestDoctrineText,
        nature: TestDoctrineNature,
    ) -> Self {
        Self {
            root,
            caveat,
            nature,
            ci_config_files_found: Vec::new(),
            has_untracked_ci_files: TestDoctrineEvidenceState::Absent,
            detected: BTreeMap::new(),
            missing: Vec::new(),
            ci_gaps: Vec::new(),
            summary: TestDoctrineSummary::new(
                TestDoctrineCount::from_usize(0),
                TestDoctrineCount::from_usize(0),
                TestDoctrineCount::from_usize(0),
                TestDoctrineCount::from_usize(0),
                TestDoctrineCount::from_usize(0),
            ),
        }
    }
    #[must_use]
    pub fn with_ci_files(
        mut self,
        files: Vec<TestDoctrineCiConfigFile>,
        untracked: TestDoctrineEvidenceState,
    ) -> Self {
        self.ci_config_files_found = files;
        self.has_untracked_ci_files = untracked;
        self
    }
    #[must_use]
    pub fn with_results(
        mut self,
        detected: BTreeMap<TestDoctrineCategory, TestDoctrineDetection>,
        missing: Vec<TestDoctrineMissing>,
        ci_gaps: Vec<TestDoctrineCiGap>,
        summary: TestDoctrineSummary,
    ) -> Self {
        self.detected = detected;
        self.missing = missing;
        self.ci_gaps = ci_gaps;
        self.summary = summary;
        self
    }
    #[must_use]
    pub fn nature(&self) -> &TestDoctrineNature {
        &self.nature
    }
    #[must_use]
    pub fn detection(&self, category: TestDoctrineCategory) -> Option<&TestDoctrineDetection> {
        self.detected.get(&category)
    }
    #[must_use]
    pub fn missing(&self) -> &[TestDoctrineMissing] {
        &self.missing
    }
    #[must_use]
    pub fn ci_gaps(&self) -> &[TestDoctrineCiGap] {
        &self.ci_gaps
    }
}

/// SERIALIZATION-DOC: committed CI config discovered by the native walk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestDoctrineCiConfigFile {
    path: TestDoctrineText,
    tracked: TestDoctrineEvidenceState,
}
impl TestDoctrineCiConfigFile {
    #[must_use]
    pub fn new(path: TestDoctrineText, tracked: TestDoctrineEvidenceState) -> Self {
        Self { path, tracked }
    }
}
