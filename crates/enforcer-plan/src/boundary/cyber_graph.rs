//! BOUNDARY-INVARIANT: raw Cyber Plan manifests, Markdown evidence, and JSON
//! catalog/proof inputs are decoded here and converted into validated graph
//! values before execution state is derived.
//! NEGATIVE-TEST: malformed paths, unknown dependencies, missing evidence,
//! protected-source access, cycles, and incomplete DONE contracts are rejected
//! by the graph validation tests.
//! SERIALIZATION-DOC: the public graph views are stable JSON wire outputs used by
//! the read-only CLI; serde does not imply implementation or proof completion.
//!
//! Repo-owned execution graph serving the CyberSkills plan.
//!
//! Markdown remains the detailed intent and acceptance source underlying the plan.
//! This module is the small machine-readable control plane above that prose:
//! it imports the CyberSkills workpack index, its workpack documents, the
//! test/proof table, CP08 evidence, and the disposition catalog; then it
//! derives dependency readiness without turning decomposition into native
//! implementation or executable proof.
//!
//! The graph is deliberately local and deterministic. It has no database, no
//! network dependency, and never reads the vendor tree. In particular, the
//! protected `sourceUnavailable` record is represented from the disposition
//! ledger only and is never opened as a vendor file.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The checked-in Cyber Plan graph manifest.
pub const GRAPH_MANIFEST_PATH: &str = "docs/engineering-graph.json";
const SCHEMA_VERSION: u32 = 1;
const PROTECTED_SKILL: &str = "detecting-fileless-malware-techniques";
const CYBERSKILLS_REGISTRY_PATH: &str = "crates/enforcer-rules/rules/cyberskills.json";

/// Errors returned while loading or validating the Cyber Plan graph.
#[derive(Debug, Error)]
pub enum GraphError {
    /// A repository file could not be read.
    #[error("engineering graph I/O failed: {0}")]
    Io(#[from] std::io::Error),
    /// A graph or evidence file was not valid JSON.
    #[error("engineering graph JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    /// A graph identifier or relative path violated its boundary contract.
    #[error("engineering graph value is invalid: {0}")]
    InvalidValue(String),
    /// A requested graph node does not exist.
    #[error("engineering graph node `{0}` does not exist")]
    MissingNode(String),
}

/// A stable graph identifier. Paths are metadata; IDs are the graph identity.
/// BRAND-INVARIANT: node identifiers are non-empty, repository-safe graph keys.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(String);

impl NodeId {
    /// Construct a validated identifier from its wire form.
    pub fn try_new(raw: impl Into<String>) -> Result<Self, GraphError> {
        let value = raw.into();
        let valid = !value.trim().is_empty()
            && !value.contains("..")
            && value
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "-_/:.".contains(character));
        valid
            .then_some(Self(value.clone()))
            .ok_or_else(|| GraphError::InvalidValue(format!("invalid node id `{value}`")))
    }

    /// Construct a validated identifier from its wire form.
    pub fn new(raw: impl Into<String>) -> Result<Self, GraphError> {
        Self::try_new(raw)
    }

    /// Borrow the stable identifier text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for NodeId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for NodeId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::try_new(raw).map_err(serde::de::Error::custom)
    }
}

impl Display for NodeId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A validated repository-relative graph path.
/// BRAND-INVARIANT: graph paths are normalized safe repository-relative paths.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphPath(String);

impl GraphPath {
    /// Construct a normalized, validated repository-relative path.
    pub fn try_new(raw: impl Into<String>) -> Result<Self, GraphError> {
        let value = raw.into().replace('\\', "/");
        is_safe_relative_path(&value)
            .then_some(Self(value.clone()))
            .ok_or_else(|| {
                GraphError::InvalidValue(format!("invalid graph-relative path `{value}`"))
            })
    }

    /// Construct a normalized forward-slash relative path.
    pub fn new(raw: impl Into<String>) -> Result<Self, GraphError> {
        Self::try_new(raw)
    }

    /// Borrow the path text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for GraphPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for GraphPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::try_new(raw).map_err(serde::de::Error::custom)
    }
}

impl Display for GraphPath {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

fn is_safe_relative_path(value: &str) -> bool {
    !value.trim().is_empty()
        && !value.starts_with('/')
        && !value.starts_with('\\')
        && !value.contains(':')
        && !value
            .split('/')
            .any(|segment| segment == ".." || segment.is_empty())
}

/// Graph node categories used by the Cyber Plan.
#[doc = "SERDE-TAG-JUSTIFICATION: graph node categories use stable scalar wire values within the read-only graph contract."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NodeKind {
    /// The plan's durable outcome.
    Goal,
    /// The CyberSkills parity plan itself.
    Plan,
    /// A bounded CP00-CP13 execution unit.
    Workpack,
    /// A checklist or acceptance requirement imported from a workpack.
    Requirement,
    /// A vendor catalog identity, kept separate from implementation status.
    Skill,
    /// A canonical intent family derived from the source matrix.
    IntentFamily,
    /// A named test gate from the plan proof table.
    Test,
    /// A committed proof or evidence artifact.
    Proof,
    /// An architecture decision record.
    Adr,
    /// A dependency owned by another plan or authority.
    Dependency,
}

/// Stored lifecycle state. Ready and blocked are always derived.
#[doc = "SERDE-TAG-JUSTIFICATION: lifecycle states use stable scalar wire values within the read-only graph contract."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LifecycleState {
    /// Work has been identified but not started.
    Planned,
    /// Work is actively being changed.
    Active,
    /// Work is awaiting its validation gates.
    Validation,
    /// Work failed a required gate and needs adaptation.
    Failed,
    /// Work is intentionally paused.
    Paused,
    /// Work was completed through the graph completion contract.
    Done,
}

/// Derived state presented to agents and humans.
#[doc = "SERDE-TAG-JUSTIFICATION: derived states use stable scalar wire values within the read-only graph contract."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DerivedState {
    /// All hard dependencies are done and the workpack may start.
    Ready,
    /// A dependency, contract, or graph integrity issue prevents progress.
    Blocked,
    /// Stored active lifecycle state.
    Active,
    /// Stored validation lifecycle state.
    Validation,
    /// Contract-validated completion.
    Done,
    /// Stored failed lifecycle state.
    Failed,
    /// Stored paused lifecycle state.
    Paused,
    /// A non-workpack node has no execution lifecycle.
    Planned,
}

/// Implementation coverage and proof coverage remain independent.
#[doc = "SERDE-TAG-JUSTIFICATION: coverage levels use stable scalar wire values within the read-only graph contract."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoverageLevel {
    /// No accepted evidence for this dimension.
    None,
    /// Some evidence exists, but the dimension is not complete.
    Partial,
    /// The dimension's own contract is complete.
    Complete,
}

/// Typed graph edge kinds.
#[doc = "SERDE-TAG-JUSTIFICATION: edge kinds use stable scalar wire values within the read-only graph contract."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EdgeKind {
    /// Hierarchical ownership from a goal/plan to its children.
    Contains,
    /// A hard execution dependency; the target must be done first.
    DependsOn,
    /// A completion-contract requirement.
    Requires,
    /// A workpack produces an evidence artifact.
    Produces,
    /// An evidence artifact supports a catalog identity.
    EvidenceFor,
    /// An intent family classifies a catalog identity.
    Classifies,
    /// An intent family routes work to a bounded action packet.
    RoutesTo,
    /// A plan document points at an ADR or retained reference.
    References,
}

/// One directed graph edge.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GraphEdge {
    /// Edge origin.
    pub from: NodeId,
    /// Edge target.
    pub to: NodeId,
    /// Semantic edge kind.
    pub kind: EdgeKind,
}

/// The mechanically checked completion contract attached to a node.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompletionContract {
    /// Repository paths that must exist before completion.
    pub required_paths: Vec<GraphPath>,
    /// Test nodes that must have inspectable evidence.
    pub required_tests: Vec<NodeId>,
    /// Proof nodes that must exist and be readable.
    pub required_proofs: Vec<NodeId>,
    /// ADR nodes required by the workpack's acceptance contract.
    pub required_adrs: Vec<NodeId>,
    /// Total checklist items imported from the workpack.
    pub checklist_total: usize,
    /// Checklist items explicitly checked in the workpack.
    pub checklist_complete: usize,
}

/// One graph node and its imported or explicitly declared metadata.
#[derive(Debug, Clone)]
pub struct GraphNode {
    /// Stable graph identity.
    pub id: NodeId,
    /// Semantic node category.
    pub kind: NodeKind,
    /// Human-readable intent title.
    pub title: String,
    /// Repository evidence path, when the node has one.
    pub path: Option<GraphPath>,
    /// Optional parent node within the hierarchy.
    pub parent: Option<NodeId>,
    /// Stored lifecycle; readiness is never stored here.
    pub lifecycle: LifecycleState,
    /// Evidence required for a graph-authorized DONE transition.
    pub completion: CompletionContract,
    /// Import facts that must not be mistaken for completion evidence.
    pub metadata: BTreeMap<String, String>,
}

impl GraphNode {
    /// Create a planned node with an explicit completion contract.
    pub fn new(
        id: NodeId,
        kind: NodeKind,
        title: impl Into<String>,
        path: Option<GraphPath>,
        completion: CompletionContract,
    ) -> Self {
        Self {
            id,
            kind,
            title: title.into(),
            path,
            parent: None,
            lifecycle: LifecycleState::Planned,
            completion,
            metadata: BTreeMap::new(),
        }
    }
}

/// A validation severity.
#[doc = "SERDE-TAG-JUSTIFICATION: issue levels use stable scalar wire values within the read-only graph contract."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueLevel {
    /// The graph cannot safely derive execution state.
    Error,
    /// The importer preserved uncertainty without inventing a relation.
    Warning,
}

/// One graph validation or migration finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphIssue {
    /// Severity used by `graph validate`.
    pub level: IssueLevel,
    /// Stable machine-readable issue code.
    pub code: String,
    /// Optional affected node.
    pub node: Option<NodeId>,
    /// Human-readable explanation.
    pub message: String,
}

/// A graph issue summary used by every read-only CLI view.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Number of graph nodes.
    pub node_count: usize,
    /// Number of graph edges.
    pub edge_count: usize,
    /// Validation findings.
    pub issues: Vec<GraphIssue>,
}

impl ValidationReport {
    /// Whether no error-level finding exists.
    pub fn is_valid(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.level == IssueLevel::Error)
    }
}

/// A derived node view for status, ready, blocked, and inspect commands.
#[derive(Debug, Clone)]
pub struct NodeStatus {
    /// Stable node ID.
    pub id: NodeId,
    /// Node category.
    pub kind: NodeKind,
    /// Human-readable title.
    pub title: String,
    /// Derived lifecycle/readiness state.
    pub state: DerivedState,
    /// Optional evidence path.
    pub path: Option<GraphPath>,
    /// Reasons the node is blocked or not complete.
    pub reasons: Vec<String>,
}

/// Catalog coverage counts kept separate from workpack lifecycle counts.
#[derive(Debug, Clone, Default)]
pub struct CatalogSummary {
    /// Total ledger records.
    pub total: usize,
    /// Available source records.
    pub available: usize,
    /// Explicit source-unavailable records.
    pub source_unavailable: usize,
    /// Rows with complete CP08 decomposition evidence.
    pub decomposed_complete: usize,
    /// Rows with partial CP08 decomposition evidence.
    pub decomposed_partial: usize,
    /// Rows with complete native implementation evidence.
    pub native_complete: usize,
    /// Rows with partial native implementation evidence.
    pub native_partial: usize,
    /// Rows with complete executable-proof evidence.
    pub proof_complete: usize,
    /// Rows with partial executable-proof evidence.
    pub proof_partial: usize,
}

/// Intent-family and packet counts kept separate from coverage counts.
#[derive(Debug, Clone, Default)]
pub struct IntentSummary {
    /// Canonical intent families loaded from the matrix.
    pub family_count: usize,
    /// Available catalog IDs assigned to a family.
    pub mapped_skill_count: usize,
    /// Graph-derived bounded action packets.
    pub packet_count: usize,
    /// Native CP09/CP12 packets.
    pub native_packet_count: usize,
    /// Advisory/manual CP11 packets.
    pub retention_packet_count: usize,
    /// Protected identities excluded from the matrix.
    pub protected_excluded: usize,
}

/// Whole-plan status output.
#[derive(Debug, Clone)]
pub struct StatusReport {
    /// Graph validation summary.
    pub validation: ValidationReport,
    /// Imported node counts by kind.
    pub nodes_by_kind: BTreeMap<NodeKind, usize>,
    /// Workpack status views.
    pub workpacks: Vec<NodeStatus>,
    /// Catalog coverage snapshot.
    pub catalog: CatalogSummary,
    /// Intent routing counts kept independent from implementation coverage.
    pub intent: IntentSummary,
}

/// A blocked node and its mechanical reasons.
#[derive(Debug, Clone)]
pub struct BlockedReport {
    /// Blocked workpack or dependency node.
    pub node: NodeStatus,
}

/// Dependency explanation attached to `graph why`.
#[derive(Debug, Clone)]
pub struct WhyReport {
    /// Requested node.
    pub requested: NodeId,
    /// Dependency chain followed in deterministic order.
    pub chain: Vec<NodeId>,
    /// First observed blockers.
    pub blockers: Vec<String>,
}

/// The in-memory graph and its repository root.
#[derive(Debug, Clone)]
pub struct CyberPlanGraph {
    root: PathBuf,
    manifest: manifest::GraphManifest,
    nodes: BTreeMap<NodeId, GraphNode>,
    edges: BTreeSet<GraphEdge>,
    issues: Vec<GraphIssue>,
    cp08_component_kinds: BTreeMap<String, BTreeSet<String>>,
}

#[path = "cyber_graph/api.rs"]
mod api;
#[path = "cyber_graph/catalog.rs"]
mod catalog;
#[path = "cyber_graph/evidence.rs"]
mod evidence;
#[path = "cyber_graph/imports.rs"]
mod imports;
#[path = "cyber_graph/intent.rs"]
mod intent;
#[path = "cyber_graph/json.rs"]
mod json;
#[path = "cyber_graph/manifest.rs"]
pub mod manifest;
#[path = "cyber_graph/manifest_wire.rs"]
mod manifest_wire;
#[path = "cyber_graph/state.rs"]
mod state;
#[path = "cyber_graph/text.rs"]
mod text;
#[path = "cyber_graph/validation.rs"]
mod validation;
#[path = "cyber_graph/validation_summary.rs"]
mod validation_summary;
#[path = "cyber_graph/wire.rs"]
mod wire;

#[cfg(test)]
mod tests {
    use super::{GraphError, GraphPath, NodeId};

    #[test]
    fn identifiers_and_paths_reject_escape_values() {
        assert!(matches!(NodeId::try_new("WP/cp00"), Ok(_)));
        assert!(matches!(
            NodeId::try_new("../escape"),
            Err(GraphError::InvalidValue(_))
        ));
        assert!(matches!(
            GraphPath::try_new("docs/plans/cyberskills-parity-plan/README.md"),
            Ok(_)
        ));
        assert!(matches!(
            GraphPath::try_new("../vendor/file"),
            Err(GraphError::InvalidValue(_))
        ));
    }
}
