//! BOUNDARY-INVARIANT: raw Cyber Plan manifests, Markdown evidence, and JSON
//! catalog/proof inputs are decoded here and converted into validated graph
//! values before execution state is derived.
//! NEGATIVE-TEST: malformed paths, unknown dependencies, missing evidence,
//! protected-source access, cycles, and incomplete DONE contracts are rejected
//! by the graph validation tests.
//! SERIALIZATION-DOC: the public graph views are stable JSON wire outputs for
//! the read-only CLI; serde does not imply implementation or proof completion.
//!
//! Repo-owned execution graph for the CyberSkills plan.
//!
//! Markdown remains the detailed intent and acceptance source for the plan.
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
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
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
        if valid {
            Ok(Self(value))
        } else {
            Err(GraphError::InvalidValue(format!(
                "invalid node id `{value}`"
            )))
        }
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
        if !is_safe_relative_path(&value) {
            return Err(GraphError::InvalidValue(format!(
                "invalid graph-relative path `{value}`"
            )));
        }
        Ok(Self(value))
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
#[doc = "SERDE-TAG-JUSTIFICATION: graph node categories use stable scalar wire values for the read-only graph contract."]
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
#[doc = "SERDE-TAG-JUSTIFICATION: lifecycle states use stable scalar wire values for the read-only graph contract."]
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
#[doc = "SERDE-TAG-JUSTIFICATION: derived states use stable scalar wire values for the read-only graph contract."]
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

/// Coverage is intentionally independent for implementation and proof.
#[doc = "SERDE-TAG-JUSTIFICATION: coverage levels use stable scalar wire values for the read-only graph contract."]
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
#[doc = "SERDE-TAG-JUSTIFICATION: edge kinds use stable scalar wire values for the read-only graph contract."]
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

/// The mechanically checked completion contract for a node.
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
    /// Parent node, if the node belongs to a hierarchy.
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
#[doc = "SERDE-TAG-JUSTIFICATION: issue levels use stable scalar wire values for the read-only graph contract."]
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

/// The checked-in graph manifest's seed node.
#[derive(Debug, Clone)]
pub struct SeedNode {
    /// Stable seed ID.
    pub id: NodeId,
    /// Seed title.
    pub title: String,
    /// Detailed Markdown source.
    pub path: GraphPath,
}

/// Which repository sources are imported by the graph loader.
#[derive(Debug, Clone)]
pub struct ImportConfig {
    /// Import CP00-CP13 Markdown workpacks.
    pub workpacks: bool,
    /// Import the Universal Language dependency workpacks as first-class nodes.
    pub dependency_workpacks: bool,
    /// Import the CyberSkills disposition catalog rows.
    pub catalog: bool,
    /// Import immutable CP08 decomposition evidence.
    pub cp08_proofs: bool,
    /// Import CP01 existing-rule reconciliation evidence.
    pub cp01_proofs: bool,
    /// Import the intent-family and bounded-packet matrix.
    pub intent_matrix: bool,
    /// Import CP11 advisory/manual retention packets.
    pub cp11_proofs: bool,
}

impl Default for ImportConfig {
    fn default() -> Self {
        Self {
            workpacks: true,
            dependency_workpacks: false,
            catalog: true,
            cp08_proofs: true,
            cp01_proofs: true,
            intent_matrix: false,
            cp11_proofs: true,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Optional lifecycle/dependency corrections that are themselves reviewable.
#[derive(Debug, Clone, Default)]
pub struct GraphOverrides {
    /// Explicit lifecycle state, never inferred from prose status labels.
    pub lifecycle: BTreeMap<NodeId, LifecycleState>,
    /// Explicit dependency corrections for a migration ambiguity.
    pub dependencies: BTreeMap<NodeId, Vec<NodeId>>,
    /// Recorded gate evidence required for an explicit DONE transition.
    pub evidence: BTreeMap<NodeId, CompletionEvidence>,
}

/// A durable record of one externally executed graph gate.
///
/// The graph stores the attestation and source anchors, not the ignored local
/// harness logs. This keeps the control plane portable while making a DONE
/// transition impossible without an explicit passed command, commit, and
/// non-proof boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionEvidence {
    /// Stable run identifier from the direct command or Enforcer harness.
    pub run_id: String,
    /// Exact command that produced the recorded result.
    pub command: String,
    /// Recorded result status; only `passed` can satisfy a contract.
    pub status: String,
    /// Process exit code; only zero can satisfy a contract.
    pub exit_code: i32,
    /// Commit whose source was tested.
    pub commit: String,
    /// Tracked source/evidence anchors inspected by the command.
    pub source_paths: Vec<GraphPath>,
    /// Claims supported by this gate record.
    pub proves: Vec<String>,
    /// Explicit boundaries that this gate does not establish.
    pub does_not_prove: Vec<String>,
}

impl CompletionEvidence {
    fn validate(&self, node: &NodeId) -> Result<(), GraphError> {
        if self.run_id.trim().is_empty()
            || self.command.trim().is_empty()
            || self.commit.trim().is_empty()
            || self.status != "passed"
            || self.exit_code != 0
            || self.source_paths.is_empty()
            || self.proves.is_empty()
            || self.does_not_prove.is_empty()
        {
            return Err(GraphError::InvalidValue(format!(
                "completion evidence `{node}` is incomplete or not passed"
            )));
        }
        if !self
            .commit
            .chars()
            .all(|character| character.is_ascii_hexdigit())
            || self.commit.len() < 12
        {
            return Err(GraphError::InvalidValue(format!(
                "completion evidence `{node}` has an invalid commit"
            )));
        }
        Ok(())
    }
}

/// The repo-owned Cyber Plan graph manifest.
#[derive(Debug, Clone)]
pub struct GraphManifest {
    /// Schema version for future migrations.
    pub schema_version: u32,
    /// Graph identity.
    pub graph_id: NodeId,
    /// Root goal seed.
    pub goal: SeedNode,
    /// CyberSkills plan seed.
    pub plan: SeedNode,
    /// Existing routing index.
    pub workpack_index: GraphPath,
    /// Existing Markdown workpack directory.
    pub workpack_root: GraphPath,
    /// Optional dependency-plan workpack index imported as first-class nodes.
    pub dependency_workpack_index: Option<GraphPath>,
    /// Optional dependency-plan workpack directory.
    pub dependency_workpack_root: Option<GraphPath>,
    /// Existing proof/gate table.
    pub test_proof_expectations: GraphPath,
    /// Existing v3 disposition ledger.
    pub catalog_path: GraphPath,
    /// Optional intent-family and bounded-packet matrix.
    pub intent_matrix_path: Option<GraphPath>,
    /// Existing proof roots; paths are evidence, not source authority.
    pub proof_roots: Vec<GraphPath>,
    /// Existing ADR roots.
    pub decision_roots: Vec<GraphPath>,
    /// Import switches.
    pub import: ImportConfig,
    /// Human-reviewed migration corrections.
    pub overrides: GraphOverrides,
}

impl GraphManifest {
    fn validate(&self) -> Result<(), GraphError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(GraphError::InvalidValue(format!(
                "unsupported graph schemaVersion {}; expected {SCHEMA_VERSION}",
                self.schema_version
            )));
        }
        let paths = [
            &self.goal.path,
            &self.plan.path,
            &self.workpack_index,
            &self.workpack_root,
            &self.test_proof_expectations,
            &self.catalog_path,
        ];
        if paths
            .iter()
            .any(|path| !is_safe_relative_path(path.as_str()))
        {
            return Err(GraphError::InvalidValue(
                "manifest contains an unsafe relative path".to_owned(),
            ));
        }
        if self
            .intent_matrix_path
            .as_ref()
            .is_some_and(|path| !is_safe_relative_path(path.as_str()))
        {
            return Err(GraphError::InvalidValue(
                "manifest contains an unsafe intent matrix path".to_owned(),
            ));
        }
        if self
            .dependency_workpack_index
            .as_ref()
            .is_some_and(|path| !is_safe_relative_path(path.as_str()))
            || self
                .dependency_workpack_root
                .as_ref()
                .is_some_and(|path| !is_safe_relative_path(path.as_str()))
        {
            return Err(GraphError::InvalidValue(
                "manifest contains an unsafe dependency workpack path".to_owned(),
            ));
        }
        let mut run_ids = BTreeSet::new();
        for (node, evidence) in &self.overrides.evidence {
            evidence.validate(node)?;
            if !run_ids.insert(evidence.run_id.as_str()) {
                return Err(GraphError::InvalidValue(format!(
                    "completion evidence run `{}` is reused",
                    evidence.run_id
                )));
            }
        }
        Ok(())
    }
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
    /// Evidence path, if any.
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

/// A dependency explanation for `graph why`.
#[derive(Debug, Clone)]
pub struct WhyReport {
    /// Requested node.
    pub requested: NodeId,
    /// Dependency chain followed in deterministic order.
    pub chain: Vec<NodeId>,
    /// First observed blockers.
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SeedNodeWire {
    id: NodeId,
    title: String,
    path: GraphPath,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ImportConfigWire {
    #[doc = "DEFAULT-JUSTIFICATION: omitted workpack import switches retain the enabled default."]
    #[serde(default = "default_true")]
    workpacks: bool,
    #[doc = "DEFAULT-JUSTIFICATION: dependency workpack import is opt-in for v1 compatibility."]
    #[serde(default)]
    dependency_workpacks: bool,
    #[doc = "DEFAULT-JUSTIFICATION: omitted catalog import switches retain the enabled default."]
    #[serde(default = "default_true")]
    catalog: bool,
    #[doc = "DEFAULT-JUSTIFICATION: omitted CP08 import switches retain the enabled default."]
    #[serde(default = "default_true")]
    cp08_proofs: bool,
    #[doc = "DEFAULT-JUSTIFICATION: omitted CP01 import switches retain the enabled default."]
    #[serde(default = "default_true")]
    cp01_proofs: bool,
    #[doc = "DEFAULT-JUSTIFICATION: omitted intent-matrix imports preserve the v1 graph default. "]
    #[serde(default)]
    intent_matrix: bool,
    #[doc = "DEFAULT-JUSTIFICATION: omitted CP11 import switches retain the enabled default."]
    #[serde(default = "default_true")]
    cp11_proofs: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GraphOverridesWire {
    #[doc = "DEFAULT-JUSTIFICATION: absent lifecycle overrides preserve imported lifecycle values."]
    #[serde(default)]
    lifecycle: BTreeMap<NodeId, LifecycleState>,
    #[doc = "DEFAULT-JUSTIFICATION: absent dependency overrides preserve imported dependencies."]
    #[serde(default)]
    dependencies: BTreeMap<NodeId, Vec<NodeId>>,
    #[doc = "DEFAULT-JUSTIFICATION: absent gate evidence preserves the conservative non-DONE default."]
    #[serde(default)]
    evidence: BTreeMap<NodeId, CompletionEvidenceWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompletionEvidenceWire {
    run_id: String,
    command: String,
    status: String,
    exit_code: i32,
    commit: String,
    source_paths: Vec<GraphPath>,
    proves: Vec<String>,
    does_not_prove: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GraphManifestWire {
    schema_version: u32,
    graph_id: NodeId,
    goal: SeedNodeWire,
    plan: SeedNodeWire,
    workpack_index: GraphPath,
    workpack_root: GraphPath,
    #[doc = "DEFAULT-JUSTIFICATION: v1 manifests may omit the optional dependency workpack index."]
    #[serde(default)]
    dependency_workpack_index: Option<GraphPath>,
    #[doc = "DEFAULT-JUSTIFICATION: v1 manifests may omit the optional dependency workpack root."]
    #[serde(default)]
    dependency_workpack_root: Option<GraphPath>,
    test_proof_expectations: GraphPath,
    catalog_path: GraphPath,
    #[doc = "DEFAULT-JUSTIFICATION: v1 manifests may omit the optional intent matrix. "]
    #[serde(default)]
    intent_matrix_path: Option<GraphPath>,
    #[doc = "DEFAULT-JUSTIFICATION: absent proof roots mean the manifest declares no proof root."]
    #[serde(default)]
    proof_roots: Vec<GraphPath>,
    #[doc = "DEFAULT-JUSTIFICATION: absent decision roots mean the manifest declares no ADR root."]
    #[serde(default)]
    decision_roots: Vec<GraphPath>,
    #[doc = "DEFAULT-JUSTIFICATION: absent import switches use the documented default importer policy."]
    #[serde(default)]
    import: ImportConfigWire,
    #[doc = "DEFAULT-JUSTIFICATION: absent overrides mean no human-reviewed migration correction."]
    #[serde(default)]
    overrides: GraphOverridesWire,
}

impl From<SeedNodeWire> for SeedNode {
    fn from(value: SeedNodeWire) -> Self {
        Self {
            id: value.id,
            title: value.title,
            path: value.path,
        }
    }
}

impl From<ImportConfigWire> for ImportConfig {
    fn from(value: ImportConfigWire) -> Self {
        Self {
            workpacks: value.workpacks,
            dependency_workpacks: value.dependency_workpacks,
            catalog: value.catalog,
            cp08_proofs: value.cp08_proofs,
            cp01_proofs: value.cp01_proofs,
            intent_matrix: value.intent_matrix,
            cp11_proofs: value.cp11_proofs,
        }
    }
}

impl From<GraphOverridesWire> for GraphOverrides {
    fn from(value: GraphOverridesWire) -> Self {
        Self {
            lifecycle: value.lifecycle,
            dependencies: value.dependencies,
            evidence: value
                .evidence
                .into_iter()
                .map(|(node, evidence)| {
                    (
                        node,
                        CompletionEvidence {
                            run_id: evidence.run_id,
                            command: evidence.command,
                            status: evidence.status,
                            exit_code: evidence.exit_code,
                            commit: evidence.commit,
                            source_paths: evidence.source_paths,
                            proves: evidence.proves,
                            does_not_prove: evidence.does_not_prove,
                        },
                    )
                })
                .collect(),
        }
    }
}

impl From<GraphManifestWire> for GraphManifest {
    fn from(value: GraphManifestWire) -> Self {
        Self {
            schema_version: value.schema_version,
            graph_id: value.graph_id,
            goal: value.goal.into(),
            plan: value.plan.into(),
            workpack_index: value.workpack_index,
            workpack_root: value.workpack_root,
            dependency_workpack_index: value.dependency_workpack_index,
            dependency_workpack_root: value.dependency_workpack_root,
            test_proof_expectations: value.test_proof_expectations,
            catalog_path: value.catalog_path,
            intent_matrix_path: value.intent_matrix_path,
            proof_roots: value.proof_roots,
            decision_roots: value.decision_roots,
            import: value.import.into(),
            overrides: value.overrides.into(),
        }
    }
}

impl From<&SeedNode> for SeedNodeWire {
    fn from(value: &SeedNode) -> Self {
        Self {
            id: value.id.clone(),
            title: value.title.clone(),
            path: value.path.clone(),
        }
    }
}

impl From<&ImportConfig> for ImportConfigWire {
    fn from(value: &ImportConfig) -> Self {
        Self {
            workpacks: value.workpacks,
            dependency_workpacks: value.dependency_workpacks,
            catalog: value.catalog,
            cp08_proofs: value.cp08_proofs,
            cp01_proofs: value.cp01_proofs,
            intent_matrix: value.intent_matrix,
            cp11_proofs: value.cp11_proofs,
        }
    }
}

impl From<&GraphOverrides> for GraphOverridesWire {
    fn from(value: &GraphOverrides) -> Self {
        Self {
            lifecycle: value.lifecycle.clone(),
            dependencies: value.dependencies.clone(),
            evidence: value
                .evidence
                .iter()
                .map(|(node, evidence)| {
                    (
                        node.clone(),
                        CompletionEvidenceWire {
                            run_id: evidence.run_id.clone(),
                            command: evidence.command.clone(),
                            status: evidence.status.clone(),
                            exit_code: evidence.exit_code,
                            commit: evidence.commit.clone(),
                            source_paths: evidence.source_paths.clone(),
                            proves: evidence.proves.clone(),
                            does_not_prove: evidence.does_not_prove.clone(),
                        },
                    )
                })
                .collect(),
        }
    }
}

impl From<&GraphManifest> for GraphManifestWire {
    fn from(value: &GraphManifest) -> Self {
        Self {
            schema_version: value.schema_version,
            graph_id: value.graph_id.clone(),
            goal: (&value.goal).into(),
            plan: (&value.plan).into(),
            workpack_index: value.workpack_index.clone(),
            workpack_root: value.workpack_root.clone(),
            dependency_workpack_index: value.dependency_workpack_index.clone(),
            dependency_workpack_root: value.dependency_workpack_root.clone(),
            test_proof_expectations: value.test_proof_expectations.clone(),
            catalog_path: value.catalog_path.clone(),
            intent_matrix_path: value.intent_matrix_path.clone(),
            proof_roots: value.proof_roots.clone(),
            decision_roots: value.decision_roots.clone(),
            import: (&value.import).into(),
            overrides: (&value.overrides).into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct GraphIssueWire {
    level: IssueLevel,
    code: String,
    node: Option<NodeId>,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
struct ValidationReportWire {
    node_count: usize,
    edge_count: usize,
    issues: Vec<GraphIssueWire>,
}

#[derive(Debug, Clone, Serialize)]
struct NodeStatusWire {
    id: NodeId,
    kind: NodeKind,
    title: String,
    state: DerivedState,
    path: Option<GraphPath>,
    reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CatalogSummaryWire {
    total: usize,
    available: usize,
    source_unavailable: usize,
    decomposed_complete: usize,
    decomposed_partial: usize,
    native_complete: usize,
    native_partial: usize,
    proof_complete: usize,
    proof_partial: usize,
}

#[derive(Debug, Clone, Serialize)]
struct IntentSummaryWire {
    family_count: usize,
    mapped_skill_count: usize,
    packet_count: usize,
    native_packet_count: usize,
    retention_packet_count: usize,
    protected_excluded: usize,
}

#[derive(Debug, Clone, Serialize)]
struct StatusReportWire {
    validation: ValidationReportWire,
    nodes_by_kind: BTreeMap<NodeKind, usize>,
    workpacks: Vec<NodeStatusWire>,
    catalog: CatalogSummaryWire,
    intent: IntentSummaryWire,
}

#[derive(Debug, Clone, Serialize)]
struct BlockedReportWire {
    node: NodeStatusWire,
}

#[derive(Debug, Clone, Serialize)]
struct WhyReportWire {
    requested: NodeId,
    chain: Vec<NodeId>,
    blockers: Vec<String>,
}

impl From<&GraphIssue> for GraphIssueWire {
    fn from(value: &GraphIssue) -> Self {
        Self {
            level: value.level,
            code: value.code.clone(),
            node: value.node.clone(),
            message: value.message.clone(),
        }
    }
}

impl From<&ValidationReport> for ValidationReportWire {
    fn from(value: &ValidationReport) -> Self {
        Self {
            node_count: value.node_count,
            edge_count: value.edge_count,
            issues: value.issues.iter().map(Into::into).collect(),
        }
    }
}

impl From<&NodeStatus> for NodeStatusWire {
    fn from(value: &NodeStatus) -> Self {
        Self {
            id: value.id.clone(),
            kind: value.kind,
            title: value.title.clone(),
            state: value.state,
            path: value.path.clone(),
            reasons: value.reasons.clone(),
        }
    }
}

impl From<&CatalogSummary> for CatalogSummaryWire {
    fn from(value: &CatalogSummary) -> Self {
        Self {
            total: value.total,
            available: value.available,
            source_unavailable: value.source_unavailable,
            decomposed_complete: value.decomposed_complete,
            decomposed_partial: value.decomposed_partial,
            native_complete: value.native_complete,
            native_partial: value.native_partial,
            proof_complete: value.proof_complete,
            proof_partial: value.proof_partial,
        }
    }
}

impl From<&IntentSummary> for IntentSummaryWire {
    fn from(value: &IntentSummary) -> Self {
        Self {
            family_count: value.family_count,
            mapped_skill_count: value.mapped_skill_count,
            packet_count: value.packet_count,
            native_packet_count: value.native_packet_count,
            retention_packet_count: value.retention_packet_count,
            protected_excluded: value.protected_excluded,
        }
    }
}

impl From<&StatusReport> for StatusReportWire {
    fn from(value: &StatusReport) -> Self {
        Self {
            validation: (&value.validation).into(),
            nodes_by_kind: value.nodes_by_kind.clone(),
            workpacks: value.workpacks.iter().map(Into::into).collect(),
            catalog: (&value.catalog).into(),
            intent: (&value.intent).into(),
        }
    }
}

impl From<&BlockedReport> for BlockedReportWire {
    fn from(value: &BlockedReport) -> Self {
        Self {
            node: (&value.node).into(),
        }
    }
}

impl From<&WhyReport> for WhyReportWire {
    fn from(value: &WhyReport) -> Self {
        Self {
            requested: value.requested.clone(),
            chain: value.chain.clone(),
            blockers: value.blockers.clone(),
        }
    }
}

/// The in-memory graph and its repository root.
#[derive(Debug, Clone)]
pub struct CyberPlanGraph {
    root: PathBuf,
    manifest: GraphManifest,
    nodes: BTreeMap<NodeId, GraphNode>,
    edges: BTreeSet<GraphEdge>,
    issues: Vec<GraphIssue>,
}

impl CyberPlanGraph {
    /// Create an empty graph for unit/integration tests.
    pub fn new_for_root(root: impl Into<PathBuf>, manifest: GraphManifest) -> Self {
        Self {
            root: root.into(),
            manifest,
            nodes: BTreeMap::new(),
            edges: BTreeSet::new(),
            issues: Vec::new(),
        }
    }

    /// Load the Cyber Plan graph from the checked-in manifest and sources.
    pub fn load(root: impl AsRef<Path>) -> Result<Self, GraphError> {
        let root = root.as_ref().to_path_buf();
        let manifest_file = root.join(GRAPH_MANIFEST_PATH);
        let manifest_wire: GraphManifestWire =
            serde_json::from_str(&fs::read_to_string(manifest_file)?)?;
        let manifest: GraphManifest = manifest_wire.into();
        manifest.validate()?;
        let mut graph = Self::new_for_root(root, manifest);
        graph.import_seeds()?;
        if graph.manifest.import.dependency_workpacks {
            graph.import_dependency_workpacks()?;
        }
        if graph.manifest.import.workpacks {
            graph.import_workpacks()?;
        }
        if graph.manifest.import.cp01_proofs {
            graph.import_cp01_proofs()?;
        }
        if graph.manifest.import.cp08_proofs {
            graph.import_cp08_proofs()?;
        }
        if graph.manifest.import.cp11_proofs {
            graph.import_cp11_proofs()?;
        }
        if graph.manifest.import.catalog {
            graph.import_catalog()?;
        }
        if graph.manifest.import.intent_matrix {
            graph.import_intent_matrix()?;
        }
        graph.apply_overrides()?;
        Ok(graph)
    }

    /// Add a node, rejecting duplicate stable IDs.
    pub fn add_node(&mut self, node: GraphNode) -> Result<(), GraphError> {
        if self.nodes.insert(node.id.clone(), node).is_some() {
            return Err(GraphError::InvalidValue(
                "duplicate graph node id".to_owned(),
            ));
        }
        Ok(())
    }

    /// Add a graph edge. Endpoint existence is checked by `validate` so
    /// missing dependency edges remain visible as explicit errors.
    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.edges.insert(edge);
    }

    /// Borrow a node by stable ID.
    pub fn node(&self, id: &NodeId) -> Option<&GraphNode> {
        self.nodes.get(id)
    }

    /// Borrow all nodes in deterministic ID order.
    pub fn nodes(&self) -> impl Iterator<Item = &GraphNode> {
        self.nodes.values()
    }

    /// Validate IDs, endpoints, dependencies, cycles, protected coverage,
    /// and DONE contracts without changing graph state.
    pub fn validate(&self) -> ValidationReport {
        let mut issues = self.issues.clone();
        issues.extend(self.endpoint_issues());
        issues.extend(self.cycle_issues());
        issues.extend(self.done_contract_issues());
        issues.extend(self.protected_issues());
        ValidationReport {
            node_count: self.nodes.len(),
            edge_count: self.edges.len(),
            issues,
        }
    }

    /// Compute the complete Cyber Plan status view.
    pub fn status(&self) -> StatusReport {
        let validation = self.validate();
        let mut nodes_by_kind = BTreeMap::new();
        for node in self.nodes.values() {
            *nodes_by_kind.entry(node.kind).or_insert(0) += 1;
        }
        let mut workpacks: Vec<NodeStatus> = self
            .nodes
            .values()
            .filter(|node| node.kind == NodeKind::Workpack)
            .map(|node| self.node_status(node))
            .collect();
        workpacks.sort_by(|left, right| left.id.cmp(&right.id));
        StatusReport {
            validation,
            nodes_by_kind,
            workpacks,
            catalog: self.catalog_summary(),
            intent: self.intent_summary(),
        }
    }

    /// Return workpacks whose hard dependencies are satisfied.
    pub fn ready(&self) -> Vec<NodeStatus> {
        self.nodes
            .values()
            .filter(|node| node.kind == NodeKind::Workpack)
            .map(|node| self.node_status(node))
            .filter(|status| status.state == DerivedState::Ready)
            .collect()
    }

    /// Return workpacks blocked by dependencies, ambiguity, or integrity.
    pub fn blocked(&self) -> Vec<BlockedReport> {
        self.nodes
            .values()
            .filter(|node| node.kind == NodeKind::Workpack)
            .map(|node| self.node_status(node))
            .filter(|status| status.state == DerivedState::Blocked)
            .map(|node| BlockedReport { node })
            .collect()
    }

    /// Inspect one node with its derived state and reasons.
    pub fn inspect(&self, id: &NodeId) -> Result<NodeStatus, GraphError> {
        self.nodes
            .get(id)
            .map(|node| self.node_status(node))
            .ok_or_else(|| GraphError::MissingNode(id.to_string()))
    }

    /// Explain the first dependency chain that prevents a node from running.
    pub fn why(&self, id: &NodeId) -> Result<WhyReport, GraphError> {
        if !self.nodes.contains_key(id) {
            return Err(GraphError::MissingNode(id.to_string()));
        }
        let mut chain = Vec::new();
        let mut blockers = Vec::new();
        self.explain(id, &mut chain, &mut blockers, &mut BTreeSet::new());
        Ok(WhyReport {
            requested: id.clone(),
            chain,
            blockers,
        })
    }

    /// Render the validation view through the explicit JSON wire boundary.
    pub fn validate_json(&self) -> Result<Value, GraphError> {
        Ok(serde_json::to_value(ValidationReportWire::from(
            &self.validate(),
        ))?)
    }

    /// Render the status view through the explicit JSON wire boundary.
    pub fn status_json(&self) -> Result<Value, GraphError> {
        Ok(serde_json::to_value(StatusReportWire::from(
            &self.status(),
        ))?)
    }

    /// Render ready workpacks through the explicit JSON wire boundary.
    pub fn ready_json(&self) -> Result<Value, GraphError> {
        let ready = self.ready();
        let wire: Vec<NodeStatusWire> = ready.iter().map(Into::into).collect();
        Ok(serde_json::to_value(wire)?)
    }

    /// Select the first dependency-legal workpack in stable graph order.
    pub fn next_json(&self) -> Result<Value, GraphError> {
        let validation = self.validate();
        let candidates = self.ready();
        let selected = candidates.first().map(NodeStatusWire::from);
        let decision = if !validation.is_valid() {
            "invalid"
        } else if selected.is_some() {
            "selected"
        } else if self
            .nodes
            .values()
            .any(|node| node.kind == NodeKind::Workpack)
        {
            "blocked"
        } else {
            "terminal"
        };
        let candidate_wire: Vec<NodeStatusWire> = candidates.iter().map(Into::into).collect();
        let excluded: Vec<BlockedReportWire> = self.blocked().iter().map(Into::into).collect();
        Ok(serde_json::json!({
            "decision": decision,
            "selected": selected,
            "candidates": candidate_wire,
            "excluded": excluded,
            "policy": {
                "requires": "derived ready state and all DependsOn nodes done",
                "order": "stable graph ID",
                "mutation": "none",
                "decompositionPromotesImplementation": false,
                "decompositionPromotesProof": false,
                "liveExternalExecution": "blocked",
                "protectedVendorSource": "excluded"
            },
            "validation": {
                "valid": validation.is_valid(),
                "nodeCount": validation.node_count,
                "edgeCount": validation.edge_count,
                "issues": validation.issues.iter().map(GraphIssueWire::from).collect::<Vec<_>>()
            }
        }))
    }

    /// Render blocked workpacks through the explicit JSON wire boundary.
    pub fn blocked_json(&self) -> Result<Value, GraphError> {
        let blocked = self.blocked();
        let wire: Vec<BlockedReportWire> = blocked.iter().map(Into::into).collect();
        Ok(serde_json::to_value(wire)?)
    }

    /// Render one inspected node through the explicit JSON wire boundary.
    pub fn inspect_json(&self, id: &NodeId) -> Result<Value, GraphError> {
        let status = self.inspect(id)?;
        Ok(serde_json::to_value(NodeStatusWire::from(&status))?)
    }

    /// Render one dependency explanation through the explicit JSON wire boundary.
    pub fn why_json(&self, id: &NodeId) -> Result<Value, GraphError> {
        let report = self.why(id)?;
        Ok(serde_json::to_value(WhyReportWire::from(&report))?)
    }

    /// Advance a lifecycle through the graph. DONE is only accepted when its
    /// hard dependencies and completion contract are both satisfied.
    pub fn transition(&mut self, id: &NodeId, target: LifecycleState) -> Result<(), GraphError> {
        let node = self
            .nodes
            .get(id)
            .ok_or_else(|| GraphError::MissingNode(id.to_string()))?;
        if target == LifecycleState::Done {
            let status = self.node_status(node);
            if !status.reasons.is_empty() || status.state == DerivedState::Blocked {
                return Err(GraphError::InvalidValue(format!(
                    "cannot mark `{id}` done: {}",
                    status.reasons.join("; ")
                )));
            }
            let contract = self.contract_result(node);
            if !contract.is_complete() {
                return Err(GraphError::InvalidValue(format!(
                    "cannot mark `{id}` done: {}",
                    contract.missing.join("; ")
                )));
            }
        }
        if let Some(node) = self.nodes.get_mut(id) {
            node.lifecycle = target;
        }
        Ok(())
    }

    fn import_seeds(&mut self) -> Result<(), GraphError> {
        let goal = GraphNode::new(
            self.manifest.goal.id.clone(),
            NodeKind::Goal,
            self.manifest.goal.title.clone(),
            Some(self.manifest.goal.path.clone()),
            CompletionContract::default(),
        );
        let plan = GraphNode::new(
            self.manifest.plan.id.clone(),
            NodeKind::Plan,
            self.manifest.plan.title.clone(),
            Some(self.manifest.plan.path.clone()),
            CompletionContract::default(),
        );
        self.add_node(goal)?;
        self.add_node(plan)?;
        self.add_edge(GraphEdge {
            from: self.manifest.goal.id.clone(),
            to: self.manifest.plan.id.clone(),
            kind: EdgeKind::Contains,
        });
        Ok(())
    }

    fn import_workpacks(&mut self) -> Result<(), GraphError> {
        let root = self.root.join(self.manifest.workpack_root.as_str());
        let proof_rows = self.read_proof_rows()?;
        let mut paths: Vec<PathBuf> = fs::read_dir(root)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("md"))
            .collect();
        paths.sort();
        let workpack_ids: BTreeMap<String, NodeId> = paths
            .iter()
            .filter_map(|path| {
                let stem = path.file_stem()?.to_str()?;
                let title = first_heading(&fs::read_to_string(path).ok()?)?;
                let key = workpack_key(&title, stem);
                NodeId::new(format!("WP/{key}")).ok().map(|id| (key, id))
            })
            .collect();
        for path in paths {
            self.import_one_workpack(&path, &workpack_ids, &proof_rows)?;
        }
        Ok(())
    }

    fn import_dependency_workpacks(&mut self) -> Result<(), GraphError> {
        let index_path = self
            .manifest
            .dependency_workpack_index
            .as_ref()
            .ok_or_else(|| {
                GraphError::InvalidValue(
                    "dependency workpack import is enabled without an index".to_owned(),
                )
            })?;
        let root_path = self
            .manifest
            .dependency_workpack_root
            .as_ref()
            .ok_or_else(|| {
                GraphError::InvalidValue(
                    "dependency workpack import is enabled without a root".to_owned(),
                )
            })?;
        let index = fs::read_to_string(self.root.join(index_path.as_str()))?;
        let root = self.root.join(root_path.as_str());
        let mut paths: Vec<PathBuf> = fs::read_dir(root)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("md"))
            .collect();
        paths.sort();
        let workpack_ids: BTreeMap<String, NodeId> = paths
            .iter()
            .filter_map(|path| {
                let stem = path.file_stem()?.to_str()?;
                let title = first_heading(&fs::read_to_string(path).ok()?)?;
                let key = workpack_key(&title, stem);
                key.starts_with("UL")
                    .then(|| NodeId::new(format!("EXT/{key}")))
                    .and_then(Result::ok)
                    .map(|id| (key, id))
            })
            .collect();
        for path in paths {
            self.import_one_dependency_workpack(&path, &workpack_ids, &index)?;
        }
        Ok(())
    }

    fn import_one_dependency_workpack(
        &mut self,
        path: &Path,
        workpack_ids: &BTreeMap<String, NodeId>,
        index: &str,
    ) -> Result<(), GraphError> {
        let contents = fs::read_to_string(path)?;
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                GraphError::InvalidValue("dependency workpack has no UTF-8 stem".to_owned())
            })?;
        let title = first_heading(&contents).unwrap_or_else(|| stem.to_owned());
        let key = workpack_key(&title, stem);
        if !key.starts_with("UL") {
            return Err(GraphError::InvalidValue(format!(
                "dependency workpack `{stem}` is not a UL workpack"
            )));
        }
        let id = NodeId::new(format!("EXT/{key}"))?;
        let index_row = parse_index_row(index, &key).ok_or_else(|| {
            GraphError::InvalidValue(format!(
                "dependency workpack `{key}` is missing from its index"
            ))
        })?;
        let relative = relative_path(&self.root, path)?;
        let mut node = GraphNode::new(
            id.clone(),
            NodeKind::Workpack,
            title,
            Some(relative),
            CompletionContract::default(),
        );
        node.metadata
            .insert("routingStatus".to_owned(), index_row.status);
        node.metadata
            .insert("ownerClass".to_owned(), index_row.owner);
        node.metadata
            .insert("batchLimit".to_owned(), index_row.batch_limit);
        node.metadata
            .insert("primaryOwns".to_owned(), index_row.owns);
        node.metadata.insert(
            "dependencyPlan".to_owned(),
            "universal-language-enforcement-plan".to_owned(),
        );
        node.metadata
            .insert("routingOnly".to_owned(), "true".to_owned());
        self.add_node(node)?;
        for dependency in dependency_tokens(&contents) {
            let target = dependency_target(&dependency, workpack_ids)?;
            if target.as_str().starts_with("EXT/") && !self.nodes.contains_key(&target) {
                self.add_node(external_dependency(&target, &dependency))?;
            }
            self.add_edge(GraphEdge {
                from: id.clone(),
                to: target,
                kind: EdgeKind::DependsOn,
            });
        }
        Ok(())
    }

    fn import_one_workpack(
        &mut self,
        path: &Path,
        workpack_ids: &BTreeMap<String, NodeId>,
        proof_rows: &BTreeMap<String, ProofRow>,
    ) -> Result<(), GraphError> {
        let contents = fs::read_to_string(path)?;
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| GraphError::InvalidValue("workpack has no UTF-8 stem".to_owned()))?;
        let relative = relative_path(&self.root, path)?;
        let title = first_heading(&contents).unwrap_or_else(|| stem.to_owned());
        let key = workpack_key(&title, stem);
        let id = NodeId::new(format!("WP/{key}"))?;
        let index = parse_index_row(
            &fs::read_to_string(self.root.join(self.manifest.workpack_index.as_str()))?,
            &key,
        );
        let proof_row = proof_rows.get(&key);
        let mut completion = completion_contract(proof_row);
        let checklist = checklist_counts(&contents);
        completion.checklist_total = checklist.0;
        completion.checklist_complete = checklist.1;
        let mut node = GraphNode::new(
            id.clone(),
            NodeKind::Workpack,
            title,
            Some(relative),
            completion.clone(),
        );
        if let Some(index) = &index {
            node.metadata
                .insert("routingStatus".to_owned(), index.status.clone());
            node.metadata
                .insert("ownerClass".to_owned(), index.owner.clone());
            node.metadata
                .insert("batchLimit".to_owned(), index.batch_limit.clone());
            node.metadata
                .insert("primaryOwns".to_owned(), index.owns.clone());
        }
        if let Some(row) = proof_row {
            node.metadata
                .insert("proofRowState".to_owned(), row.state.clone());
        }
        self.add_node(node)?;
        self.add_edge(GraphEdge {
            from: self.manifest.plan.id.clone(),
            to: id.clone(),
            kind: EdgeKind::Contains,
        });
        for evidence_id in completion
            .required_tests
            .iter()
            .chain(completion.required_proofs.iter())
        {
            if !self.nodes.contains_key(evidence_id) {
                let kind = if evidence_id.as_str().starts_with("TEST/") {
                    NodeKind::Test
                } else {
                    NodeKind::Proof
                };
                self.add_node(GraphNode::new(
                    evidence_id.clone(),
                    kind,
                    "Named completion-contract evidence",
                    None,
                    CompletionContract::default(),
                ))?;
            }
        }
        for requirement in checklist_nodes(&id, &contents)? {
            self.add_node(requirement.clone())?;
            self.add_edge(GraphEdge {
                from: id.clone(),
                to: requirement.id,
                kind: EdgeKind::Contains,
            });
        }
        for dependency in dependency_tokens(&contents) {
            let target = dependency_target(&dependency, workpack_ids)?;
            if target.as_str().starts_with("EXT/") && !self.nodes.contains_key(&target) {
                self.add_node(external_dependency(&target, &dependency))?;
            }
            self.add_edge(GraphEdge {
                from: id.clone(),
                to: target,
                kind: EdgeKind::DependsOn,
            });
        }
        for proof in &completion.required_proofs {
            self.add_edge(GraphEdge {
                from: id.clone(),
                to: proof.clone(),
                kind: EdgeKind::Requires,
            });
        }
        for test in &completion.required_tests {
            self.add_edge(GraphEdge {
                from: id.clone(),
                to: test.clone(),
                kind: EdgeKind::Requires,
            });
        }
        Ok(())
    }

    fn import_intent_matrix(&mut self) -> Result<(), GraphError> {
        let path = self
            .manifest
            .intent_matrix_path
            .as_ref()
            .ok_or_else(|| {
                GraphError::InvalidValue(
                    "intent matrix import is enabled without a path".to_owned(),
                )
            })?
            .clone();
        let matrix: Value =
            serde_json::from_str(&fs::read_to_string(self.root.join(path.as_str()))?)?;
        validate_intent_matrix_header(&matrix)?;
        let families = matrix
            .get("families")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                GraphError::InvalidValue("intent matrix families are missing".to_owned())
            })?;
        let mut assigned = BTreeSet::new();
        let mut owned_components = BTreeSet::new();
        for family in families {
            self.import_intent_family(family, &path, &mut assigned, &mut owned_components)?;
        }
        self.validate_intent_partition(&assigned)?;
        Ok(())
    }

    fn import_intent_family(
        &mut self,
        family: &Value,
        matrix_path: &GraphPath,
        assigned: &mut BTreeSet<String>,
        owned_components: &mut BTreeSet<String>,
    ) -> Result<(), GraphError> {
        let family_id = string_field(family, &["familyId"])
            .ok_or_else(|| GraphError::InvalidValue("intent family has no familyId".to_owned()))?;
        let intent = string_field(family, &["intent"]).ok_or_else(|| {
            GraphError::InvalidValue(format!("intent family `{family_id}` has no intent"))
        })?;
        let skill_ids = required_string_array(family, &["skillIds"], &family_id)?;
        let family_node = NodeId::new(family_id.clone())?;
        let mut node = GraphNode::new(
            family_node.clone(),
            NodeKind::IntentFamily,
            intent,
            Some(matrix_path.clone()),
            CompletionContract {
                required_paths: vec![matrix_path.clone()],
                ..CompletionContract::default()
            },
        );
        node.metadata
            .insert("familyId".to_owned(), family_id.clone());
        node.metadata
            .insert("skillCount".to_owned(), skill_ids.len().to_string());
        node.metadata.insert(
            "nativeRoute".to_owned(),
            string_field(family, &["nativeRoute"]).unwrap_or_else(|| "CP09".to_owned()),
        );
        self.add_node(node)?;
        self.add_edge(GraphEdge {
            from: self.manifest.plan.id.clone(),
            to: family_node.clone(),
            kind: EdgeKind::Contains,
        });
        for skill_id in &skill_ids {
            self.classify_intent_skill(&family_node, skill_id, assigned)?;
        }
        let native_route =
            string_field(family, &["nativeRoute"]).unwrap_or_else(|| "CP09".to_owned());
        let native_limit = usize_field(family, &["nativeBatchLimit"]).unwrap_or(5);
        let native_dependencies = string_array(family, &["dependencies"]);
        self.derive_intent_packets(
            &family_node,
            &family_id,
            matrix_path,
            &skill_ids,
            &native_route,
            native_limit,
            &native_dependencies,
            "native-predicate",
            owned_components,
        )?;
        self.derive_intent_packets(
            &family_node,
            &family_id,
            matrix_path,
            &skill_ids,
            "CP11",
            10,
            &["WP/CP08".to_owned()],
            "advisory-manual",
            owned_components,
        )?;
        Ok(())
    }

    fn classify_intent_skill(
        &mut self,
        family_node: &NodeId,
        skill_id: &str,
        assigned: &mut BTreeSet<String>,
    ) -> Result<(), GraphError> {
        let skill_node = NodeId::new(format!("SKILL/{skill_id}"))?;
        let Some(skill) = self.nodes.get(&skill_node) else {
            return Err(GraphError::InvalidValue(format!(
                "intent matrix references unknown skill `{skill_id}`"
            )));
        };
        if skill.metadata.get("sourceAvailability").map(String::as_str) == Some("sourceUnavailable")
            || skill_id == PROTECTED_SKILL
        {
            return Err(GraphError::InvalidValue(
                "protected sourceUnavailable skill appears in intent matrix".to_owned(),
            ));
        }
        if !assigned.insert(skill_id.to_owned()) {
            return Err(GraphError::InvalidValue(format!(
                "intent matrix assigns `{skill_id}` to more than one family"
            )));
        }
        self.add_edge(GraphEdge {
            from: family_node.clone(),
            to: skill_node,
            kind: EdgeKind::Classifies,
        });
        Ok(())
    }

    fn derive_intent_packets(
        &mut self,
        family_node: &NodeId,
        family_id: &str,
        matrix_path: &GraphPath,
        skill_ids: &[String],
        route: &str,
        limit: usize,
        dependencies: &[String],
        owned_kind: &str,
        owned_components: &mut BTreeSet<String>,
    ) -> Result<(), GraphError> {
        if limit == 0 {
            return Err(GraphError::InvalidValue(format!(
                "intent family `{family_id}` has a zero packet limit"
            )));
        }
        let family_key = family_id.strip_prefix("IF/").unwrap_or(family_id);
        for (offset, chunk) in skill_ids.chunks(limit).enumerate() {
            let packet_id = NodeId::new(format!("WP/{route}/IF-{family_key}/B{:02}", offset + 1))?;
            let gate_id = NodeId::new(format!("TEST/{packet_id}/gate"))?;
            let mut packet = GraphNode::new(
                packet_id.clone(),
                NodeKind::Workpack,
                format!("{family_id} {owned_kind} packet {}", offset + 1),
                Some(matrix_path.clone()),
                CompletionContract {
                    required_paths: vec![matrix_path.clone()],
                    required_tests: vec![gate_id],
                    ..CompletionContract::default()
                },
            );
            packet
                .metadata
                .insert("workpackClass".to_owned(), "intent-packet".to_owned());
            packet
                .metadata
                .insert("familyId".to_owned(), family_id.to_owned());
            packet.metadata.insert("route".to_owned(), route.to_owned());
            packet
                .metadata
                .insert("ownedKind".to_owned(), owned_kind.to_owned());
            packet
                .metadata
                .insert("batchLimit".to_owned(), limit.to_string());
            packet
                .metadata
                .insert("skillCount".to_owned(), chunk.len().to_string());
            self.add_node(packet)?;
            self.add_edge(GraphEdge {
                from: family_node.clone(),
                to: packet_id.clone(),
                kind: EdgeKind::RoutesTo,
            });
            for dependency in dependencies {
                self.add_edge(GraphEdge {
                    from: packet_id.clone(),
                    to: NodeId::new(dependency.clone())?,
                    kind: EdgeKind::DependsOn,
                });
            }
            for skill_id in chunk {
                let component_kinds = if owned_kind == "advisory-manual" {
                    vec!["advisory", "manual"]
                } else {
                    vec![owned_kind]
                };
                for component_kind in component_kinds {
                    let key = format!("{skill_id}:{component_kind}");
                    if !owned_components.insert(key) {
                        return Err(GraphError::InvalidValue(format!(
                            "intent packet component ownership overlaps for `{skill_id}:{component_kind}`"
                        )));
                    }
                }
                self.add_edge(GraphEdge {
                    from: packet_id.clone(),
                    to: NodeId::new(format!("SKILL/{skill_id}"))?,
                    kind: EdgeKind::RoutesTo,
                });
            }
        }
        Ok(())
    }

    fn validate_intent_partition(&self, assigned: &BTreeSet<String>) -> Result<(), GraphError> {
        let available: BTreeSet<String> = self
            .nodes
            .values()
            .filter(|node| {
                node.kind == NodeKind::Skill
                    && node.metadata.get("sourceAvailability").map(String::as_str)
                        != Some("sourceUnavailable")
            })
            .filter_map(|node| node.id.as_str().strip_prefix("SKILL/").map(str::to_owned))
            .collect();
        if assigned != &available {
            return Err(GraphError::InvalidValue(format!(
                "intent matrix partition mismatch: assigned {}, available {}",
                assigned.len(),
                available.len()
            )));
        }
        Ok(())
    }

    fn read_proof_rows(&self) -> Result<BTreeMap<String, ProofRow>, GraphError> {
        let path = self
            .root
            .join(self.manifest.test_proof_expectations.as_str());
        let text = fs::read_to_string(path)?;
        Ok(text
            .lines()
            .filter_map(parse_proof_row)
            .map(|row| (row.workpack.clone(), row))
            .collect())
    }

    fn import_cp01_proofs(&mut self) -> Result<(), GraphError> {
        let Some(proof_root) = self.manifest.proof_roots.first() else {
            return Ok(());
        };
        let root = self.root.join(proof_root.as_str()).join("cp01");
        if !root.exists() {
            return Ok(());
        }
        let mut paths: Vec<PathBuf> = fs::read_dir(root)?
            .filter_map(Result::ok)
            .map(|entry| entry.path().join("reconciliation.json"))
            .filter(|path| path.is_file())
            .collect();
        paths.sort();
        for path in &paths {
            self.import_one_cp01_proof(path)?;
        }
        self.validate_cp01_partition(&paths)
    }

    fn import_one_cp01_proof(&mut self, path: &Path) -> Result<(), GraphError> {
        let evidence: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
        let batch = string_field(&evidence, &["batch"]).unwrap_or_else(|| "unknown".to_owned());
        let id = NodeId::new(format!("PROOF/CP01/{batch}"))?;
        let relative = relative_path(&self.root, path)?;
        let mut node = GraphNode::new(
            id.clone(),
            NodeKind::Proof,
            format!("CP01 reconciliation evidence {batch}"),
            Some(relative),
            CompletionContract::default(),
        );
        if let Some(count) = array_field(&evidence, &["rules"]).map(Vec::len) {
            node.metadata
                .insert("ruleCount".to_owned(), count.to_string());
        }
        self.add_node(node)?;
        self.add_edge(GraphEdge {
            from: NodeId::new("WP/CP01")?,
            to: id,
            kind: EdgeKind::Produces,
        });
        Ok(())
    }

    fn validate_cp01_partition(&mut self, paths: &[PathBuf]) -> Result<(), GraphError> {
        let registry: Value = serde_json::from_str(&fs::read_to_string(
            self.root.join(CYBERSKILLS_REGISTRY_PATH),
        )?)?;
        let expected: BTreeSet<String> = array_field(&registry, &[])
            .into_iter()
            .flat_map(|rules| rules.iter())
            .filter_map(|rule| string_field(rule, &["ruleId"]))
            .collect();
        let mut seen = BTreeSet::new();
        for path in paths {
            let evidence: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
            for rule in array_field(&evidence, &["rules"])
                .into_iter()
                .flat_map(|rules| rules.iter())
            {
                if let Some(rule_id) = string_field(rule, &["ruleId"]) {
                    if !seen.insert(rule_id.clone()) {
                        self.issues.push(partition_issue(
                            "CP01-DUPLICATE-RULE",
                            format!("CP01 evidence repeats registry rule `{rule_id}`"),
                        ));
                    }
                }
            }
        }
        let missing: Vec<&String> = expected.difference(&seen).collect();
        let extra: Vec<&String> = seen.difference(&expected).collect();
        if !missing.is_empty() || !extra.is_empty() || expected.len() != seen.len() {
            self.issues.push(partition_issue(
                "CP01-REGISTRY-PARTITION",
                format!(
                    "CP01 registry partition mismatch: expected {}, covered {}, missing {:?}, extra {:?}",
                    expected.len(),
                    seen.len(),
                    missing,
                    extra
                ),
            ));
        }
        Ok(())
    }

    fn import_cp11_proofs(&mut self) -> Result<(), GraphError> {
        let Some(proof_root) = self.manifest.proof_roots.first() else {
            return Ok(());
        };
        let root = self.root.join(proof_root.as_str()).join("cp11");
        if !root.exists() {
            return Ok(());
        }
        let mut paths: Vec<PathBuf> = fs::read_dir(root)?
            .filter_map(Result::ok)
            .map(|entry| entry.path().join("retention.json"))
            .filter(|path| path.is_file())
            .collect();
        paths.sort();
        for path in &paths {
            self.import_one_cp11_proof(path)?;
            self.validate_cp11_packet(path)?;
        }
        Ok(())
    }

    fn import_one_cp11_proof(&mut self, path: &Path) -> Result<(), GraphError> {
        let evidence: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
        let batch = string_field(&evidence, &["batch"]).unwrap_or_else(|| "unknown".to_owned());
        let id = NodeId::new(format!("PROOF/CP11/{batch}"))?;
        let relative = relative_path(&self.root, path)?;
        let mut node = GraphNode::new(
            id.clone(),
            NodeKind::Proof,
            format!("CP11 retention evidence {batch}"),
            Some(relative),
            CompletionContract::default(),
        );
        if let Some(count) = array_field(&evidence, &["skills"]).map(Vec::len) {
            node.metadata
                .insert("skillCount".to_owned(), count.to_string());
        }
        self.add_node(node)?;
        self.add_edge(GraphEdge {
            from: NodeId::new("WP/CP11")?,
            to: id,
            kind: EdgeKind::Produces,
        });
        Ok(())
    }

    fn validate_cp11_packet(&mut self, path: &Path) -> Result<(), GraphError> {
        let evidence: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
        let Some(skills) = array_field(&evidence, &["skills"]) else {
            self.issues.push(packet_issue("CP11-SKILLS-MISSING", path));
            return Ok(());
        };
        let mut ids = BTreeSet::new();
        for skill in skills {
            let valid = valid_cp11_skill(&self.root, skill, &mut ids);
            if !valid {
                self.issues.push(packet_issue("CP11-SKILL-BOUNDARY", path));
            }
        }
        if ids.len() != skills.len() || skills.is_empty() || skills.len() > 10 {
            self.issues.push(packet_issue("CP11-PACKET-SIZE", path));
        }
        Ok(())
    }

    fn import_cp08_proofs(&mut self) -> Result<(), GraphError> {
        let Some(proof_root) = self.manifest.proof_roots.first() else {
            return Ok(());
        };
        let root = self.root.join(proof_root.as_str()).join("cp08");
        if !root.exists() {
            return Ok(());
        }
        let mut paths: Vec<PathBuf> = fs::read_dir(root)?
            .filter_map(Result::ok)
            .map(|entry| entry.path().join("decomposition.json"))
            .filter(|path| path.is_file())
            .collect();
        paths.sort();
        for path in &paths {
            self.import_one_cp08_proof(path)?;
        }
        self.validate_cp08_retention(&paths)?;
        Ok(())
    }

    fn validate_cp08_retention(&mut self, paths: &[PathBuf]) -> Result<(), GraphError> {
        let mut catalog_ids = BTreeSet::new();
        for path in paths {
            let evidence: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
            let Some(skills) = evidence.get("skills").and_then(Value::as_array) else {
                self.issues.push(GraphIssue {
                    level: IssueLevel::Error,
                    code: "CP11-SKILLS-MISSING".to_owned(),
                    node: None,
                    message: format!("CP08 artifact `{}` has no skills array", path.display()),
                });
                continue;
            };
            for skill in skills {
                let Some(catalog_id) = string_field(skill, &["catalogId"]) else {
                    self.issues.push(GraphIssue {
                        level: IssueLevel::Error,
                        code: "CP11-CATALOG-ID-MISSING".to_owned(),
                        node: None,
                        message: format!(
                            "CP08 artifact `{}` contains a skill without catalogId",
                            path.display()
                        ),
                    });
                    continue;
                };
                let node = NodeId::new(format!("SKILL/{catalog_id}"))?;
                if !catalog_ids.insert(catalog_id.clone()) {
                    self.issues.push(GraphIssue {
                        level: IssueLevel::Error,
                        code: "CP11-DUPLICATE-SKILL".to_owned(),
                        node: Some(node.clone()),
                        message: format!("retention evidence repeats catalog ID `{catalog_id}`"),
                    });
                }
                let source_valid = string_field(skill, &["source", "path"])
                    .is_some_and(|value| !value.trim().is_empty())
                    && string_field(skill, &["source", "sha256"])
                        .is_some_and(|value| value.len() == 64)
                    && string_field(skill, &["source", "license"])
                        .is_some_and(|value| value == "Apache-2.0")
                    && !string_array(skill, &["source", "anchors"]).is_empty();
                if !source_valid {
                    self.issues.push(GraphIssue {
                        level: IssueLevel::Error,
                        code: "CP11-SOURCE-EVIDENCE-MISSING".to_owned(),
                        node: Some(node.clone()),
                        message: format!(
                            "retention evidence for `{catalog_id}` lacks source path/hash/license/anchors"
                        ),
                    });
                }
                let components = skill
                    .get("components")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for kind in ["advisory", "manual"] {
                    let retained = components.iter().filter(|component| {
                        string_field(component, &["kind"]).as_deref() == Some(kind)
                            && string_field(component, &["status"]).as_deref() == Some("retained")
                            && string_field(component, &["predicateOrPurpose"])
                                .is_some_and(|value| !value.trim().is_empty())
                            && component
                                .get("notProved")
                                .and_then(Value::as_array)
                                .is_some_and(|values| {
                                    !values.is_empty()
                                        && values.iter().all(|value| {
                                            value
                                                .as_str()
                                                .is_some_and(|text| !text.trim().is_empty())
                                        })
                                })
                    });
                    if retained.count() != 1 {
                        self.issues.push(GraphIssue {
                            level: IssueLevel::Error,
                            code: "CP11-RETENTION-KIND".to_owned(),
                            node: Some(node.clone()),
                            message: format!(
                                "`{catalog_id}` must have exactly one retained {kind} component with purpose and notProved"
                            ),
                        });
                    }
                }
            }
        }
        if catalog_ids.len() != 816 {
            self.issues.push(GraphIssue {
                level: IssueLevel::Error,
                code: "CP11-RETENTION-COUNT".to_owned(),
                node: None,
                message: format!(
                    "retention evidence covers {} catalog IDs; expected 816",
                    catalog_ids.len()
                ),
            });
        }
        Ok(())
    }

    fn import_one_cp08_proof(&mut self, path: &Path) -> Result<(), GraphError> {
        let evidence: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
        let batch = string_field(&evidence, &["batch"]).unwrap_or_else(|| "unknown".to_owned());
        let id = NodeId::new(format!("PROOF/CP08/{batch}"))?;
        let relative = relative_path(&self.root, path)?;
        let mut node = GraphNode::new(
            id.clone(),
            NodeKind::Proof,
            format!("CP08 decomposition evidence {batch}"),
            Some(relative),
            CompletionContract::default(),
        );
        if let Some(count) = array_field(&evidence, &["selection", "catalogIds"]).map(Vec::len) {
            node.metadata
                .insert("catalogCount".to_owned(), count.to_string());
        }
        self.add_node(node)?;
        let workpack = NodeId::new("WP/CP08")?;
        self.add_edge(GraphEdge {
            from: workpack,
            to: id.clone(),
            kind: EdgeKind::Produces,
        });
        for catalog_id in string_array(&evidence, &["selection", "catalogIds"]) {
            if let Ok(skill) = NodeId::new(format!("SKILL/{catalog_id}")) {
                self.add_edge(GraphEdge {
                    from: id.clone(),
                    to: skill,
                    kind: EdgeKind::EvidenceFor,
                });
            }
        }
        Ok(())
    }

    fn import_catalog(&mut self) -> Result<(), GraphError> {
        let path = self.root.join(self.manifest.catalog_path.as_str());
        let catalog: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
        let records = catalog
            .get("records")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                GraphError::InvalidValue("catalog records array is missing".to_owned())
            })?;
        for record in records {
            self.import_one_catalog_record(record)?;
        }
        Ok(())
    }

    fn import_one_catalog_record(&mut self, record: &Value) -> Result<(), GraphError> {
        let Some(catalog_id) = string_field(record, &["catalogId"]) else {
            self.issues.push(GraphIssue {
                level: IssueLevel::Error,
                code: "CATALOG-ID-MISSING".to_owned(),
                node: None,
                message: "catalog record has no catalogId".to_owned(),
            });
            return Ok(());
        };
        let id = NodeId::new(format!("SKILL/{catalog_id}"))?;
        let availability =
            string_field(record, &["sourceAvailability"]).unwrap_or_else(|| "unknown".to_owned());
        let decomposition = string_field(record, &["cp08Projection", "status"])
            .or_else(|| string_field(record, &["decompositionState"]))
            .unwrap_or_else(|| "unreviewed".to_owned());
        let implementation = coverage_field(record, &["implementation", "native", "coverage"]);
        let proof = coverage_field(record, &["implementation", "executableProof", "coverage"]);
        let mut node = GraphNode::new(
            id.clone(),
            NodeKind::Skill,
            catalog_id.clone(),
            None,
            CompletionContract::default(),
        );
        node.metadata
            .insert("sourceAvailability".to_owned(), availability.clone());
        node.metadata
            .insert("decomposition".to_owned(), decomposition);
        node.metadata.insert(
            "implementationCoverage".to_owned(),
            coverage_name(implementation),
        );
        node.metadata
            .insert("proofCoverage".to_owned(), coverage_name(proof));
        if availability == "sourceUnavailable" || catalog_id == PROTECTED_SKILL {
            node.metadata
                .insert("protectedBoundary".to_owned(), "excluded".to_owned());
        } else if let Some(source_path) = string_field(record, &["sourcePath"]) {
            node.metadata.insert("sourcePath".to_owned(), source_path);
        }
        self.add_node(node)?;
        self.add_edge(GraphEdge {
            from: self.manifest.plan.id.clone(),
            to: id,
            kind: EdgeKind::Contains,
        });
        Ok(())
    }

    fn apply_overrides(&mut self) -> Result<(), GraphError> {
        for (id, state) in &self.manifest.overrides.lifecycle {
            let node = self
                .nodes
                .get_mut(id)
                .ok_or_else(|| GraphError::MissingNode(id.to_string()))?;
            node.lifecycle = *state;
        }
        let dependencies = self.manifest.overrides.dependencies.clone();
        for (from, dependencies) in dependencies {
            self.edges
                .retain(|edge| !(edge.from == from && edge.kind == EdgeKind::DependsOn));
            for target in dependencies {
                self.add_edge(GraphEdge {
                    from: from.clone(),
                    to: target,
                    kind: EdgeKind::DependsOn,
                });
            }
        }
        Ok(())
    }

    fn node_status(&self, node: &GraphNode) -> NodeStatus {
        let mut reasons = Vec::new();
        let state = self.state_for(&node.id, &mut BTreeSet::new(), &mut reasons);
        NodeStatus {
            id: node.id.clone(),
            kind: node.kind,
            title: node.title.clone(),
            state,
            path: node.path.clone(),
            reasons,
        }
    }

    fn state_for(
        &self,
        id: &NodeId,
        visiting: &mut BTreeSet<NodeId>,
        reasons: &mut Vec<String>,
    ) -> DerivedState {
        let Some(node) = self.nodes.get(id) else {
            reasons.push(format!("missing node `{id}`"));
            return DerivedState::Blocked;
        };
        if !visiting.insert(id.clone()) {
            reasons.push(format!("dependency cycle reaches `{id}`"));
            return DerivedState::Blocked;
        }
        let state = match node.lifecycle {
            LifecycleState::Active => DerivedState::Active,
            LifecycleState::Validation => DerivedState::Validation,
            LifecycleState::Failed => DerivedState::Failed,
            LifecycleState::Paused => DerivedState::Paused,
            LifecycleState::Done => {
                let contract = self.contract_result(node);
                if contract.is_complete() {
                    DerivedState::Done
                } else {
                    reasons.extend(contract.missing);
                    DerivedState::Blocked
                }
            }
            LifecycleState::Planned => {
                let mut blocked = false;
                for dependency in self.dependencies(id) {
                    let dependency_state = self.state_for(&dependency, visiting, reasons);
                    if dependency_state != DerivedState::Done {
                        blocked = true;
                        reasons.push(format!("dependency `{dependency}` is {dependency_state:?}"));
                    }
                }
                if blocked {
                    DerivedState::Blocked
                } else if node.kind == NodeKind::Workpack {
                    DerivedState::Ready
                } else {
                    DerivedState::Planned
                }
            }
        };
        visiting.remove(id);
        state
    }

    fn dependencies(&self, id: &NodeId) -> Vec<NodeId> {
        self.edges
            .iter()
            .filter(|edge| edge.from == *id && edge.kind == EdgeKind::DependsOn)
            .map(|edge| edge.to.clone())
            .collect()
    }

    fn contract_result(&self, node: &GraphNode) -> ContractResult {
        let mut missing = Vec::new();
        for path in &node.completion.required_paths {
            if !self.root.join(path.as_str()).is_file() {
                missing.push(format!("required path `{path}` is absent"));
            }
        }
        for id in node
            .completion
            .required_tests
            .iter()
            .chain(node.completion.required_proofs.iter())
            .chain(node.completion.required_adrs.iter())
        {
            missing.extend(self.evidence_requirements(id));
        }
        if node.completion.checklist_complete < node.completion.checklist_total {
            missing.push(format!(
                "checklist is {}/{} complete",
                node.completion.checklist_complete, node.completion.checklist_total
            ));
        }
        ContractResult { missing }
    }

    fn evidence_requirements(&self, id: &NodeId) -> Vec<String> {
        if self
            .nodes
            .get(id)
            .and_then(|evidence| evidence.path.as_ref())
            .is_some_and(|path| self.root.join(path.as_str()).is_file())
        {
            return Vec::new();
        }
        let Some(record) = self.manifest.overrides.evidence.get(id) else {
            return vec![format!(
                "required evidence `{id}` has no readable artifact or recorded gate"
            )];
        };
        record
            .source_paths
            .iter()
            .filter(|source_path| !self.root.join(source_path.as_str()).is_file())
            .map(|source_path| format!("recorded evidence `{id}` source `{source_path}` is absent"))
            .collect()
    }

    fn endpoint_issues(&self) -> Vec<GraphIssue> {
        self.edges
            .iter()
            .flat_map(|edge| {
                let mut findings = Vec::new();
                if !self.nodes.contains_key(&edge.from) {
                    findings.push(missing_endpoint(edge, &edge.from));
                }
                if !self.nodes.contains_key(&edge.to) {
                    findings.push(missing_endpoint(edge, &edge.to));
                }
                findings
            })
            .collect()
    }

    fn cycle_issues(&self) -> Vec<GraphIssue> {
        let mut findings = Vec::new();
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        for node in self.nodes.keys() {
            if let Some(cycle) = self.find_cycle(node, &mut visiting, &mut visited) {
                findings.push(GraphIssue {
                    level: IssueLevel::Error,
                    code: "GRAPH-CYCLE".to_owned(),
                    node: Some(node.clone()),
                    message: format!(
                        "dependency cycle: {}",
                        cycle
                            .iter()
                            .map(NodeId::as_str)
                            .collect::<Vec<_>>()
                            .join(" -> ")
                    ),
                });
            }
        }
        findings
    }

    fn find_cycle(
        &self,
        node: &NodeId,
        visiting: &mut BTreeSet<NodeId>,
        visited: &mut BTreeSet<NodeId>,
    ) -> Option<Vec<NodeId>> {
        if visited.contains(node) {
            return None;
        }
        if !visiting.insert(node.clone()) {
            return Some(vec![node.clone()]);
        }
        for dependency in self.dependencies(node) {
            if let Some(mut cycle) = self.find_cycle(&dependency, visiting, visited) {
                cycle.insert(0, node.clone());
                return Some(cycle);
            }
        }
        visiting.remove(node);
        visited.insert(node.clone());
        None
    }

    fn done_contract_issues(&self) -> Vec<GraphIssue> {
        self.nodes
            .values()
            .filter(|node| node.lifecycle == LifecycleState::Done)
            .filter_map(|node| {
                let contract = self.contract_result(node);
                if contract.is_complete() {
                    None
                } else {
                    Some(GraphIssue {
                        level: IssueLevel::Error,
                        code: "DONE-CONTRACT".to_owned(),
                        node: Some(node.id.clone()),
                        message: contract.missing.join("; "),
                    })
                }
            })
            .collect()
    }

    fn protected_issues(&self) -> Vec<GraphIssue> {
        self.nodes
            .values()
            .filter(|node| {
                node.metadata.get("sourceAvailability").map(String::as_str)
                    == Some("sourceUnavailable")
            })
            .filter_map(|node| {
                if node.path.is_some() {
                    Some(GraphIssue {
                        level: IssueLevel::Error,
                        code: "PROTECTED-SOURCE-READ".to_owned(),
                        node: Some(node.id.clone()),
                        message: "sourceUnavailable node must not carry a materialized vendor path"
                            .to_owned(),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn catalog_summary(&self) -> CatalogSummary {
        let mut summary = CatalogSummary::default();
        for node in self
            .nodes
            .values()
            .filter(|node| node.kind == NodeKind::Skill)
        {
            summary.total += 1;
            if node.metadata.get("sourceAvailability").map(String::as_str)
                == Some("sourceUnavailable")
            {
                summary.source_unavailable += 1;
            } else {
                summary.available += 1;
            }
            match node.metadata.get("decomposition").map(String::as_str) {
                Some("complete") => summary.decomposed_complete += 1,
                Some("partial") => summary.decomposed_partial += 1,
                _ => {}
            }
            count_coverage(
                node.metadata.get("implementationCoverage"),
                &mut summary.native_complete,
                &mut summary.native_partial,
            );
            count_coverage(
                node.metadata.get("proofCoverage"),
                &mut summary.proof_complete,
                &mut summary.proof_partial,
            );
        }
        summary
    }

    fn intent_summary(&self) -> IntentSummary {
        let mut summary = IntentSummary {
            protected_excluded: self
                .nodes
                .values()
                .filter(|node| {
                    node.kind == NodeKind::Skill
                        && node.metadata.get("protectedBoundary").map(String::as_str)
                            == Some("excluded")
                })
                .count(),
            ..IntentSummary::default()
        };
        for node in self.nodes.values() {
            match node.kind {
                NodeKind::IntentFamily => summary.family_count += 1,
                NodeKind::Workpack
                    if node.metadata.get("workpackClass").map(String::as_str)
                        == Some("intent-packet") =>
                {
                    summary.packet_count += 1;
                    match node.metadata.get("route").map(String::as_str) {
                        Some("CP11") => summary.retention_packet_count += 1,
                        Some("CP09" | "CP12") => summary.native_packet_count += 1,
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        summary.mapped_skill_count = self
            .nodes
            .values()
            .filter(|node| node.kind == NodeKind::Skill)
            .filter(|node| {
                self.edges
                    .iter()
                    .any(|edge| edge.kind == EdgeKind::Classifies && edge.to == node.id)
            })
            .count();
        summary
    }

    fn explain(
        &self,
        id: &NodeId,
        chain: &mut Vec<NodeId>,
        blockers: &mut Vec<String>,
        visiting: &mut BTreeSet<NodeId>,
    ) {
        if !visiting.insert(id.clone()) {
            blockers.push(format!("cycle at `{id}`"));
            return;
        }
        chain.push(id.clone());
        let status = self.inspect(id).ok();
        if let Some(status) = status {
            blockers.extend(status.reasons);
        }
        for dependency in self.dependencies(id) {
            if self
                .nodes
                .get(&dependency)
                .map(|node| node.lifecycle != LifecycleState::Done)
                .unwrap_or(true)
            {
                self.explain(&dependency, chain, blockers, visiting);
            }
        }
        visiting.remove(id);
    }
}

#[derive(Debug)]
struct ContractResult {
    missing: Vec<String>,
}

impl ContractResult {
    fn is_complete(&self) -> bool {
        self.missing.is_empty()
    }
}

#[derive(Debug, Clone)]
struct ProofRow {
    workpack: String,
    proof: String,
    gates: String,
    state: String,
}

#[derive(Debug)]
struct IndexRow {
    status: String,
    owner: String,
    batch_limit: String,
    owns: String,
}

fn relative_path(root: &Path, path: &Path) -> Result<GraphPath, GraphError> {
    let relative = path.strip_prefix(root).map_err(|_| {
        GraphError::InvalidValue("evidence path escaped repository root".to_owned())
    })?;
    GraphPath::new(relative.to_string_lossy().into_owned())
}

fn first_heading(text: &str) -> Option<String> {
    text.lines()
        .find(|line| line.starts_with("# "))
        .map(|line| line.trim_start_matches("# ").trim().to_owned())
}

fn workpack_key(title: &str, stem: &str) -> String {
    title
        .split_whitespace()
        .next()
        .filter(|value| {
            let value = value.to_ascii_uppercase();
            value.starts_with("CP") || value.starts_with("UL")
        })
        .unwrap_or(stem)
        .to_ascii_uppercase()
}

fn backtick_values(text: &str) -> Vec<String> {
    text.split('`')
        .enumerate()
        .filter_map(|(index, value)| (index % 2 == 1).then_some(value.trim().to_owned()))
        .filter(|value| !value.is_empty())
        .collect()
}

fn dependency_tokens(text: &str) -> Vec<String> {
    let Some(line) = text
        .lines()
        .find(|line| line.trim_start().starts_with("- deps:"))
    else {
        return Vec::new();
    };
    let values = backtick_values(line);
    if values
        .iter()
        .any(|value| value.eq_ignore_ascii_case("none"))
    {
        return Vec::new();
    }
    values
        .into_iter()
        .flat_map(|value| {
            value
                .split(',')
                .map(str::trim)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|value| !value.is_empty())
        .collect()
}

fn dependency_target(
    raw: &str,
    workpack_ids: &BTreeMap<String, NodeId>,
) -> Result<NodeId, GraphError> {
    let key = raw.trim().to_ascii_uppercase();
    if let Some(target) = workpack_ids.get(&key) {
        return Ok(target.clone());
    }
    if key.starts_with("UL") {
        return NodeId::new(format!("EXT/{key}"));
    }
    NodeId::new(format!("MISSING/{key}"))
}

fn external_dependency(id: &NodeId, raw: &str) -> GraphNode {
    let mut node = GraphNode::new(
        id.clone(),
        NodeKind::Dependency,
        format!("External dependency {raw}"),
        None,
        CompletionContract::default(),
    );
    node.metadata
        .insert("authority".to_owned(), "external-plan".to_owned());
    node
}

fn checklist_counts(text: &str) -> (usize, usize) {
    text.lines()
        .filter(|line| line.trim_start().starts_with("- [") || line.trim_start().starts_with("* ["))
        .fold((0, 0), |(total, complete), line| {
            (
                total + 1,
                complete + usize::from(line.contains("[x]") || line.contains("[X]")),
            )
        })
}

fn checklist_nodes(id: &NodeId, text: &str) -> Result<Vec<GraphNode>, GraphError> {
    text.lines()
        .filter(|line| line.trim_start().starts_with("- [") || line.trim_start().starts_with("* ["))
        .enumerate()
        .map(|(index, line)| {
            let content = line
                .split_once(']')
                .map(|(_, value)| value.trim())
                .unwrap_or(line.trim());
            let slug = stable_slug(content);
            let requirement_id = NodeId::new(format!("{id}/REQ/{slug}-{index}"))?;
            let mut node = GraphNode::new(
                requirement_id,
                NodeKind::Requirement,
                content,
                None,
                CompletionContract::default(),
            );
            node.metadata.insert(
                "checked".to_owned(),
                (line.contains("[x]") || line.contains("[X]")).to_string(),
            );
            Ok(node)
        })
        .collect()
}

fn stable_slug(text: &str) -> String {
    let mut slug = String::new();
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "item".to_owned()
    } else {
        trimmed.chars().take(72).collect()
    }
}

fn parse_index_row(index: &str, stem: &str) -> Option<IndexRow> {
    index.lines().find_map(|line| {
        if !line.trim_start().starts_with('|') {
            return None;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() < 8 || !cells.get(2)?.eq_ignore_ascii_case(stem) {
            return None;
        }
        Some(IndexRow {
            status: cells.get(1)?.to_string(),
            owner: cells.get(4)?.to_string(),
            batch_limit: cells.get(6)?.to_string(),
            owns: cells.get(7)?.to_string(),
        })
    })
}

fn parse_proof_row(line: &str) -> Option<ProofRow> {
    if !line.trim_start().starts_with('|') || line.contains("---") {
        return None;
    }
    let cells: Vec<&str> = line.split('|').map(str::trim).collect();
    if cells.len() < 5 || !cells.get(1)?.to_ascii_uppercase().starts_with("CP") {
        return None;
    }
    Some(ProofRow {
        workpack: cells.get(1)?.to_ascii_uppercase(),
        proof: cells.get(2)?.to_string(),
        gates: cells.get(3)?.to_string(),
        state: cells.get(4)?.to_string(),
    })
}

fn completion_contract(row: Option<&ProofRow>) -> CompletionContract {
    let Some(row) = row else {
        return CompletionContract::default();
    };
    let required_proofs: Vec<NodeId> = backtick_values(&row.proof)
        .into_iter()
        .filter(|path| path.contains('/') || path.ends_with(".json"))
        .filter_map(|path| NodeId::new(format!("PROOF/PATH/{}", path.replace('/', "_"))).ok())
        .collect();
    let required_tests: Vec<NodeId> = row
        .gates
        .split(';')
        .map(str::trim)
        .filter(|gate| !gate.is_empty())
        .filter_map(|gate| NodeId::new(format!("TEST/{}/{}", row.workpack, stable_slug(gate))).ok())
        .collect();
    CompletionContract {
        required_paths: Vec::new(),
        required_tests,
        required_proofs,
        required_adrs: Vec::new(),
        checklist_total: 0,
        checklist_complete: 0,
    }
}

fn string_field(value: &Value, path: &[&str]) -> Option<String> {
    path.iter()
        .try_fold(value, |current, segment| current.get(*segment))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn usize_field(value: &Value, path: &[&str]) -> Option<usize> {
    path.iter()
        .try_fold(value, |current, segment| current.get(*segment))
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn required_string_array(
    value: &Value,
    path: &[&str],
    owner: &str,
) -> Result<Vec<String>, GraphError> {
    let array = array_field(value, path).ok_or_else(|| {
        GraphError::InvalidValue(format!("intent family `{owner}` has no skillIds array"))
    })?;
    let values: Vec<String> = array
        .iter()
        .map(|value| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                GraphError::InvalidValue(format!(
                    "intent family `{owner}` contains a non-string skill ID"
                ))
            })
        })
        .collect::<Result<_, _>>()?;
    if values.is_empty() {
        return Err(GraphError::InvalidValue(format!(
            "intent family `{owner}` has no skills"
        )));
    }
    Ok(values)
}

fn validate_intent_matrix_header(matrix: &Value) -> Result<(), GraphError> {
    let schema = usize_field(matrix, &["schemaVersion"]);
    let skills = usize_field(matrix, &["skillCount"]);
    let families = usize_field(matrix, &["familyCount"]);
    if schema != Some(1) || skills != Some(816) || families != Some(34) {
        return Err(GraphError::InvalidValue(
            "intent matrix header must declare schema 1, 34 families, and 816 skills".to_owned(),
        ));
    }
    let protected = string_array(matrix, &["generatedFrom", "protectedExcluded"]);
    if protected != [PROTECTED_SKILL.to_owned()] {
        return Err(GraphError::InvalidValue(
            "intent matrix protected exclusion is not exact".to_owned(),
        ));
    }
    Ok(())
}

fn coverage_field(value: &Value, path: &[&str]) -> CoverageLevel {
    match string_field(value, path).as_deref() {
        Some("complete") => CoverageLevel::Complete,
        Some("partial") => CoverageLevel::Partial,
        _ => CoverageLevel::None,
    }
}

fn coverage_name(level: CoverageLevel) -> String {
    match level {
        CoverageLevel::None => "none",
        CoverageLevel::Partial => "partial",
        CoverageLevel::Complete => "complete",
    }
    .to_owned()
}

fn array_field<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Vec<Value>> {
    path.iter()
        .try_fold(value, |current, segment| current.get(*segment))
        .and_then(Value::as_array)
}

fn string_array(value: &Value, path: &[&str]) -> Vec<String> {
    array_field(value, path)
        .map(|array| {
            array
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn missing_endpoint(edge: &GraphEdge, endpoint: &NodeId) -> GraphIssue {
    GraphIssue {
        level: IssueLevel::Error,
        code: "GRAPH-MISSING-NODE".to_owned(),
        node: Some(edge.from.clone()),
        message: format!("edge {:?} references missing node `{endpoint}`", edge.kind),
    }
}

fn partition_issue(code: &str, message: String) -> GraphIssue {
    GraphIssue {
        level: IssueLevel::Error,
        code: code.to_owned(),
        node: None,
        message,
    }
}

fn packet_issue(code: &str, path: &Path) -> GraphIssue {
    GraphIssue {
        level: IssueLevel::Error,
        code: code.to_owned(),
        node: None,
        message: format!(
            "CP11 retention packet `{}` violates its boundary",
            path.display()
        ),
    }
}

fn valid_cp11_skill(root: &Path, skill: &Value, ids: &mut BTreeSet<String>) -> bool {
    let Some(catalog_id) = string_field(skill, &["catalogId"]) else {
        return false;
    };
    let source_path = string_field(skill, &["source", "path"]);
    let source_hash = string_field(skill, &["source", "sha256"]);
    let artifact_path = string_field(skill, &["cp08Evidence", "artifactPath"]);
    let artifact_hash = string_field(skill, &["cp08Evidence", "artifactSha256"]);
    let source_ok = source_path.is_some_and(|path| !path.trim().is_empty())
        && source_hash.is_some_and(|hash| valid_sha256(hash.as_str()))
        && string_field(skill, &["source", "license"]).as_deref() == Some("Apache-2.0")
        && array_field(skill, &["source", "anchors"]).is_some_and(|anchors| !anchors.is_empty());
    let artifact_ok = artifact_path
        .is_some_and(|path| is_safe_relative_path(path.as_str()) && root.join(path).is_file())
        && artifact_hash.is_some_and(|hash| valid_sha256(hash.as_str()));
    ids.insert(catalog_id.clone())
        && catalog_id != PROTECTED_SKILL
        && source_ok
        && artifact_ok
        && valid_retention_component(skill.get("advisory"))
        && valid_retention_component(skill.get("manual"))
}

fn valid_retention_component(value: Option<&Value>) -> bool {
    value
        .and_then(|component| string_field(component, &["status"]))
        .is_some_and(|status| status == "retained")
        && value
            .and_then(|component| string_field(component, &["purpose"]))
            .is_some_and(|purpose| !purpose.trim().is_empty())
        && value
            .and_then(|component| array_field(component, &["notProved"]))
            .is_some_and(|not_proved| !not_proved.is_empty())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value == value.to_ascii_lowercase()
        && value.chars().all(|character| character.is_ascii_hexdigit())
}

fn count_coverage(value: Option<&String>, complete: &mut usize, partial: &mut usize) {
    match value.map(String::as_str) {
        Some("complete") => *complete += 1,
        Some("partial") => *partial += 1,
        _ => {}
    }
}

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
