//! BOUNDARY-INVARIANT: lifecycle and completion state are derived from graph
//! contracts and evidence; stored lifecycle never implies proof completion.
//! NEGATIVE-TEST: missing dependencies, paths, tests, proofs, and ADRs block
//! completion instead of being treated as satisfied.
use super::manifest::GraphManifest;
use super::{
    CyberPlanGraph, DerivedState, EdgeKind, GraphEdge, GraphError, GraphNode, LifecycleState,
    NodeId, NodeKind, NodeStatus,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

impl CyberPlanGraph {
    /// Create an empty graph for unit/integration tests.
    pub fn new_for_root(root: impl Into<PathBuf>, manifest: GraphManifest) -> Self {
        Self {
            root: root.into(),
            manifest,
            nodes: BTreeMap::new(),
            edges: BTreeSet::new(),
            issues: Vec::new(),
            cp08_component_kinds: BTreeMap::new(),
        }
    }

    /// Add a node, rejecting duplicate stable IDs.
    pub fn add_node(&mut self, node: GraphNode) -> Result<(), GraphError> {
        self.nodes
            .insert(node.id.clone(), node)
            .map_or(Ok(()), |_| {
                Err(GraphError::InvalidValue(
                    "duplicate graph node id".to_owned(),
                ))
            })
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

    pub(crate) fn apply_overrides(&mut self) -> Result<(), GraphError> {
        self.manifest
            .overrides
            .lifecycle
            .iter()
            .try_for_each(|(id, state)| {
                let node = self
                    .nodes
                    .get_mut(id)
                    .ok_or_else(|| GraphError::MissingNode(id.to_string()))?;
                node.lifecycle = *state;
                Ok::<(), GraphError>(())
            })?;
        let dependencies = self.manifest.overrides.dependencies.clone();
        dependencies
            .into_iter()
            .try_for_each(|(from, dependencies)| {
                apply_dependency_override(self, from, dependencies)
            })
    }

    pub(crate) fn node_status(&self, node: &GraphNode) -> NodeStatus {
        let mut reasons = Vec::new();
        let state = state_for(self, &node.id, &mut BTreeSet::new(), &mut reasons);
        NodeStatus {
            id: node.id.clone(),
            kind: node.kind,
            title: node.title.clone(),
            state,
            path: node.path.clone(),
            reasons,
        }
    }

    pub(super) fn dependencies(&self, id: &NodeId) -> Vec<NodeId> {
        self.edges
            .iter()
            .filter(|edge| edge.from == *id && edge.kind == EdgeKind::DependsOn)
            .map(|edge| edge.to.clone())
            .collect()
    }

    pub(super) fn contract_result(&self, node: &GraphNode) -> ContractResult {
        let mut missing: Vec<String> = node
            .completion
            .required_paths
            .iter()
            .filter(|path| !self.root.join(path.as_str()).is_file())
            .map(|path| format!("required path `{path}` is absent"))
            .collect();
        missing.extend(
            node.completion
                .required_tests
                .iter()
                .chain(node.completion.required_proofs.iter())
                .chain(node.completion.required_adrs.iter())
                .flat_map(|id| evidence_requirements(self, id)),
        );
        if node.completion.checklist_complete < node.completion.checklist_total {
            missing.push(format!(
                "checklist is {}/{} complete",
                node.completion.checklist_complete, node.completion.checklist_total
            ));
        }
        ContractResult { missing }
    }
}

fn evidence_requirements(graph: &CyberPlanGraph, id: &NodeId) -> Vec<String> {
    graph
        .nodes
        .get(id)
        .and_then(|evidence| evidence.path.as_ref())
        .is_some_and(|path| graph.root.join(path.as_str()).is_file())
        .then_some(Vec::new())
        .unwrap_or_else(|| {
            let Some(record) = graph.manifest.overrides.evidence.get(id) else {
                return vec![format!(
                    "required evidence `{id}` has no readable artifact or recorded gate"
                )];
            };
            record
                .source_paths
                .iter()
                .filter(|source_path| !graph.root.join(source_path.as_str()).is_file())
                .map(|source_path| {
                    format!("recorded evidence `{id}` source `{source_path}` is absent")
                })
                .collect()
        })
}

fn apply_dependency_override(
    graph: &mut CyberPlanGraph,
    from: NodeId,
    dependencies: Vec<NodeId>,
) -> Result<(), GraphError> {
    graph
        .edges
        .retain(|edge| !(edge.from == from && edge.kind == EdgeKind::DependsOn));
    dependencies.into_iter().try_for_each(|target| {
        graph.add_edge(GraphEdge {
            from: from.clone(),
            to: target,
            kind: EdgeKind::DependsOn,
        });
        Ok::<(), GraphError>(())
    })
}

fn state_for(
    graph: &CyberPlanGraph,
    id: &NodeId,
    visiting: &mut BTreeSet<NodeId>,
    reasons: &mut Vec<String>,
) -> DerivedState {
    let Some(node) = graph.nodes.get(id) else {
        reasons.push(format!("missing node `{id}`"));
        return DerivedState::Blocked;
    };
    if !visiting.insert(id.clone()) {
        reasons.push(format!("dependency cycle reaches `{id}`"));
        return DerivedState::Blocked;
    }
    let state = if let Some((authority_state, reason)) = authority_constraint(node) {
        reasons.push(reason.to_owned());
        authority_state
    } else if (node.kind == NodeKind::Workpack
        && node.metadata.get("workpackClass").map(String::as_str) == Some("intent-packet"))
        && dependencies_blocked(graph, id, visiting, reasons)
    {
        DerivedState::Blocked
    } else if node.lifecycle == LifecycleState::Done {
        let contract = graph.contract_result(node);
        if contract.is_complete() {
            DerivedState::Done
        } else {
            reasons.extend(contract.missing);
            DerivedState::Blocked
        }
    } else if node.lifecycle == LifecycleState::Planned {
        planned_state(graph, id, node, visiting, reasons)
    } else {
        stored_state(node.lifecycle)
    };
    visiting.remove(id);
    state
}

fn authority_constraint(node: &GraphNode) -> Option<(DerivedState, &'static str)> {
    if !node.id.as_str().starts_with("WP/") {
        return None;
    }
    let routing_status = node.metadata.get("routingStatus").map(String::as_str);
    let routing_constraint = match routing_status {
        Some("BLOCKED" | "PENDING") => Some((
            DerivedState::Blocked,
            "authoritative routing status is blocked or pending",
        )),
        Some("READY-AUDIT" | "VALIDATION") => Some((
            DerivedState::Validation,
            "authoritative routing status requires validation",
        )),
        _ => None,
    };
    routing_constraint.or_else(|| {
        (node.metadata.get("proofRowState").map(String::as_str) == Some("PENDING")).then_some((
            DerivedState::Validation,
            "authoritative proof row is pending",
        ))
    })
}

fn dependencies_blocked(
    graph: &CyberPlanGraph,
    id: &NodeId,
    visiting: &mut BTreeSet<NodeId>,
    reasons: &mut Vec<String>,
) -> bool {
    graph
        .dependencies(id)
        .into_iter()
        .map(|dependency| {
            let dependency_state = state_for(graph, &dependency, visiting, reasons);
            (dependency_state != DerivedState::Done)
                .then(|| {
                    reasons.push(format!("dependency `{dependency}` is {dependency_state:?}"));
                    true
                })
                .unwrap_or(false)
        })
        .any(|blocked| blocked)
}

fn planned_state(
    graph: &CyberPlanGraph,
    id: &NodeId,
    node: &GraphNode,
    visiting: &mut BTreeSet<NodeId>,
    reasons: &mut Vec<String>,
) -> DerivedState {
    let blocked = dependencies_blocked(graph, id, visiting, reasons);
    if blocked {
        DerivedState::Blocked
    } else if node.kind == NodeKind::Workpack {
        node.metadata
            .get("entryApproval")
            .map(|entry_approval| {
                reasons.push(format!("entry contract unresolved: {entry_approval}"));
                DerivedState::Blocked
            })
            .unwrap_or(DerivedState::Ready)
    } else {
        DerivedState::Planned
    }
}

fn stored_state(lifecycle: LifecycleState) -> DerivedState {
    [
        (LifecycleState::Active, DerivedState::Active),
        (LifecycleState::Validation, DerivedState::Validation),
        (LifecycleState::Failed, DerivedState::Failed),
        (LifecycleState::Paused, DerivedState::Paused),
    ]
    .into_iter()
    .find(|(stored, _)| *stored == lifecycle)
    .map(|(_, derived)| derived)
    .unwrap_or(DerivedState::Planned)
}

pub(super) struct ContractResult {
    pub(super) missing: Vec<String>,
}

impl ContractResult {
    pub(super) fn is_complete(&self) -> bool {
        self.missing.is_empty()
    }
}
