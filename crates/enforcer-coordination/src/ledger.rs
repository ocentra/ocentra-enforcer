//! Minimal ledger materialization: replay the append-only event stream into
//! the set of currently-active claims.
//!
//! Ported (narrowed) from `src/coordination/vendor/materialize.js`. The full
//! JS `materialize()` also tracks lanes/workers/tasks/sessions/dashboard
//! stats; this pass ports only the ACTIVE-CLAIMS projection (claim/release/
//! claim.resolve event folding) because that is what `api::claim`,
//! `api::release`, and `api::closeout` need. Broader dashboard/session
//! materialization is deferred — see the crate-level deviation note.

use crate::events::boundary::{
    to_domain_valid_claim_paths, to_domain_valid_claim_writers, HubEventResponse,
};
use crate::lock::path_overlaps;
use crate::lock::singletons::normalize_coordination_path;
use crate::lock::RawClaim;
use enforcer_domain::coordination_types::ClaimPath;
use std::path::Path;

pub mod boundary;

/// Rebuild the read-only derived snapshot from canonical event streams.
pub fn materialize(root: &Path) -> crate::error::Result<boundary::LedgerSnapshot> {
    boundary::materialize(root)
}

/// A currently-active claim, keyed by `(writer, eventId)` in the JS source's
/// `claimIdentityKey`; here we key by event id alone since ids are globally
/// unique.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Owned active-claim projection materialized from the append-only event stream."]
pub struct ActiveClaim {
    pub raw: RawClaim,
}

/// Fold a full event history into the set of currently-active claims. Ported
/// (narrowed) from `materialize.js`'s claim/release/claim.resolve handling
/// (lines ~150-209).
pub fn active_claims(events: &[HubEventResponse]) -> Vec<RawClaim> {
    let mut claims = Vec::new();
    for event in events {
        match event.kind.as_str() {
            "claim" => {
                if let Some(paths) = &event.paths {
                    if paths.is_empty() {
                        continue;
                    }
                    let Ok(claim) = event.to_domain() else {
                        continue;
                    };
                    claims.push(claim);
                }
            }
            "release" => {
                if let Some(release_paths) = &event.paths {
                    let normalized_release =
                        normalize_valid_paths(to_domain_valid_claim_paths(release_paths));
                    claims.retain_mut(|claim| {
                        claim.paths.retain(|claim_path| {
                            let Some(normalized_claim) = normalized_claim_path(claim_path) else {
                                return true;
                            };
                            !normalized_release.iter().any(|release_path| {
                                matches!(
                                    path_overlaps(release_path, &normalized_claim),
                                    enforcer_domain::coordination_types::CoordinationMatch::Matches
                                )
                            })
                        });
                        !claim.paths.is_empty()
                    });
                }
            }
            "claim.resolve" => {
                if let Some(resolve_paths) = &event.paths {
                    let owners = event.owners.as_ref().map(|raw_owners| {
                        to_domain_valid_claim_writers(raw_owners)
                            .into_iter()
                            .collect::<std::collections::HashSet<_>>()
                    });
                    let normalized_resolve =
                        normalize_valid_paths(to_domain_valid_claim_paths(resolve_paths));
                    claims.retain(|claim| {
                        let overlaps = claim.paths.iter().any(|claim_path| {
                            let Some(normalized_claim) = normalized_claim_path(claim_path) else {
                                return false;
                            };
                            normalized_resolve.iter().any(|resolve_path| {
                                matches!(
                                    path_overlaps(resolve_path, &normalized_claim),
                                    enforcer_domain::coordination_types::CoordinationMatch::Matches
                                )
                            })
                        });
                        if !overlaps {
                            return true;
                        }
                        let should_resolve = match &owners {
                            Some(owners) => owners.contains(&claim.writer),
                            None => event.owner.as_deref() != Some(claim.writer.as_str()),
                        };
                        !should_resolve
                    });
                }
            }
            _ => {}
        }
    }
    claims
}

fn normalized_claim_path(path: &ClaimPath) -> Option<ClaimPath> {
    let Ok(path) = normalize_coordination_path(path) else {
        return None;
    };
    Some(path)
}

fn normalize_valid_paths(paths: Vec<ClaimPath>) -> Vec<ClaimPath> {
    let mut normalized = Vec::new();
    for path in paths {
        if let Ok(path) = normalize_coordination_path(&path) {
            normalized.push(path);
        }
    }
    normalized
}
