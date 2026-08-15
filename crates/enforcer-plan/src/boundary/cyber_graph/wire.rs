//! BOUNDARY-INVARIANT: graph output wire types serialize validated read-only
//! views and cannot mutate or promote underlying plan truth.
//! NEGATIVE-TEST: output conversion has no path to vendor or external-system
//! execution and preserves validation findings.
use super::{
    BlockedReport, CatalogSummary, CyberPlanGraph, DerivedState, GraphError, GraphIssue, GraphPath,
    IntentSummary, IssueLevel, NodeId, NodeKind, NodeStatus, StatusReport, ValidationReport,
    WhyReport,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub(super) struct GraphIssueWire {
    level: IssueLevel,
    code: String,
    node: Option<NodeId>,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ValidationReportWire {
    node_count: usize,
    edge_count: usize,
    issues: Vec<GraphIssueWire>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct NodeStatusWire {
    id: NodeId,
    kind: NodeKind,
    title: String,
    state: DerivedState,
    path: Option<GraphPath>,
    reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct CatalogSummaryWire {
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
pub(super) struct IntentSummaryWire {
    family_count: usize,
    mapped_skill_count: usize,
    packet_count: usize,
    native_packet_count: usize,
    retention_packet_count: usize,
    protected_excluded: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct StatusReportWire {
    validation: ValidationReportWire,
    nodes_by_kind: BTreeMap<NodeKind, usize>,
    workpacks: Vec<NodeStatusWire>,
    catalog: CatalogSummaryWire,
    intent: IntentSummaryWire,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct BlockedReportWire {
    node: NodeStatusWire,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct WhyReportWire {
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

impl CyberPlanGraph {
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
        let selected = validation
            .is_valid()
            .then(|| candidates.first().map(NodeStatusWire::from))
            .flatten();
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
                "requires": "derived ready state and satisfied DependsOn nodes",
                "dependencySatisfaction": "DONE or an explicit READY entry-routing gate",
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
}
