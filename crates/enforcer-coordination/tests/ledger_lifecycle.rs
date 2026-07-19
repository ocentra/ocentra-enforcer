//! External ledger materialization contract tests.

use enforcer_coordination::events::boundary::HubEventResponse;
use enforcer_coordination::ledger::active_claims;
use enforcer_domain::coordination_types::ClaimPath;

fn claim_event(id: &str, writer: &str, lane: &str, paths: &[&str]) -> HubEventResponse {
    HubEventResponse {
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
        paths: Some(paths.iter().map(|path| (*path).to_owned()).collect()),
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

fn release_event(id: &str, writer: &str, lane: &str, paths: &[&str]) -> HubEventResponse {
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
fn unreleased_claim_remains_active() -> Result<(), Box<dyn std::error::Error>> {
    let events = vec![claim_event("evt1", "node.laneA", "laneA", &["src/lib.rs"])];
    let claims = active_claims(&events);
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].paths, vec![ClaimPath::parse("src/lib.rs")?]);
    Ok(())
}
