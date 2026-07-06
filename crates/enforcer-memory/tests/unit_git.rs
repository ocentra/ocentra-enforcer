use enforcer_memory::git::GitMetadata;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

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
