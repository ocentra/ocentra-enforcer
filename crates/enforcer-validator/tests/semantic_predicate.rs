//! UL13 route-coverage predicate matrix.

use enforcer_domain::hashes::Sha256;
use enforcer_domain::memory_types::{
    CommitId, GraphArtifactSchemaVersion, GraphEdgeCount, GraphNodeCount, ImpactSignal,
};
use enforcer_domain::paths::RelPath;
use enforcer_domain::semantic_types::{
    GraphFactProvider, GraphFactProviderKind, GraphFactState, GraphPredicateEvidence,
    GraphPredicateInput,
};
use enforcer_validator::semantic::{evaluate_route_coverage, RouteCoverageDisposition};
use std::error::Error;

struct FixtureProvider {
    evidence: GraphPredicateEvidence,
}

impl GraphFactProvider for FixtureProvider {
    fn route_coverage(&self, _input: &GraphPredicateInput) -> GraphPredicateEvidence {
        self.evidence.clone()
    }
}

fn input() -> Result<GraphPredicateInput, Box<dyn Error>> {
    Ok(GraphPredicateInput::new(
        RelPath::try_new("src/lib.rs")?,
        CommitId::try_new("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned())?,
    ))
}

fn evidence(
    state: GraphFactState,
    routes: GraphNodeCount,
    tests: GraphNodeCount,
) -> Result<GraphPredicateEvidence, Box<dyn Error>> {
    let input = input()?;
    let digest_text = format!("sha256:{}", "a".repeat(64));
    Ok(GraphPredicateEvidence {
        schema_version: GraphArtifactSchemaVersion::CURRENT,
        provider: GraphFactProviderKind::CodeGraph,
        source_commit: input.expected_commit,
        graph_digest: Sha256::try_new(&digest_text)?,
        state,
        route_file_count: routes,
        covering_test_count: tests,
        edge_count: GraphEdgeCount::from(3),
        has_test_coverage: ImpactSignal::from(usize::from(tests) > 0),
    })
}

#[test]
fn complete_route_with_test_coverage_passes() -> Result<(), Box<dyn Error>> {
    let evaluation = evaluate_route_coverage(
        &FixtureProvider {
            evidence: evidence(
                GraphFactState::Complete,
                GraphNodeCount::from(1),
                GraphNodeCount::from(1),
            )?,
        },
        &input()?,
    );
    assert_eq!(evaluation.disposition, RouteCoverageDisposition::Pass);
    Ok(())
}

#[test]
fn complete_route_without_test_coverage_fails() -> Result<(), Box<dyn Error>> {
    let evaluation = evaluate_route_coverage(
        &FixtureProvider {
            evidence: evidence(
                GraphFactState::Complete,
                GraphNodeCount::from(1),
                GraphNodeCount::from(0),
            )?,
        },
        &input()?,
    );
    assert_eq!(evaluation.disposition, RouteCoverageDisposition::Fail);
    Ok(())
}

#[test]
fn absent_route_is_no_claim() -> Result<(), Box<dyn Error>> {
    let evaluation = evaluate_route_coverage(
        &FixtureProvider {
            evidence: evidence(
                GraphFactState::Complete,
                GraphNodeCount::from(0),
                GraphNodeCount::from(0),
            )?,
        },
        &input()?,
    );
    assert_eq!(evaluation.disposition, RouteCoverageDisposition::NoClaim);
    Ok(())
}

#[test]
fn incomplete_states_never_pass_or_fail() -> Result<(), Box<dyn Error>> {
    for state in [
        GraphFactState::Partial,
        GraphFactState::Stale,
        GraphFactState::Cyclic,
        GraphFactState::Unavailable,
    ] {
        let evaluation = evaluate_route_coverage(
            &FixtureProvider {
                evidence: evidence(state, GraphNodeCount::from(1), GraphNodeCount::from(1))?,
            },
            &input()?,
        );
        assert_eq!(evaluation.disposition, RouteCoverageDisposition::NoClaim);
    }
    Ok(())
}
