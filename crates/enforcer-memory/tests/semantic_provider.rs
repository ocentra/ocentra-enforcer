//! UL13 adapter proof over the existing memory `CodeGraph` boundary.

use enforcer_domain::hashes::Sha256;
use enforcer_domain::memory_types::CommitId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::semantic_types::{GraphFactProvider, GraphFactState, GraphPredicateInput};
use enforcer_memory::code_graph::CodeGraph;
use enforcer_memory::semantic_provider::CodeGraphRouteCoverageProvider;
use std::error::Error;

#[test]
fn code_graph_provider_returns_versioned_complete_no_route_evidence_for_empty_graph(
) -> Result<(), Box<dyn Error>> {
    let graph = CodeGraph::new();
    let commit = CommitId::try_new("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned())?;
    let digest_text = format!("sha256:{}", "b".repeat(64));
    let digest = Sha256::try_new(&digest_text)?;
    let provider = CodeGraphRouteCoverageProvider::new(&graph, commit.clone(), digest);
    let request = GraphPredicateInput::new(RelPath::try_new("src/lib.rs")?, commit);

    let evidence = provider.route_coverage(&request);
    assert_eq!(evidence.state, GraphFactState::Complete);
    assert_eq!(usize::from(evidence.route_file_count), 0);
    assert_eq!(usize::from(evidence.edge_count), 0);
    Ok(())
}
