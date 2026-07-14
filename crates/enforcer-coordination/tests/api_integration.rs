//! Public API proof for the coordination hub.
//!
//! These tests exercise the crate from outside its implementation module so
//! callers and the Rust API stay aligned without inline implementation tests.

use std::path::Path;

use enforcer_coordination::api::{
    acknowledge_message, claim_all, closeout, init, open, release, send_message, CallerContext,
    ClaimRequestArgs, CloseoutFilters, Hub,
};
use enforcer_coordination::ledger::active_claims;
use enforcer_domain::ids::{HubName, LaneId};
use tempfile::tempdir;

fn caller(worktree: &str, branch: &str) -> CallerContext {
    CallerContext {
        project_id: "test-project".into(),
        worktree_root: worktree.into(),
        branch: branch.into(),
        commit: Some("abc123".into()),
        codex_thread_id: None,
        codex_session_id: None,
    }
}

fn open_hub(root: &Path, hub_name: &str, lane: &str) -> Result<Hub, Box<dyn std::error::Error>> {
    let hub: HubName = hub_name.parse()?;
    let lane_id: LaneId = lane.parse()?;
    init(root, &hub, &lane_id)?;
    Ok(open(root)?)
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
fn claim_uses_explicit_caller_context() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let hub = open_hub(dir.path(), "test-hub", "arc-16")?;
    let repo = tempdir()?;
    std::fs::write(repo.path().join("lib.rs"), "// fixture file")?;
    let lane: LaneId = "arc-16".parse()?;
    let caller_worktree = "C:/Projects/some-other-worktree";
    let outcome = claim_all(
        &hub,
        ClaimRequestArgs {
            repo_root: repo.path(),
            lane: &lane,
            owns: &["lib.rs".to_owned()],
            caller: &caller(caller_worktree, "lane/arc-16"),
            reason: None,
        },
    )?;
    let context = outcome.events[0].context.as_ref().ok_or("context missing")?;
    assert_eq!(context.get("worktreeRoot").and_then(|value| value.as_str()), Some(caller_worktree));
    assert_eq!(context.get("branch").and_then(|value| value.as_str()), Some("lane/arc-16"));
    assert_eq!(context.get("commit").and_then(|value| value.as_str()), Some("abc123"));
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
    let outcome = claim_all(
        &hub,
        ClaimRequestArgs {
            repo_root: repo.path(),
            lane: &lane,
            owns: &["crates/big-crate/**".to_owned()],
            caller: &caller("C:/Projects/wt", "lane/arc-16"),
            reason: None,
        },
    )?;
    assert_eq!(outcome.events.len(), 2);
    Ok(())
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
    for (lane, file) in [(&lane_a, "a.rs"), (&lane_b, "b.rs")] {
        claim_all(&hub, ClaimRequestArgs { repo_root: repo.path(), lane, owns: &[file.to_owned()], caller: &caller("wt", lane.as_str()), reason: None })?;
    }
    let filters = CloseoutFilters { lane: Some(lane_a.clone()), ..Default::default() };
    let events = closeout(&hub, &lane_a, &filters, &caller("wt-a", "br-a"), None)?;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].lane, "lane-a");
    Ok(())
}

#[test]
fn release_and_message_acknowledgement_are_public_operations() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let hub = open_hub(dir.path(), "test-hub", "desktop")?;
    let repo = tempdir()?;
    std::fs::write(repo.path().join("a.rs"), "// a")?;
    let lane: LaneId = "desktop".parse()?;
    claim_all(&hub, ClaimRequestArgs { repo_root: repo.path(), lane: &lane, owns: &["a.rs".to_owned()], caller: &caller("wt", "branch"), reason: None })?;
    let released = release(&hub, &lane, &["a.rs".to_owned()], &caller("wt", "branch"), None)?;
    assert_eq!(released.kind, "release");
    let recipient: LaneId = "reviewer".parse()?;
    let message = send_message(&hub, &lane, &recipient, "Please inspect.", &caller("wt", "branch"))?;
    let acknowledgement = acknowledge_message(&hub, &lane, &message.id, &caller("wt", "branch"))?;
    assert_eq!(acknowledgement.message_id.as_deref(), Some(message.id.as_str()));
    Ok(())
}

#[test]
fn releasing_one_path_preserves_the_remaining_claimed_paths() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let hub = open_hub(dir.path(), "test-hub", "desktop")?;
    let repo = tempdir()?;
    std::fs::write(repo.path().join("a.rs"), "// a")?;
    std::fs::write(repo.path().join("b.rs"), "// b")?;
    let lane: LaneId = "desktop".parse()?;
    let caller = caller("wt", "branch");
    claim_all(&hub, ClaimRequestArgs {
        repo_root: repo.path(),
        lane: &lane,
        owns: &["a.rs".to_owned(), "b.rs".to_owned()],
        caller: &caller,
        reason: None,
    })?;
    release(&hub, &lane, &["a.rs".to_owned()], &caller, None)?;

    let events = enforcer_coordination::sync::stream::read_all_streams(&hub.root)?;
    let claims = active_claims(&events.events);
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].paths, vec!["b.rs".to_owned()]);
    Ok(())
}
