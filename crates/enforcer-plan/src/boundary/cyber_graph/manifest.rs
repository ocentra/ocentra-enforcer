//! BOUNDARY-INVARIANT: the graph manifest is a validated repository-relative
//! configuration boundary and never grants authority to external systems.
//! NEGATIVE-TEST: unsupported schema, unsafe paths, and invalid overrides are
//! rejected before graph import begins.
use super::{GraphError, GraphPath, LifecycleState, NodeId, SCHEMA_VERSION, is_safe_relative_path};
use std::collections::{BTreeMap, BTreeSet};

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
    pub(super) fn validate(&self) -> Result<(), GraphError> {
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
        self.overrides
            .evidence
            .iter()
            .try_for_each(|(node, evidence)| {
                evidence.validate(node)?;
                (!run_ids.insert(evidence.run_id.as_str()))
                    .then_some(())
                    .map_or(Ok(()), |_| {
                        Err(GraphError::InvalidValue(format!(
                            "completion evidence run `{}` is reused",
                            evidence.run_id
                        )))
                    })
            })?;
        Ok(())
    }
}
