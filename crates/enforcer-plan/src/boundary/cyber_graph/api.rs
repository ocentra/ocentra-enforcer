//! BOUNDARY-INVARIANT: public graph operations expose only validated graph
//! domain values and read-only JSON views.
//! NEGATIVE-TEST: invalid manifests, missing nodes, and incomplete contracts
//! remain rejected by the graph integration suite.
use super::manifest::GraphManifest;
use super::manifest_wire;
use super::state::is_ready_entry_gate;
use super::{
    BlockedReport, CyberPlanGraph, DerivedState, GraphError, LifecycleState, NodeId, NodeKind,
    NodeStatus, StatusReport, ValidationReport, WhyReport, GRAPH_MANIFEST_PATH,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

struct ImportStep {
    enabled: bool,
    run: fn(&mut CyberPlanGraph) -> Result<(), GraphError>,
}

impl CyberPlanGraph {
    /// Load the Cyber Plan graph from the checked-in manifest and sources.
    pub fn load(root: impl AsRef<Path>) -> Result<Self, GraphError> {
        let root = root.as_ref().to_path_buf();
        let manifest_file = root.join(GRAPH_MANIFEST_PATH);
        let manifest_wire: manifest_wire::GraphManifestWire =
            serde_json::from_str(&fs::read_to_string(manifest_file)?)?;
        let manifest: GraphManifest = manifest_wire.into();
        manifest.validate()?;
        let mut graph = Self::new_for_root(root, manifest);
        graph.import_seeds()?;
        let importers = [
            ImportStep {
                enabled: graph.manifest.import.dependency_workpacks,
                run: Self::import_dependency_workpacks,
            },
            ImportStep {
                enabled: graph.manifest.import.workpacks,
                run: Self::import_workpacks,
            },
            ImportStep {
                enabled: graph.manifest.import.cp01_proofs,
                run: Self::import_cp01_proofs,
            },
            ImportStep {
                enabled: graph.manifest.import.cp08_proofs,
                run: Self::import_cp08_proofs,
            },
            ImportStep {
                enabled: graph.manifest.import.cp11_proofs,
                run: Self::import_cp11_proofs,
            },
            ImportStep {
                enabled: graph.manifest.import.catalog,
                run: Self::import_catalog,
            },
            ImportStep {
                enabled: graph.manifest.import.intent_matrix,
                run: Self::import_intent_matrix,
            },
        ];
        importers
            .into_iter()
            .filter(|step| step.enabled)
            .try_for_each(|step| (step.run)(&mut graph))?;
        graph.apply_overrides()?;
        Ok(graph)
    }

    /// Validate IDs, endpoints, dependencies, cycles, protected coverage,
    /// and DONE contracts without changing graph state.
    pub fn validate(&self) -> ValidationReport {
        let mut issues = self.issues.clone();
        issues.extend(self.endpoint_issues());
        issues.extend(self.cycle_issues());
        issues.extend(self.done_contract_issues());
        issues.extend(self.authority_issues());
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

    /// Return executable workpacks whose dependencies are satisfied.
    ///
    /// A READY entry-routing gate is intentionally omitted: it authorizes
    /// dependent intent packets but is not itself an implementation packet.
    pub fn ready(&self) -> Vec<NodeStatus> {
        self.nodes
            .values()
            .filter(|node| node.kind == NodeKind::Workpack && !is_ready_entry_gate(self, &node.id))
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

    /// Advance a lifecycle through the graph. DONE is only accepted when its
    /// hard dependencies and completion contract are both satisfied.
    pub fn transition(&mut self, id: &NodeId, target: LifecycleState) -> Result<(), GraphError> {
        let node = self
            .nodes
            .get(id)
            .ok_or_else(|| GraphError::MissingNode(id.to_string()))?;
        if target == LifecycleState::Done {
            if let Some(issue) = super::validation::authority_lifecycle_issue(node) {
                return Err(GraphError::InvalidValue(format!(
                    "cannot mark `{id}` done: {}",
                    issue.message
                )));
            }
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
}
