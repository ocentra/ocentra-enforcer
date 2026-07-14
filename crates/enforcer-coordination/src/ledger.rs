//! Minimal ledger materialization: replay the append-only event stream into
//! the set of currently-active claims.
//!
//! Ported (narrowed) from `src/coordination/vendor/materialize.js`. The full
//! JS `materialize()` also tracks lanes/workers/tasks/sessions/dashboard
//! stats; this pass ports only the ACTIVE-CLAIMS projection (claim/release/
//! claim.resolve event folding) because that is what `api::claim`,
//! `api::release`, and `api::closeout` need. Broader dashboard/session
//! materialization is deferred — see the crate-level deviation note.

use std::collections::BTreeMap;

use crate::events::HubEvent;
use crate::lock::path_overlaps;
use crate::lock::singletons::normalize_coordination_path;
use crate::lock::{ClaimContext, ClaimEventId, ClaimLane, ClaimWriter, RawClaim};

/// A currently-active claim, keyed by `(writer, eventId)` in the JS source's
/// `claimIdentityKey`; here we key by event id alone since ids are globally
/// unique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveClaim {
    pub raw: RawClaim,
}

/// Fold a full event history into the set of currently-active claims. Ported
/// (narrowed) from `materialize.js`'s claim/release/claim.resolve handling
/// (lines ~150-209).
pub fn active_claims(events: &[HubEvent]) -> Vec<RawClaim> {
    let mut claims: BTreeMap<String, RawClaim> = BTreeMap::new();
    for event in events {
        match event.kind.as_str() {
            "claim" => {
                if let Some(paths) = &event.paths {
                    let context = context_from_event(event);
                    // CLONE-JUSTIFICATION: the active-claim index owns its
                    // event key independently of the durable claim snapshot.
                    claims.insert(
                        event.id.clone(),
                        RawClaim {
                            // CLONE-JUSTIFICATION: materialization produces
                            // an owned snapshot after the event stream borrow
                            // ends; its identity fields cannot borrow events.
                            writer: ClaimWriter::from(event.writer.clone()),
                            lane: ClaimLane::from(event.lane.clone()),
                            // CLONE-JUSTIFICATION: the active snapshot keeps
                            // its claimed paths, event identity, and optional
                            // release reason after stream replay completes.
                            paths: paths.clone(),
                            event_id: ClaimEventId::from(event.id.clone()),
                            // CLONE-JUSTIFICATION: the active projection
                            // owns the optional reason after replay releases
                            // the source event borrow.
                            reason: event.reason.clone(),
                            context,
                        },
                    );
                }
            }
            "release" => {
                if let Some(release_paths) = &event.paths {
                    let normalized_release: Vec<String> = release_paths
                        .iter()
                        .map(|p| normalize_coordination_path(p))
                        .collect();
                    claims.retain(|_, claim| {
                        claim.paths.retain(|claim_path| {
                            let cp = normalize_coordination_path(claim_path);
                            !normalized_release.iter().any(|rp| path_overlaps(rp, &cp))
                        });
                        !claim.paths.is_empty()
                    });
                }
            }
            "claim.resolve" => {
                if let Some(resolve_paths) = &event.paths {
                    let owners: Option<std::collections::HashSet<String>> = event
                        .owners
                        .as_ref()
                        .map(|owners| owners.iter().cloned().collect());
                    let normalized_resolve: Vec<String> = resolve_paths
                        .iter()
                        .map(|p| normalize_coordination_path(p))
                        .collect();
                    claims.retain(|_, claim| {
                        let overlaps = claim.paths.iter().any(|claim_path| {
                            let cp = normalize_coordination_path(claim_path);
                            normalized_resolve.iter().any(|rp| path_overlaps(rp, &cp))
                        });
                        if !overlaps {
                            return true;
                        }
                        let should_resolve = match &owners {
                            Some(owners) => owners.contains(claim.writer.as_str()),
                            None => event.owner.as_deref() != Some(claim.writer.as_str()),
                        };
                        !should_resolve
                    });
                }
            }
            _ => {}
        }
    }
    claims.into_values().collect()
}

fn context_from_event(event: &HubEvent) -> ClaimContext {
    let Some(value) = &event.context else {
        return ClaimContext::default();
    };
    let get = |key: &str| -> Option<String> {
        value.get(key).and_then(|v| v.as_str()).map(str::to_owned)
    };
    ClaimContext {
        project_id: get("projectId"),
        git_remote: get("gitRemote"),
        repo_root: get("repoRoot"),
        worktree_root: get("worktreeRoot"),
        branch: get("branch"),
        codex_thread_id: get("codexThreadId"),
        codex_session_id: get("codexSessionId"),
        explicit_codex_thread_id: get("explicitCodexThreadId"),
        explicit_codex_session_id: get("explicitCodexSessionId"),
        claim_group: get("claimGroup"),
        lock_kind: get("lockKind"),
        operation: get("operation"),
    }
}
