//! Typed read-only proof query contracts.
//!
//! These are intentionally owned by `enforcer-proof`: transports decode raw
//! input and serialize these DTOs, but neither discover proof state nor infer
//! a registry themselves.

use enforcer_domain::proof_types::{ProofCapability, ProofId, ProofStatus};
use enforcer_domain::{config_types::ConfigProfileName, paths::RelPath};

/// A route request against the packaged proof catalog.
#[derive(Debug, Clone, Default)]
pub struct ProofRouteQuery {
    pub proof_id: Option<ProofId>,
    pub files: Vec<RelPath>,
    pub plan: Option<String>,
    pub capability: Option<ProofCapability>,
    pub scope: Option<String>,
    pub profile: Option<ConfigProfileName>,
}

/// A bounded filter for persisted proof runs.
#[derive(Debug, Clone)]
pub struct ProofStatusQuery {
    pub proof_id: Option<ProofId>,
    pub status: Option<ProofStatus>,
    pub limit: usize,
}

/// An inventory request. Script rows remain opt-in and bounded.
#[derive(Debug, Clone)]
pub struct ProofInventoryQuery {
    pub include_scripts: bool,
    pub limit: usize,
}
