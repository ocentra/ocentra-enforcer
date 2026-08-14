//! Adapter from the repository-owned `CodeGraph` to UL13 semantic facts.
//!
//! This adapter is read-only and bounded. It reuses the existing impact
//! analysis traversal; it does not expose SQLite, persistence, raw parser
//! nodes, or arbitrary process execution to validator consumers.

use crate::code_graph::CodeGraph;
use crate::impact::{analyze_diff_impact_scoped, DEFAULT_DEPTH};
use enforcer_domain::hashes::Sha256;
use enforcer_domain::memory_types::{
    CommitId, GraphArtifactSchemaVersion, GraphEdgeCount, GraphNodeCount, ImpactScope, ImpactSignal,
};
use enforcer_domain::semantic_types::{
    GraphFactProvider, GraphFactProviderKind, GraphFactState, GraphPredicateEvidence,
    GraphPredicateInput,
};
use std::fmt;

/// Read-only UL13 provider backed by an existing in-process `CodeGraph`.
pub struct CodeGraphRouteCoverageProvider<'a> {
    graph: &'a CodeGraph,
    source_commit: CommitId,
    graph_digest: Sha256,
}

impl fmt::Debug for CodeGraphRouteCoverageProvider<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CodeGraphRouteCoverageProvider")
            .field("source_commit", &self.source_commit)
            .field("graph_digest", &self.graph_digest)
            .finish()
    }
}

impl<'a> CodeGraphRouteCoverageProvider<'a> {
    /// Bind a graph snapshot to its independently verified commit and digest.
    pub fn new(graph: &'a CodeGraph, source_commit: CommitId, graph_digest: Sha256) -> Self {
        Self {
            graph,
            source_commit,
            graph_digest,
        }
    }

    fn edge_count(&self) -> GraphEdgeCount {
        [
            self.graph.imports().len(),
            self.graph.calls().len(),
            self.graph.routes().len(),
            self.graph.inherits().len(),
            self.graph.implements().len(),
            self.graph.decorates().len(),
            self.graph.type_refs().len(),
            self.graph.defines().len(),
        ]
        .into_iter()
        .sum::<usize>()
        .into()
    }

    fn evidence(
        &self,
        state: GraphFactState,
        route_file_count: GraphNodeCount,
        covering_test_count: GraphNodeCount,
    ) -> GraphPredicateEvidence {
        GraphPredicateEvidence {
            schema_version: GraphArtifactSchemaVersion::CURRENT,
            provider: GraphFactProviderKind::CodeGraph,
            // CLONE-JUSTIFICATION: the evidence snapshot owns provenance values
            // independently of the provider's bound graph snapshot.
            source_commit: self.source_commit.clone(),
            // CLONE-JUSTIFICATION: the evidence snapshot owns the graph digest
            // independently of the provider's bound graph snapshot.
            graph_digest: self.graph_digest.clone(),
            state,
            route_file_count,
            covering_test_count,
            edge_count: self.edge_count(),
            has_test_coverage: ImpactSignal::from(usize::from(covering_test_count) > 0),
        }
    }
}

impl GraphFactProvider for CodeGraphRouteCoverageProvider<'_> {
    fn route_coverage(&self, input: &GraphPredicateInput) -> GraphPredicateEvidence {
        if input.expected_commit != self.source_commit {
            return self.evidence(
                GraphFactState::Stale,
                GraphNodeCount::from(0),
                GraphNodeCount::from(0),
            );
        }

        let changed_path = input.changed_path.as_str().into();
        let report = analyze_diff_impact_scoped(
            self.graph,
            std::slice::from_ref(&changed_path),
            DEFAULT_DEPTH.into(),
            ImpactScope::All,
        );
        let impact = report
            .impacted
            .iter()
            .find(|entry| entry.rel_path.as_str() == input.changed_path.as_str());
        let (route_file_count, covering_test_count) = impact.map_or(
            (GraphNodeCount::from(0), GraphNodeCount::from(0)),
            |entry| {
                (
                    GraphNodeCount::from(entry.factors.downstream_route_file_ids.len()),
                    GraphNodeCount::from(entry.factors.covering_test_ids.len()),
                )
            },
        );
        self.evidence(
            GraphFactState::Complete,
            route_file_count,
            covering_test_count,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::CodeGraphRouteCoverageProvider;
    use enforcer_domain::hashes::Sha256;
    use enforcer_domain::memory_types::CommitId;
    use enforcer_domain::paths::RelPath;
    use enforcer_domain::semantic_types::{GraphFactProvider, GraphFactState, GraphPredicateInput};
    use std::error::Error;

    fn input(commit: CommitId) -> Result<GraphPredicateInput, Box<dyn Error>> {
        Ok(GraphPredicateInput::new(
            RelPath::try_new("src/lib.rs")?,
            commit,
        ))
    }

    fn provider<'a>(
        graph: &'a crate::code_graph::CodeGraph,
        source_commit: CommitId,
        graph_digest: Sha256,
    ) -> CodeGraphRouteCoverageProvider<'a> {
        CodeGraphRouteCoverageProvider::new(graph, source_commit, graph_digest)
    }

    #[test]
    fn empty_graph_is_complete_but_not_a_route_claim() -> Result<(), Box<dyn Error>> {
        let graph = crate::code_graph::CodeGraph::new();
        let commit = CommitId::try_new("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned())?;
        let digest_text = format!("sha256:{}", "a".repeat(64));
        let provider = provider(&graph, commit.clone(), Sha256::try_new(&digest_text)?);
        let result = provider.route_coverage(&input(commit)?);
        assert_eq!(result.state, GraphFactState::Complete);
        assert_eq!(usize::from(result.route_file_count), 0);
        Ok(())
    }

    #[test]
    fn commit_mismatch_is_stale() -> Result<(), Box<dyn Error>> {
        let graph = crate::code_graph::CodeGraph::new();
        let source_commit =
            CommitId::try_new("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned())?;
        let requested_commit =
            CommitId::try_new("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned())?;
        let digest_text = format!("sha256:{}", "a".repeat(64));
        let provider = provider(&graph, source_commit, Sha256::try_new(&digest_text)?);
        let result = provider.route_coverage(&input(requested_commit)?);
        assert_eq!(result.state, GraphFactState::Stale);
        Ok(())
    }
}
