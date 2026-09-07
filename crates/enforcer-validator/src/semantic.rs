//! One bounded semantic predicate over the typed graph-provider contract.
//!
//! The predicate is deliberately narrower than security or correctness: it
//! reports whether a complete graph observation shows route impact with test
//! coverage. Missing, stale, partial, cyclic, and unavailable observations
//! produce `NoClaim`, never a clean result.

use enforcer_domain::semantic_types::{
    GraphFactProvider, GraphFactState, GraphPredicateEvidence, GraphPredicateInput,
};

/// Closed result of the UL13 route-coverage predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteCoverageDisposition {
    /// A complete route-impact observation includes test coverage.
    Pass,
    /// A complete route-impact observation has no test coverage.
    Fail,
    /// The observation cannot support this predicate without claiming facts.
    NoClaim,
}

/// Predicate result retaining the exact provider evidence used for the decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteCoverageEvaluation {
    /// Mechanical predicate disposition.
    pub disposition: RouteCoverageDisposition,
    /// Provider result retained for audit and explanation.
    pub evidence: GraphPredicateEvidence,
}

/// Evaluate one route-coverage predicate over a read-only provider.
pub fn evaluate_route_coverage(
    provider: &dyn GraphFactProvider,
    input: &GraphPredicateInput,
) -> RouteCoverageEvaluation {
    let evidence = provider.route_coverage(input);
    let route_count = usize::from(evidence.route_file_count);
    let disposition = match evidence.state {
        GraphFactState::Complete if route_count > 0 => {
            if evidence.has_test_coverage.is_present() {
                RouteCoverageDisposition::Pass
            } else {
                RouteCoverageDisposition::Fail
            }
        }
        GraphFactState::Complete
        | GraphFactState::Partial
        | GraphFactState::Stale
        | GraphFactState::Cyclic
        | GraphFactState::Unavailable => RouteCoverageDisposition::NoClaim,
    };
    RouteCoverageEvaluation {
        disposition,
        evidence,
    }
}
