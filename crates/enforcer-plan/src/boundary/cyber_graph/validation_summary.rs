//! BOUNDARY-INVARIANT: status summaries report coverage and lifecycle facts
//! separately, never treating catalog decomposition as native proof.
//! NEGATIVE-TEST: cycles and unresolved dependencies remain visible in the
//! explanation chain instead of being collapsed into readiness.
use super::json::count_coverage;
use super::state::is_ready_entry_gate;
use super::{
    CatalogSummary, CyberPlanGraph, DerivedState, EdgeKind, IntentSummary, NodeId, NodeKind,
};
use std::collections::BTreeSet;

impl CyberPlanGraph {
    pub(crate) fn catalog_summary(&self) -> CatalogSummary {
        let mut summary = CatalogSummary::default();
        self.nodes
            .values()
            .filter(|node| node.kind == NodeKind::Skill)
            .for_each(|node| {
                summary.total += 1;
                let unavailable = node.metadata.get("sourceAvailability").map(String::as_str)
                    == Some("sourceUnavailable");
                summary.source_unavailable += usize::from(unavailable);
                summary.available += usize::from(!unavailable);
                let decomposition = node.metadata.get("decomposition").map(String::as_str);
                summary.decomposed_complete += usize::from(decomposition == Some("complete"));
                summary.decomposed_partial += usize::from(decomposition == Some("partial"));
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
            });
        summary
    }

    pub(crate) fn intent_summary(&self) -> IntentSummary {
        let skills = self
            .nodes
            .values()
            .filter(|node| node.kind == NodeKind::Skill);
        let packets = self
            .nodes
            .values()
            .filter(|node| node.kind == NodeKind::Workpack)
            .filter(|node| {
                node.metadata.get("workpackClass").map(String::as_str) == Some("intent-packet")
            });
        let mut summary = IntentSummary {
            protected_excluded: skills
                .clone()
                .filter(|node| {
                    node.metadata.get("protectedBoundary").map(String::as_str) == Some("excluded")
                })
                .count(),
            ..IntentSummary::default()
        };
        summary.family_count = self
            .nodes
            .values()
            .filter(|node| node.kind == NodeKind::IntentFamily)
            .count();
        summary.packet_count = packets.clone().count();
        summary.retention_packet_count = packets
            .clone()
            .filter(|node| node.metadata.get("route").map(String::as_str) == Some("CP11"))
            .count();
        summary.native_packet_count = packets
            .filter(|node| {
                matches!(
                    node.metadata.get("route").map(String::as_str),
                    Some("CP09" | "CP12")
                )
            })
            .count();
        summary.mapped_skill_count = skills
            .filter(|node| {
                self.edges
                    .iter()
                    .any(|edge| edge.kind == EdgeKind::Classifies && edge.to == node.id)
            })
            .count();
        summary
    }

    pub(super) fn explain(
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
        blockers.extend(
            self.inspect(id)
                .ok()
                .into_iter()
                .flat_map(|status| status.reasons),
        );
        self.dependencies(id)
            .into_iter()
            .filter(|dependency| {
                let state = self
                    .inspect(dependency)
                    .map(|status| status.state)
                    .unwrap_or(DerivedState::Blocked);
                state != DerivedState::Done
                    && (state != DerivedState::Ready || !is_ready_entry_gate(self, dependency))
            })
            .for_each(|dependency| self.explain(&dependency, chain, blockers, visiting));
        visiting.remove(id);
    }
}
