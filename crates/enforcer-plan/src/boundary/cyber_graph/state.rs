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
                apply_dependency_override(self, &from, dependencies)
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
        (node.completion.checklist_complete < node.completion.checklist_total)
            .then(|| {
                format!(
                    "checklist is {}/{} complete",
                    node.completion.checklist_complete, node.completion.checklist_total
                )
            })
            .into_iter()
            .for_each(|message| missing.push(message));
        ContractResult { missing }
    }
}

fn evidence_requirements(graph: &CyberPlanGraph, id: &NodeId) -> Vec<String> {
    graph
        .nodes
        .get(id)
        .and_then(|evidence| evidence.path.as_ref())
        .filter(|path| graph.root.join(path.as_str()).is_file())
        .map(|_| Vec::new())
        .or_else(|| {
            graph.manifest.overrides.evidence.get(id).map(|record| {
                record
                    .source_paths
                    .iter()
                    .filter(|source_path| !graph.root.join(source_path.as_str()).is_file())
                    .map(|source_path| {
                        format!("recorded evidence `{id}` source `{source_path}` is absent")
                    })
                    .collect()
            })
        })
        .unwrap_or_else(|| {
            vec![format!(
                "required evidence `{id}` has no readable artifact or recorded gate"
            )]
        })
}

fn apply_dependency_override(
    graph: &mut CyberPlanGraph,
    from: &NodeId,
    dependencies: Vec<NodeId>,
) -> Result<(), GraphError> {
    graph
        .edges
        .retain(|edge| !(edge.from == *from && edge.kind == EdgeKind::DependsOn));
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
    let state = authority_constraint(node)
        .map(|(authority_state, reason)| {
            reasons.push(reason.to_owned());
            authority_state
        })
        .or_else(|| {
            matches!(
                (
                    node.kind,
                    node.metadata.get("workpackClass").map(String::as_str)
                ),
                (NodeKind::Workpack, Some("intent-packet"))
            )
            .then(|| {
                dependencies_blocked(graph, id, visiting, reasons).then_some(DerivedState::Blocked)
            })
            .flatten()
        })
        .or_else(|| {
            (node.lifecycle == LifecycleState::Done)
                .then(|| {
                    let contract = graph.contract_result(node);
                    contract
                        .is_complete()
                        .then_some(DerivedState::Done)
                        .or_else(|| {
                            reasons.extend(contract.missing);
                            Some(DerivedState::Blocked)
                        })
                })
                .flatten()
        })
        .or_else(|| {
            (node.lifecycle == LifecycleState::Planned)
                .then(|| planned_state(graph, id, node, visiting, reasons))
        })
        .unwrap_or_else(|| stored_state(node.lifecycle));
    visiting.remove(id);
    state
}

fn authority_constraint(node: &GraphNode) -> Option<(DerivedState, &'static str)> {
    node.id
        .as_str()
        .starts_with("WP/")
        .then(|| {
            let routing_status = node.metadata.get("routingStatus").map(String::as_str);
            let routing_constraint = routing_status.and_then(|status| {
                [
                    (
                        "BLOCKED",
                        DerivedState::Blocked,
                        "authoritative routing status is blocked or pending",
                    ),
                    (
                        "PENDING",
                        DerivedState::Blocked,
                        "authoritative routing status is blocked or pending",
                    ),
                    (
                        "READY-AUDIT",
                        DerivedState::Validation,
                        "authoritative routing status requires validation",
                    ),
                    (
                        "VALIDATION",
                        DerivedState::Validation,
                        "authoritative routing status requires validation",
                    ),
                ]
                .into_iter()
                .find_map(|(candidate, state, reason)| {
                    (candidate == status).then_some((state, reason))
                })
            });
            let proof_allowed = routing_status != Some("READY");
            routing_constraint.or_else(|| {
                proof_allowed
                    .then(|| {
                        (node.metadata.get("proofRowState").map(String::as_str) == Some("PENDING"))
                            .then_some((
                                DerivedState::Validation,
                                "authoritative proof row is pending",
                            ))
                    })
                    .flatten()
            })
        })
        .flatten()
}

pub(super) fn is_ready_entry_gate(graph: &CyberPlanGraph, id: &NodeId) -> bool {
    graph.nodes.get(id).is_some_and(|node| {
        node.kind == NodeKind::Workpack
            && node.metadata.get("routingStatus").map(String::as_str) == Some("READY")
            && node.metadata.get("workpackClass").map(String::as_str) != Some("intent-packet")
            && node.metadata.get("proofRowState").map(String::as_str) == Some("PENDING")
            && graph.edges.iter().any(|edge| {
                edge.kind == EdgeKind::DependsOn
                    && edge.to == *id
                    && graph.nodes.get(&edge.from).is_some_and(|packet| {
                        packet.kind == NodeKind::Workpack
                            && packet.metadata.get("workpackClass").map(String::as_str)
                                == Some("intent-packet")
                    })
            })
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
        .find_map(|dependency| {
            let dependency_state = state_for(graph, &dependency, visiting, reasons);
            let satisfied = dependency_state == DerivedState::Done
                || (dependency_state == DerivedState::Ready
                    && is_ready_entry_gate(graph, &dependency));
            (!satisfied).then(|| {
                reasons.push(format!("dependency `{dependency}` is {dependency_state:?}"));
                true
            })
        })
        .is_some()
}

fn planned_state(
    graph: &CyberPlanGraph,
    id: &NodeId,
    node: &GraphNode,
    visiting: &mut BTreeSet<NodeId>,
    reasons: &mut Vec<String>,
) -> DerivedState {
    dependencies_blocked(graph, id, visiting, reasons)
        .then_some(DerivedState::Blocked)
        .or_else(|| {
            (node.kind == NodeKind::Workpack).then(|| {
                node.metadata
                    .get("entryApproval")
                    .map(|entry_approval| {
                        reasons.push(format!("entry contract unresolved: {entry_approval}"));
                        DerivedState::Blocked
                    })
                    .unwrap_or(DerivedState::Ready)
            })
        })
        .unwrap_or(DerivedState::Planned)
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
