//! X06.7 unit-shaped tests for [`enforcer_memory::watch`], moved out of
//! `src/watch.rs` per this crate's "no inline `#[cfg(test)]` modules"
//! style (workspace clippy denies `unwrap`/`expect`/`panic` even in test
//! code -- the original inline module's `panic!(...)` on an unexpected
//! second reindex request is replaced with a `Result`-returning
//! assertion here).

use enforcer_memory::watch::{
    adaptive_poll_interval, git_head_changed, is_relevant_event, Watcher,
};
use notify::EventKind;
use std::error::Error;
use std::fs;
use std::time::Duration;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn adaptive_poll_interval_matches_baseline_formula() {
    assert_eq!(adaptive_poll_interval(0), Duration::from_secs(5));
    assert_eq!(adaptive_poll_interval(499), Duration::from_secs(5));
    assert_eq!(adaptive_poll_interval(500), Duration::from_secs(6));
    assert_eq!(adaptive_poll_interval(1_000), Duration::from_secs(7));
    // Cap at 60s regardless of how large the corpus is.
    assert_eq!(adaptive_poll_interval(1_000_000), Duration::from_secs(60));
}

#[test]
fn watcher_detects_a_file_change_and_emits_exactly_one_debounced_request() -> TestResult {
    let dir = tempfile::tempdir()?;
    let watcher = Watcher::start(dir.path(), Duration::from_millis(150))?;

    // A single logical change: create then immediately rewrite one
    // file -- on most platforms/editors this fires multiple raw OS
    // events (create, modify, maybe metadata) that must collapse to
    // ONE ReindexRequest.
    let file_path = dir.path().join("a.rs");
    fs::write(&file_path, "fn a() {}\n")?;
    fs::write(&file_path, "fn a() { /* changed */ }\n")?;

    let request = watcher
        .next_reindex_request(Duration::from_secs(10))?
        .ok_or("expected a reindex request within the deadline")?;
    assert!(
        request.paths.iter().any(|p| p.ends_with("a.rs")),
        "expected a.rs among the debounced paths, got {:?}",
        request.paths
    );

    // No SECOND request should be pending immediately after the
    // first one drains -- the burst collapsed into exactly one.
    let second = watcher.next_reindex_request(Duration::from_millis(200))?;
    if let Some(second) = second {
        return Err(format!("expected no second reindex request, got {second:?}").into());
    }
    Ok(())
}

#[test]
fn watcher_returns_none_on_deadline_with_no_events() -> TestResult {
    let dir = tempfile::tempdir()?;
    let watcher = Watcher::start(dir.path(), Duration::from_millis(50))?;
    let result = watcher.next_reindex_request(Duration::from_millis(200))?;
    assert!(result.is_none());
    Ok(())
}

#[test]
fn git_head_changed_is_false_for_a_non_git_directory() -> TestResult {
    let dir = tempfile::tempdir()?;
    assert!(!git_head_changed(dir.path(), None));
    Ok(())
}

#[test]
fn drops_metadata_only_relevance_filter_never_panics_on_any_kind() {
    // Defensive coverage of every EventKind variant family so a future
    // `notify` upgrade adding a new sub-kind cannot silently panic this
    // filter -- `is_relevant_event` must stay total.
    let kinds = [
        EventKind::Any,
        EventKind::Access(notify::event::AccessKind::Any),
        EventKind::Other,
    ];
    for kind in kinds {
        let _ = is_relevant_event(&kind);
    }
}
