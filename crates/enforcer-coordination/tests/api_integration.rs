//! Public API proof for the coordination hub.
//!
//! These tests exercise the crate from outside its implementation module so
//! callers and the Rust API stay aligned without inline implementation tests.

use std::path::Path;

use enforcer_coordination::api::{
    acknowledge_message, claim_all, closeout, init, load_identity, normalize_owns_paths, open,
    release, send_message, CallerContext, ClaimRequestArgs, CloseoutFilters, Hub,
};
use enforcer_coordination::events::boundary::HubEventResponse;
use enforcer_coordination::ledger::active_claims;
use enforcer_domain::coordination_types::{
    ClaimEventId, ClaimPath, CoordinationBranch, CoordinationLedgerRoot, CoordinationMessageBody,
    CoordinationProjectId, CoordinationRepoRoot, CoordinationWorktree,
};
use enforcer_domain::ids::{HubName, LaneId};
use enforcer_domain::scan_types::CommitRef;
use proptest::{prop_assert, prop_assert_eq, proptest};
use tempfile::tempdir;

fn caller(worktree: &str, branch: &str) -> Result<CallerContext, Box<dyn std::error::Error>> {
    Ok(CallerContext {
        project_id: CoordinationProjectId::parse("test-project")?,
        worktree_root: CoordinationWorktree::parse(worktree)?,
        branch: CoordinationBranch::parse(branch)?,
        commit: Some("abc123".parse::<CommitRef>()?),
        codex_thread_id: None,
        codex_session_id: None,
    })
}

fn open_hub(root: &Path, hub_name: &str, lane: &str) -> Result<Hub, Box<dyn std::error::Error>> {
    let hub: HubName = hub_name.parse()?;
    let lane_id: LaneId = lane.parse()?;
    init(root, &hub, &lane_id)?;
    Ok(open(CoordinationLedgerRoot::parse(root)?)?)
}

#[test]
fn init_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let hub: HubName = "test-hub".parse()?;
    let lane: LaneId = "primary".parse()?;
    let first = init(dir.path(), &hub, &lane)?;
    let second = init(dir.path(), &hub, &lane)?;
    assert_eq!(first, second);
    Ok(())
}

#[test]
fn load_identity_rejects_a_blank_persisted_hub_name() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let identity_dir = dir.path().join("identity");
    std::fs::create_dir_all(&identity_dir)?;
    std::fs::write(
        identity_dir.join("node.json"),
        r#"{"hub":" ","nodeId":"node","nodeName":"node","defaultLane":"main","createdAt":"2026-07-19T00:00:00.000Z"}"#,
    )?;

    let result = load_identity(dir.path());
    let error = result.err().ok_or("blank persisted hub must be rejected")?;
    assert_eq!(
        error.to_string(),
        "coordination decode error: decode/validation failed at `hubName`: expected lowercase kebab-case (e.g. `enforcer-rust-build`)"
    );
    Ok(())
}

#[test]
fn claim_uses_explicit_caller_context() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let hub = open_hub(dir.path(), "test-hub", "arc-16")?;
    let repo = tempdir()?;
    std::fs::write(repo.path().join("lib.rs"), "// fixture file")?;
    let lane: LaneId = "arc-16".parse()?;
    let caller_worktree = "C:/Projects/some-other-worktree";
    let repo_root = CoordinationRepoRoot::parse(repo.path())?;
    let outcome = claim_all(
        &hub,
        ClaimRequestArgs {
            repo_root: &repo_root,
            lane: &lane,
            owns: &[ClaimPath::from_static("lib.rs")?],
            caller: &caller(caller_worktree, "lane/arc-16")?,
            reason: None,
        },
    )?;
    let context = outcome.events[0]
        .context
        .as_ref()
        .ok_or("context missing")?;
    assert_eq!(
        context.get("worktreeRoot").and_then(|value| value.as_str()),
        Some(caller_worktree)
    );
    assert_eq!(
        context.get("branch").and_then(|value| value.as_str()),
        Some("lane/arc-16")
    );
    assert_eq!(
        context.get("commit").and_then(|value| value.as_str()),
        Some("abc123")
    );
    Ok(())
}

#[test]
fn claim_batches_directory_owns_sets() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let hub = open_hub(dir.path(), "test-hub", "arc-16")?;
    let repo = tempdir()?;
    let crate_dir = repo.path().join("crates/big-crate/src");
    std::fs::create_dir_all(&crate_dir)?;
    let module_names = (0..15)
        .map(|number| format!("mod{number}.rs"))
        .collect::<Vec<_>>();
    for module_name in module_names {
        std::fs::write(crate_dir.join(module_name), "// fixture file")?;
    }
    let lane: LaneId = "arc-16".parse()?;
    let repo_root = CoordinationRepoRoot::parse(repo.path())?;
    let outcome = claim_all(
        &hub,
        ClaimRequestArgs {
            repo_root: &repo_root,
            lane: &lane,
            owns: &[ClaimPath::from_static("crates/big-crate/**")?],
            caller: &caller("C:/Projects/wt", "lane/arc-16")?,
            reason: None,
        },
    )?;
    assert_eq!(outcome.events.len(), 2);
    Ok(())
}

proptest! {
    #[test]
    fn normalize_owns_paths_is_stable_for_generated_exact_paths(
        raw_entries in proptest::collection::vec("[a-z]{1,8}\\.rs", 1..16),
    ) {
        let entries = raw_entries
            .into_iter()
            .filter_map(|raw| ClaimPath::parse(&raw).ok())
            .collect::<Vec<_>>();
        let temp_root = std::env::temp_dir();
        let Ok(root) = CoordinationRepoRoot::parse(&temp_root) else {
            prop_assert!(false, "system temp directory is a valid coordination root");
            return Ok(());
        };
        let Ok(first) = normalize_owns_paths(&root, &entries) else {
            prop_assert!(false, "exact generated paths normalize without filesystem errors");
            return Ok(());
        };
        let Ok(second) = normalize_owns_paths(&root, &first) else {
            prop_assert!(false, "normalized exact paths remain valid inputs");
            return Ok(());
        };

        prop_assert_eq!(&first, &second);
        prop_assert!(second.iter().all(|path| !path.as_str().contains('\\')));
    }
}

#[test]
fn closeout_does_not_release_another_lane() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let hub = open_hub(dir.path(), "test-hub", "primary")?;
    let repo = tempdir()?;
    std::fs::write(repo.path().join("a.rs"), "// a")?;
    std::fs::write(repo.path().join("b.rs"), "// b")?;
    let lane_a: LaneId = "lane-a".parse()?;
    let lane_b: LaneId = "lane-b".parse()?;
    let repo_root = CoordinationRepoRoot::parse(repo.path())?;
    for (lane, file) in [(&lane_a, "a.rs"), (&lane_b, "b.rs")] {
        claim_all(
            &hub,
            ClaimRequestArgs {
                repo_root: &repo_root,
                lane,
                owns: &[ClaimPath::parse(file)?],
                caller: &caller("wt", lane.as_str())?,
                reason: None,
            },
        )?;
    }
    let filters = CloseoutFilters {
        lane: Some(lane_a.clone()),
        ..Default::default()
    };
    let events = closeout(&hub, &lane_a, &filters, &caller("wt-a", "br-a")?, None)?;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].lane, "lane-a");

    let after_selected = enforcer_coordination::sync::stream::read_all_streams(hub.root.as_path())?;
    let remaining = active_claims(&after_selected.events);
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].lane.as_str(), "lane-b");
    assert_eq!(remaining[0].paths, vec![ClaimPath::from_static("b.rs")?]);

    let cross_lane_filters = CloseoutFilters {
        lane: Some(lane_a.clone()),
        lane_scope: enforcer_domain::coordination_types::CloseoutLaneScope::AllLanes,
        ..Default::default()
    };
    let cross_lane_events = closeout(
        &hub,
        &lane_a,
        &cross_lane_filters,
        &caller("wt-a", "br-a")?,
        None,
    )?;
    assert_eq!(cross_lane_events.len(), 1);
    assert_eq!(cross_lane_events[0].lane, "lane-b");
    let after_cross_lane =
        enforcer_coordination::sync::stream::read_all_streams(hub.root.as_path())?;
    assert!(active_claims(&after_cross_lane.events).is_empty());
    Ok(())
}

#[test]
fn release_and_message_acknowledgement_are_public_operations(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let hub = open_hub(dir.path(), "test-hub", "desktop")?;
    let repo = tempdir()?;
    std::fs::write(repo.path().join("a.rs"), "// a")?;
    let lane: LaneId = "desktop".parse()?;
    let repo_root = CoordinationRepoRoot::parse(repo.path())?;
    claim_all(
        &hub,
        ClaimRequestArgs {
            repo_root: &repo_root,
            lane: &lane,
            owns: &[ClaimPath::from_static("a.rs")?],
            caller: &caller("wt", "branch")?,
            reason: None,
        },
    )?;
    let released = release(
        &hub,
        &lane,
        &[ClaimPath::from_static("a.rs")?],
        &caller("wt", "branch")?,
        None,
    )?;
    assert_eq!(released.kind, "release");
    let recipient: LaneId = "reviewer".parse()?;
    let body = CoordinationMessageBody::from_static("Please inspect.")?;
    let message = send_message(&hub, &lane, recipient, body, &caller("wt", "branch")?)?;
    let message_id = ClaimEventId::try_from(message.id.clone())?;
    let acknowledgement = acknowledge_message(&hub, &lane, message_id, &caller("wt", "branch")?)?;
    assert_eq!(
        acknowledgement.message_id.as_deref(),
        Some(message.id.as_str())
    );
    Ok(())
}

#[test]
fn releasing_one_path_preserves_the_remaining_claimed_paths(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let hub = open_hub(dir.path(), "test-hub", "desktop")?;
    let repo = tempdir()?;
    std::fs::write(repo.path().join("a.rs"), "// a")?;
    std::fs::write(repo.path().join("b.rs"), "// b")?;
    let lane: LaneId = "desktop".parse()?;
    let caller = caller("wt", "branch")?;
    let repo_root = CoordinationRepoRoot::parse(repo.path())?;
    claim_all(
        &hub,
        ClaimRequestArgs {
            repo_root: &repo_root,
            lane: &lane,
            owns: &[
                ClaimPath::from_static("a.rs")?,
                ClaimPath::from_static("b.rs")?,
            ],
            caller: &caller,
            reason: None,
        },
    )?;
    release(
        &hub,
        &lane,
        &[ClaimPath::from_static("a.rs")?],
        &caller,
        None,
    )?;

    let events = enforcer_coordination::sync::stream::read_all_streams(hub.root.as_path())?;
    let claims = active_claims(&events.events);
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].paths, vec![ClaimPath::parse("b.rs")?]);
    Ok(())
}

#[test]
fn hub_event_response_round_trips_the_public_wire_contract(
) -> Result<(), Box<dyn std::error::Error>> {
    let wire = serde_json::json!({
        "id": "evt_public_round_trip",
        "schema": 1,
        "hub": "test-hub",
        "nodeId": "node-test",
        "nodeName": "Test Node",
        "lane": "primary",
        "writer": "node-test.primary",
        "type": "claim",
        "ts": "2026-07-19T00:00:00.000Z",
        "seq": 1,
        "prevEventId": null,
        "prevHash": null,
        "hash": "sha256:public-round-trip",
        "paths": ["crates/enforcer-coordination/src/lib.rs"]
    });

    let response: HubEventResponse = serde_json::from_value(wire.clone())?;
    let decoded: HubEventResponse = serde_json::from_value(serde_json::to_value(&response)?)?;
    assert_eq!(decoded, response);
    assert_eq!(serde_json::to_value(decoded)?, wire);
    Ok(())
}
