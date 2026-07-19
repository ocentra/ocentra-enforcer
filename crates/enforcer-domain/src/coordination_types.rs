//! Canonical coordination identities and policy values.
//!
//! These types are deliberately distinct even when they share a string wire
//! representation: a writer is not a node, a claim event is not a message,
//! and a lock operation is not a conflict classification.

use crate::boundary::decode_error::DecodeError;
use crate::ids::LaneId;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdentitySpelling {
    Valid,
    Invalid,
}

fn identity_spelling(raw: &str) -> IdentitySpelling {
    let len = raw.chars().count();
    if (1..=96).contains(&len)
        && raw.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
    {
        IdentitySpelling::Valid
    } else {
        IdentitySpelling::Invalid
    }
}

/// A coordination node identifier, e.g. `node_<uuid-no-dashes>`.
#[doc = "BRAND-INVARIANT: a node identity is validated to 1..=96 safe coordination characters."]
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(String);

impl NodeId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Rejects invalid identity spelling; negative cases live in the crate's
    /// coordination parser tests.
    pub fn parse(raw: String) -> Result<Self, DecodeError> {
        if identity_spelling(&raw) == IdentitySpelling::Valid {
            Ok(Self(raw))
        } else {
            Err(DecodeError::new(
                "nodeId",
                "expected 1..=96 alphanumeric/dot/underscore/dash characters",
            ))
        }
    }

    /// Generate a fresh non-secret node label.
    pub fn random() -> Self {
        Self(format!("node_{}", random_node_suffix()))
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A coordination node display name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: validated coordination display identity with bounded safe characters."]
pub struct NodeName(String);

impl NodeName {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(raw: String) -> Result<Self, DecodeError> {
        if identity_spelling(&raw) == IdentitySpelling::Valid {
            Ok(Self(raw))
        } else {
            Err(DecodeError::new(
                "nodeName",
                "expected 1..=96 alphanumeric/dot/underscore/dash characters",
            ))
        }
    }

    pub fn sanitize_hostname(raw: &str) -> Self {
        let sanitized: String = raw
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
                    character
                } else {
                    '_'
                }
            })
            .collect();
        Self(if sanitized.is_empty() {
            // ALLOC-JUSTIFICATION: the branded node name owns its sanitized fallback.
            "unknown-host".to_owned()
        } else {
            sanitized.chars().take(96).collect()
        })
    }
}

impl std::fmt::Display for NodeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// `<nodeId>.<lane>` identity attached to an appended coordination event.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: composed only from validated node and lane identities."]
pub struct WriterId(String);

impl WriterId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn new(node_id: &NodeId, lane: &LaneId) -> Self {
        Self(format!("{node_id}.{}", lane.as_str()))
    }
    pub fn node_id_prefix(&self) -> &str {
        self.0.split('.').next().unwrap_or(&self.0)
    }
}

impl std::fmt::Display for WriterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable writer identity read from a persisted lock claim.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: validated persisted coordination writer identity."]
pub struct ClaimWriter(String);
impl ClaimWriter {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn parse(raw: String) -> Result<Self, DecodeError> {
        (identity_spelling(&raw) == IdentitySpelling::Valid)
            .then_some(Self(raw))
            .ok_or_else(|| DecodeError::new("claimWriter", "expected a validated writer identity"))
    }
}
impl From<&WriterId> for ClaimWriter {
    fn from(value: &WriterId) -> Self {
        // ALLOC-JUSTIFICATION: the persisted claim owns its writer identity independently.
        Self(value.as_str().to_owned())
    }
}
impl TryFrom<String> for ClaimWriter {
    type Error = DecodeError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}
impl std::fmt::Display for ClaimWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable lane identity read from a persisted lock claim.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: validated persisted coordination lane identity."]
pub struct ClaimLane(String);
impl ClaimLane {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn parse(raw: String) -> Result<Self, DecodeError> {
        (identity_spelling(&raw) == IdentitySpelling::Valid)
            .then_some(Self(raw))
            .ok_or_else(|| DecodeError::new("claimLane", "expected a validated lane identity"))
    }

    pub fn to_lane_id(&self) -> Result<LaneId, DecodeError> {
        self.as_str().parse()
    }
}
impl From<&LaneId> for ClaimLane {
    fn from(value: &LaneId) -> Self {
        // ALLOC-JUSTIFICATION: the persisted claim owns its lane identity independently.
        Self(value.as_str().to_owned())
    }
}
impl TryFrom<String> for ClaimLane {
    type Error = DecodeError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}
impl std::fmt::Display for ClaimLane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Event-stream identity of a lock claim.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: validated coordination claim event identity."]
pub struct ClaimEventId(String);
impl ClaimEventId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn into_string(self) -> String {
        self.0
    }
    pub fn parse(raw: String) -> Result<Self, DecodeError> {
        (identity_spelling(&raw) == IdentitySpelling::Valid)
            .then_some(Self(raw))
            .ok_or_else(|| {
                DecodeError::new("claimEventId", "expected a validated claim event identity")
            })
    }

    pub fn request_placeholder() -> Self {
        // ALLOC-JUSTIFICATION: the request sentinel is an owned branded event identity.
        Self("__request__".to_owned())
    }
}
impl TryFrom<String> for ClaimEventId {
    type Error = DecodeError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}
impl std::fmt::Display for ClaimEventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

macro_rules! non_blank_coordination_brand {
    ($name:ident, $field:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        // BRAND-INVARIANT: a non-blank coordination wire value is preserved verbatim.
        pub struct $name(String);

        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn parse(raw: &str) -> Result<Self, DecodeError> {
                if raw.trim().is_empty() {
                    Err(DecodeError::new($field, "expected a non-blank value"))
                } else {
                    // ALLOC-JUSTIFICATION: the branded value must outlive the borrowed boundary input.
                    Ok(Self(raw.to_owned()))
                }
            }

            pub fn into_string(self) -> String {
                self.0
            }

            pub fn from_static(raw: &'static str) -> Result<Self, DecodeError> {
                Self::parse(raw)
            }

            pub fn from_display(value: &impl std::fmt::Display) -> Result<Self, DecodeError> {
                // ALLOC-JUSTIFICATION: formatting materializes the owned canonical boundary value.
                Self::try_from(value.to_string())
            }
        }

        impl TryFrom<String> for $name {
            type Error = DecodeError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(&value)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

non_blank_coordination_brand!(
    ClaimPath,
    "claimPath",
    "A path or path pattern covered by a coordination claim."
);

impl From<&CoordinationWorktree> for CoordinationRepository {
    fn from(value: &CoordinationWorktree) -> Self {
        // ALLOC-JUSTIFICATION: repository identity is retained independently from the worktree value.
        Self(value.as_str().to_owned())
    }
}

/// Absolute filesystem root containing one coordination ledger.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: validated non-empty absolute coordination ledger root."]
pub struct CoordinationLedgerRoot(PathBuf);

impl CoordinationLedgerRoot {
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    pub fn parse(path: &Path) -> Result<Self, DecodeError> {
        if path.as_os_str().is_empty() || !path.is_absolute() {
            Err(DecodeError::new(
                "coordinationLedgerRoot",
                "expected a non-empty absolute path",
            ))
        } else {
            Ok(Self(path.to_path_buf()))
        }
    }
}

impl TryFrom<PathBuf> for CoordinationLedgerRoot {
    type Error = DecodeError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl std::fmt::Display for CoordinationLedgerRoot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0.display())
    }
}

/// Absolute repository root used to expand coordination owns-sets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: validated non-empty absolute coordination repository root."]
pub struct CoordinationRepoRoot(PathBuf);

impl CoordinationRepoRoot {
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    pub fn parse(path: &Path) -> Result<Self, DecodeError> {
        if path.as_os_str().is_empty() || !path.is_absolute() {
            Err(DecodeError::new(
                "coordinationRepoRoot",
                "expected a non-empty absolute path",
            ))
        } else {
            Ok(Self(path.to_path_buf()))
        }
    }
}

impl TryFrom<PathBuf> for CoordinationRepoRoot {
    type Error = DecodeError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl std::fmt::Display for CoordinationRepoRoot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0.display())
    }
}

/// Absolute filesystem path to a coordination stream or archive segment.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: validated non-empty absolute coordination ledger path."]
pub struct CoordinationLedgerPath(PathBuf);

impl CoordinationLedgerPath {
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    pub fn parse(path: &Path) -> Result<Self, DecodeError> {
        if path.as_os_str().is_empty() || !path.is_absolute() {
            Err(DecodeError::new(
                "coordinationLedgerPath",
                "expected a non-empty absolute path",
            ))
        } else {
            Ok(Self(path.to_path_buf()))
        }
    }

    pub fn from_absolute_path(path: &Path) -> Result<Self, DecodeError> {
        Self::parse(path)
    }
}

impl std::fmt::Display for CoordinationLedgerPath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0.display())
    }
}
non_blank_coordination_brand!(
    CoordinationProjectId,
    "coordinationProjectId",
    "The caller-supplied project identity used to scope coordination."
);
non_blank_coordination_brand!(
    CoordinationRepository,
    "coordinationRepository",
    "A repository identity, supplied as either a root or a remote."
);
non_blank_coordination_brand!(
    CoordinationWorktree,
    "coordinationWorktree",
    "A worktree identity within a coordinated repository."
);
non_blank_coordination_brand!(
    CoordinationBranch,
    "coordinationBranch",
    "A branch identity used by coordination conflict evaluation."
);
non_blank_coordination_brand!(
    CoordinationOwnerIdentity,
    "coordinationOwnerIdentity",
    "A thread or session identity used to determine claim ownership."
);
non_blank_coordination_brand!(
    ClaimGroup,
    "claimGroup",
    "A caller-defined group shared by related coordination claims."
);
non_blank_coordination_brand!(
    ClaimReason,
    "claimReason",
    "The human-readable reason attached to a coordination claim."
);
non_blank_coordination_brand!(
    ClaimComparisonKey,
    "claimComparisonKey",
    "A normalized internal key used only for coordination comparisons."
);
non_blank_coordination_brand!(
    CoordinationTimestamp,
    "coordinationTimestamp",
    "A timestamp recorded by the coordination ledger."
);
non_blank_coordination_brand!(
    CoordinationMessageBody,
    "coordinationMessageBody",
    "A non-blank message body carried by coordination mail."
);
non_blank_coordination_brand!(
    CoordinationStreamName,
    "coordinationStreamName",
    "The canonical name of a coordination event stream."
);
non_blank_coordination_brand!(
    CoordinationArchiveStamp,
    "coordinationArchiveStamp",
    "The sortable timestamp label of one archived coordination stream segment."
);
non_blank_coordination_brand!(
    CoordinationWarning,
    "coordinationWarning",
    "A diagnostic warning emitted while reading coordination streams."
);
non_blank_coordination_brand!(
    CoordinationRejection,
    "coordinationRejection",
    "A typed explanation for a rejected coordination operation."
);
non_blank_coordination_brand!(
    FixGeneratorName,
    "fixGeneratorName",
    "The stable human-readable identity of a fix generator."
);

/// Absolute path to a file targeted by the bounded fix loop.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: validated non-empty absolute fix-loop target path."]
pub struct FixTargetPath(PathBuf);

impl FixTargetPath {
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    pub fn parent_root(&self) -> Result<FixWorkspaceRoot, DecodeError> {
        let root = self.0.parent().unwrap_or(&self.0).to_path_buf();
        FixWorkspaceRoot::try_from(root)
    }
}

impl AsRef<Path> for FixTargetPath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl TryFrom<PathBuf> for FixTargetPath {
    type Error = DecodeError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        if value.as_os_str().is_empty() || !value.is_absolute() {
            Err(DecodeError::new(
                "fixTargetPath",
                "expected a non-empty absolute path",
            ))
        } else {
            Ok(Self(value))
        }
    }
}

/// Absolute directory under which one fix generator may edit.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: validated non-empty absolute fix-loop workspace root."]
pub struct FixWorkspaceRoot(PathBuf);

impl FixWorkspaceRoot {
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl TryFrom<PathBuf> for FixWorkspaceRoot {
    type Error = DecodeError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        if value.as_os_str().is_empty() || !value.is_absolute() {
            Err(DecodeError::new(
                "fixWorkspaceRoot",
                "expected a non-empty absolute path",
            ))
        } else {
            Ok(Self(value))
        }
    }
}

/// Non-negative number of findings observed by the fix loop.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: represents a non-negative in-memory finding cardinality."]
pub struct FindingCount(usize);

impl FindingCount {
    pub fn from_collection<T>(values: &[T]) -> Self {
        Self(values.len())
    }

    pub fn increment(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Non-negative count of coordination events.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: represents a non-negative in-memory event cardinality."]
pub struct CoordinationEventCount(usize);

impl CoordinationEventCount {
    pub fn from_collection<T>(values: &[T]) -> Self {
        Self(values.len())
    }

    pub fn increment(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Positive number of newest events retained in a live stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CompactionKeepCount(std::num::NonZeroUsize);

impl CompactionKeepCount {
    pub const fn new(value: std::num::NonZeroUsize) -> Self {
        Self(value)
    }

    pub const fn value(self) -> std::num::NonZeroUsize {
        self.0
    }
}

impl std::fmt::Display for FindingCount {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// One-based bounded fix-loop iteration number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: validated positive fix-loop iteration number."]
pub struct FixIteration(std::num::NonZeroU32);

impl FixIteration {
    pub const fn new(value: std::num::NonZeroU32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> std::num::NonZeroU32 {
        self.0
    }
}

/// Whether a generator changed its permitted fix-loop workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixAttemptOutcome {
    Changed,
    Declined,
}

/// Whether one proposed fix-loop edit was accepted after validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixAcceptance {
    Accepted,
    Reverted,
}

/// Whether the fix loop stopped at its configured iteration cap.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IterationCapStatus {
    Reached,
    #[default]
    NotReached,
}

/// Whether a claim request was accepted or blocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimOutcomeStatus {
    Accepted,
    Blocked,
}

/// Whether closeout filtering is restricted to one lane or includes all lanes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CloseoutLaneScope {
    #[default]
    SelectedLane,
    AllLanes,
}

/// Result of evaluating one coordination filter predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimFilterMatch {
    Matches,
    Excluded,
}

/// Whether a raw claim carried caller-supplied context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimContextPresence {
    Explicit,
    LegacyImplicit,
}

/// Result of comparing two coordination identities or paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinationMatch {
    Matches,
    Differs,
}

/// Whether a claim covers a protected singleton file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectedSingletonStatus {
    Protected,
    Ordinary,
}

/// Event kinds emitted by the coordination command surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinationEventKind {
    Claim,
    Release,
    Message,
    Acknowledgement,
}

impl CoordinationEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claim => "claim",
            Self::Release => "release",
            Self::Message => "message",
            Self::Acknowledgement => "ack",
        }
    }
}

/// The kind of exclusive coordination claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockKind {
    WriteLock,
    GlobalWriteLock,
    BranchLease,
    WorkReservation,
}
impl LockKind {
    pub fn parse(raw: &str) -> Result<Self, DecodeError> {
        match raw {
            "writeLock" => Ok(Self::WriteLock),
            "globalWriteLock" => Ok(Self::GlobalWriteLock),
            "branchLease" => Ok(Self::BranchLease),
            "workReservation" => Ok(Self::WorkReservation),
            _ => Err(DecodeError::new(
                "lockKind",
                "expected writeLock, globalWriteLock, branchLease, or workReservation",
            )),
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WriteLock => "writeLock",
            Self::GlobalWriteLock => "globalWriteLock",
            Self::BranchLease => "branchLease",
            Self::WorkReservation => "workReservation",
        }
    }
}

/// The coordination operation for conflict evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Operation {
    Inspect,
    #[default]
    Edit,
    Commit,
    Push,
    Rebase,
    Merge,
    PrReady,
}
impl Operation {
    pub fn parse(raw: &str) -> Result<Self, DecodeError> {
        match raw {
            "inspect" => Ok(Self::Inspect),
            "edit" => Ok(Self::Edit),
            "commit" => Ok(Self::Commit),
            "push" => Ok(Self::Push),
            "rebase" => Ok(Self::Rebase),
            "merge" => Ok(Self::Merge),
            "pr_ready" => Ok(Self::PrReady),
            _ => Err(DecodeError::new(
                "operation",
                "expected inspect, edit, commit, push, rebase, merge, or pr_ready",
            )),
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inspect => "inspect",
            Self::Edit => "edit",
            Self::Commit => "commit",
            Self::Push => "push",
            Self::Rebase => "rebase",
            Self::Merge => "merge",
            Self::PrReady => "pr_ready",
        }
    }
}

/// Request behavior when a claim conflicts with an active claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnConflict {
    Fail,
    Intent,
}
impl OnConflict {
    pub fn parse(raw: &str) -> Result<Self, DecodeError> {
        match raw {
            "fail" => Ok(Self::Fail),
            "intent" => Ok(Self::Intent),
            _ => Err(DecodeError::new("onConflict", "expected fail or intent")),
        }
    }
}

/// One coordination conflict classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictType {
    WriteLockConflict,
    BranchWriteConflict,
    GlobalWriteConflict,
    BranchLeaseConflict,
    MergeRisk,
    WorkReservationOverlap,
}
impl ConflictType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WriteLockConflict => "write-lock-conflict",
            Self::BranchWriteConflict => "branch-write-conflict",
            Self::GlobalWriteConflict => "global-write-conflict",
            Self::BranchLeaseConflict => "branch-lease-conflict",
            Self::MergeRisk => "merge-risk",
            Self::WorkReservationOverlap => "work-reservation-overlap",
        }
    }
}

/// Why one bounded fix-loop iteration was accepted or rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IterationReason {
    Improved,
    GeneratorDeclined,
    NotImproved,
    NewRuleIdIntroduced,
}

impl IterationReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Improved => "improved",
            Self::GeneratorDeclined => "generatorDeclined",
            Self::NotImproved => "notImproved",
            Self::NewRuleIdIntroduced => "newRuleIdIntroduced",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, DecodeError> {
        match raw {
            "improved" => Ok(Self::Improved),
            "generatorDeclined" => Ok(Self::GeneratorDeclined),
            "notImproved" => Ok(Self::NotImproved),
            "newRuleIdIntroduced" => Ok(Self::NewRuleIdIntroduced),
            _ => Err(DecodeError::new(
                "iterationReason",
                "expected improved, generatorDeclined, notImproved, or newRuleIdIntroduced",
            )),
        }
    }
}

fn random_node_suffix() -> String {
    const SUFFIX_LENGTH: usize = 32;
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
        ^ u128::from(std::process::id()) << 64;
    let mut state = seed | 1;
    let mut out = String::with_capacity(SUFFIX_LENGTH);
    while out.len() < SUFFIX_LENGTH {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let low_bits = u64::try_from(state & u128::from(u64::MAX)).unwrap_or(u64::MAX);
        out.push_str(&format!("{low_bits:016x}"));
    }
    out.truncate(SUFFIX_LENGTH);
    out
}

#[cfg(test)]
mod property_tests {
    use super::NodeId;
    use proptest::{prop_assert_eq, proptest};

    proptest! {
        #[test]
        fn node_id_parser_accepts_generated_safe_values(raw in "[A-Za-z0-9._-]{1,96}") {
            prop_assert_eq!(
                NodeId::parse(raw.clone()).map(|value| value.as_str().to_owned()),
                Ok(raw)
            );
        }
    }
}
