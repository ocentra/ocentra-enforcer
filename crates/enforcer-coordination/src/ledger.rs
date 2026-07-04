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
use crate::lock::{ClaimContext, RawClaim};

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
                    claims.insert(
                        event.id.clone(),
                        RawClaim {
                            writer: event.writer.clone(),
                            lane: event.lane.clone(),
                            paths: paths.clone(),
                            event_id: event.id.clone(),
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
                        !claim.paths.iter().any(|claim_path| {
                            let cp = normalize_coordination_path(claim_path);
                            normalized_release
                                .iter()
                                .any(|rp| path_overlaps(rp, &cp))
                        })
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
                            normalized_resolve
                                .iter()
                                .any(|rp| path_overlaps(rp, &cp))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn claim_event(id: &str, writer: &str, lane: &str, paths: &[&str]) -> HubEvent {
        HubEvent {
            id: id.into(),
            schema: 1,
            hub: "hub".into(),
            node_id: "node".into(),
            node_name: "Node".into(),
            lane: lane.into(),
            writer: writer.into(),
            kind: "claim".into(),
            ts: "2026-07-04T00:00:00.000Z".into(),
            seq: 1,
            prev_event_id: None,
            prev_hash: None,
            hash: "sha256:0".into(),
            to: None,
            body: None,
            message_id: None,
            paths: Some(paths.iter().map(|p| p.to_string()).collect()),
            reason: None,
            owner: None,
            owners: None,
            state: None,
            worker_state: None,
            task_id: None,
            task_state: None,
            title: None,
            pr_url: None,
            summary: None,
            ttl_seconds: None,
            session_id: None,
            context: None,
        }
    }

    fn release_event(id: &str, writer: &str, lane: &str, paths: &[&str]) -> HubEvent {
        let mut event = claim_event(id, writer, lane, paths);
        event.kind = "release".into();
        event
    }

    #[test]
    fn claim_then_release_clears_the_active_claim() {
        let events = vec![
            claim_event("evt1", "node.laneA", "laneA", &["src/lib.rs"]),
            release_event("evt2", "node.laneA", "laneA", &["src/lib.rs"]),
        ];
        assert!(active_claims(&events).is_empty());
    }

    #[test]
    fn unreleased_claim_remains_active() {
        let events = vec![claim_event("evt1", "node.laneA", "laneA", &["src/lib.rs"])];
        let claims = active_claims(&events);
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].paths, vec!["src/lib.rs".to_string()]);
    }
}
