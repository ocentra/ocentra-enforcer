use enforcer_memory::git::GitMetadata;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn git_diff_path_lookup_uses_delta_iterator_for_old_and_new_paths() {
    let source = include_str!("../src/git.rs");

    assert_eq!(
        source
            .matches("for idx in 0..diff.deltas().count()")
            .count(),
        0
    );
    assert_eq!(source.matches("diff.deltas().any(|delta|").count(), 1);
    assert_eq!(source.matches(".old_file()").count(), 1);
    assert_eq!(source.matches(".new_file()").count(), 1);
}

/// Initialize a throwaway git repo with git CLI (available in every
/// CI/dev environment this workspace targets) so tests exercise real
/// commit history rather than a hand-built libgit2 object graph.
fn init_repo(dir: &Path) -> TestResult {
    run_git(dir, &["init", "--quiet"])?;
    run_git(dir, &["config", "user.email", "test@example.com"])?;
    run_git(dir, &["config", "user.name", "Test"])?;
    Ok(())
}

fn commit_all(dir: &Path, message: &str) -> TestResult {
    run_git(dir, &["add", "-A"])?;
    run_git(dir, &["commit", "--quiet", "-m", message])?;
    Ok(())
}

fn run_git(dir: &Path, args: &[&str]) -> TestResult {
    let status = Command::new("git").args(args).current_dir(dir).status()?;
    if !status.success() {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(())
}

#[test]
fn open_on_non_git_dir_returns_none_not_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let result = GitMetadata::open(dir.path());
    assert!(matches!(result, Ok(None)));
    Ok(())
}

#[test]
fn open_over_a_canonicalized_path_still_finds_the_repo() -> TestResult {
    // Regression test: `Path::canonicalize()` on Windows returns an
    // extended-length path (`\\?\C:\...`), which `git2::Repository::
    // discover` fails to resolve unless the `\\?\` prefix is stripped
    // first. The feature-parity harness's `workspace_root()` calls
    // `canonicalize()`, so `GitMetadata::open` must tolerate it.
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    fs::write(dir.path().join("a.txt"), "one")?;
    commit_all(dir.path(), "first")?;

    let canonical = dir.path().canonicalize()?;
    let meta = GitMetadata::open(&canonical)?.ok_or("expected a repo via canonicalized path")?;
    assert!(meta.head_commit().is_some());
    Ok(())
}

#[test]
fn open_on_git_repo_reports_head_commit() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    fs::write(dir.path().join("a.txt"), "one")?;
    commit_all(dir.path(), "first")?;

    let meta = GitMetadata::open(dir.path())?.ok_or("expected a repo")?;
    assert!(meta.head_commit().is_some());
    Ok(())
}

#[test]
fn open_on_this_repo_linked_worktree_resolves_head_history() -> TestResult {
    // Regression test for the x06-w5-gitpath finding: `C:\Projects\
    // enforcer-rust` (this repo's checkout) is a LINKED git worktree
    // whose main repo (`C:\Projects\ocentra-enforcer`) has a
    // multi-pack-index (MIDX) in its object store. `git2::Revwalk`
    // over a linked worktree + MIDX combination was reported to fail
    // with "object not found" on the very first commit resolution,
    // which would make every GitHistory QA row honestly unrunnable
    // against the real repo (not just tempdir fixtures). This test
    // exercises the real repo path directly -- it is naturally
    // repo-dependent (skips gracefully if this ever runs somewhere
    // that is not a linked worktree, e.g. a plain clone in CI) but on
    // the dev machine this repo lives on, it must pass.
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("expected crates/enforcer-memory to have a workspace root two levels up")?
        .to_path_buf();

    let Some(mut meta) = GitMetadata::open(&repo_root)? else {
        // Not a git repo in this environment -- nothing to regress.
        return Ok(());
    };

    let head = meta
        .head_commit()
        .ok_or("expected HEAD to resolve to a commit in the real repo")?;
    assert_eq!(head.len(), 40);

    // Exercise the revwalk-based path history query, the exact code
    // path the bug report says fails with "object not found" in a
    // linked worktree over a MIDX-backed object store.
    let history = meta.history_for("crates/enforcer-memory/src/git.rs");
    assert!(
        history.last_commit.is_some(),
        "expected git.rs to have at least one commit touching it"
    );
    Ok(())
}

#[test]
fn history_for_tracks_change_count_across_commits() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    fs::write(dir.path().join("a.txt"), "one")?;
    commit_all(dir.path(), "first")?;
    fs::write(dir.path().join("a.txt"), "two")?;
    commit_all(dir.path(), "second")?;
    fs::write(dir.path().join("b.txt"), "untouched-by-a")?;
    commit_all(dir.path(), "third")?;

    let mut meta = GitMetadata::open(dir.path())?.ok_or("expected a repo")?;
    let history = meta.history_for("a.txt");
    assert_eq!(history.change_count, 2);
    assert!(history.last_commit.is_some());

    let untouched = meta.history_for("does-not-exist.txt");
    assert_eq!(untouched.change_count, 0);
    assert_eq!(untouched.last_commit, None);
    Ok(())
}
