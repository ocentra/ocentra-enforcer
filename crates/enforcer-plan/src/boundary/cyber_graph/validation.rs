//! BOUNDARY-INVARIANT: graph validation checks endpoints, cycles, protection,
//! and completion contracts without executing external or vendor systems.
//! NEGATIVE-TEST: missing endpoints, cycles, protected-source access, and
//! incomplete contracts are emitted as deterministic issues.
use super::json::missing_endpoint;
use super::{CyberPlanGraph, GraphEdge, GraphIssue, GraphNode, IssueLevel, LifecycleState, NodeId};
use std::collections::{BTreeMap, BTreeSet};

impl CyberPlanGraph {
    pub(crate) fn endpoint_issues(&self) -> Vec<GraphIssue> {
        self.edges
            .iter()
            .flat_map(|edge| endpoint_findings(edge, &self.nodes))
            .collect()
    }

    pub(crate) fn cycle_issues(&self) -> Vec<GraphIssue> {
        let mut findings = Vec::new();
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        for node in self.nodes.keys() {
            if let Some(cycle) = find_cycle(self, node, &mut visiting, &mut visited) {
                findings.push(cycle_issue(node, cycle));
            }
        }
        findings
    }

    pub(crate) fn done_contract_issues(&self) -> Vec<GraphIssue> {
        self.nodes
            .values()
            .filter(|node| node.lifecycle == LifecycleState::Done)
            .filter_map(|node| {
                let contract = self.contract_result(node);
                (!contract.is_complete()).then_some(GraphIssue {
                    level: IssueLevel::Error,
                    code: "DONE-CONTRACT".to_owned(),
                    node: Some(node.id.clone()),
                    message: contract.missing.join("; "),
                })
            })
            .collect()
    }

    pub(crate) fn protected_issues(&self) -> Vec<GraphIssue> {
        self.nodes
            .values()
            .filter(|node| {
                node.metadata.get("sourceAvailability").map(String::as_str)
                    == Some("sourceUnavailable")
            })
            .filter_map(protected_issue)
            .collect()
    }

    pub(crate) fn authority_issues(&self) -> Vec<GraphIssue> {
        self.nodes
            .values()
            .filter(|node| node.kind == super::NodeKind::Workpack)
            .filter(|node| node.id.as_str().starts_with("WP/"))
            .filter(|node| node.lifecycle == LifecycleState::Done)
            .filter_map(authority_lifecycle_issue)
            .collect()
    }
}

fn authority_lifecycle_issue(node: &GraphNode) -> Option<GraphIssue> {
    let routing_status = node.metadata.get("routingStatus").map(String::as_str);
    let routing_conflict = matches!(
        routing_status,
        Some("BLOCKED" | "PENDING" | "READY-AUDIT" | "VALIDATION")
    )
    .then(|| format!("lifecycle done contradicts routing status `{routing_status:?}`"));
    let proof_conflict = (node.metadata.get("proofRowState").map(String::as_str)
        == Some("PENDING"))
    .then_some("lifecycle done contradicts pending proof row".to_owned());
    routing_conflict
        .or(proof_conflict)
        .map(|message| GraphIssue {
            level: IssueLevel::Error,
            code: "AUTHORITY-LIFECYCLE-CONTRADICTION".to_owned(),
            node: Some(node.id.clone()),
            message,
        })
}

fn endpoint_findings(edge: &GraphEdge, nodes: &BTreeMap<NodeId, GraphNode>) -> Vec<GraphIssue> {
    let mut findings = Vec::new();
    if !nodes.contains_key(&edge.from) {
        findings.push(missing_endpoint(edge, &edge.from));
    }
    if !nodes.contains_key(&edge.to) {
        findings.push(missing_endpoint(edge, &edge.to));
    }
    findings
}

fn cycle_issue(node: &NodeId, cycle: Vec<NodeId>) -> GraphIssue {
    GraphIssue {
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
    }
}

fn protected_issue(node: &GraphNode) -> Option<GraphIssue> {
    node.path.is_some().then_some(GraphIssue {
        level: IssueLevel::Error,
        code: "PROTECTED-SOURCE-READ".to_owned(),
        node: Some(node.id.clone()),
        message: "sourceUnavailable node must not carry a materialized vendor path".to_owned(),
    })
}

fn find_cycle(
    graph: &CyberPlanGraph,
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
    for dependency in graph.dependencies(node) {
        if let Some(mut cycle) = find_cycle(graph, &dependency, visiting, visited) {
            cycle.insert(0, node.clone());
            return Some(cycle);
        }
    }
    visiting.remove(node);
    visited.insert(node.clone());
    None
}
