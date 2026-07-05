//! Read-only git metadata for the code KG indexer.
//!
//! [`crate::code_graph`] needs three facts per indexing run: the current
//! HEAD commit, the last commit that touched a given path, and how many
//! commits have touched that path (the "change count" the workpack's
//! file-history summary requires). This module wraps `git2` (libgit2
//! bindings) to answer exactly those three questions and nothing else --
//! no write access, no network fetch, no history rewriting.
//!
//! Any repository-shaped directory that is not (yet) a git repository --
//! e.g. a freshly `cargo new`'d fixture directory in a test -- degrades
//! gracefully: [`GitMetadata::open`] returns `Ok(None)` rather than an
//! error, and [`crate::code_graph`] treats a `None` git context as
//! "index the files, but leave commit/history fields empty" rather than
//! failing the whole run.

use git2::Repository;
use std::collections::HashMap;
use std::path::Path;

/// Everything the indexer knows about the git history of one path.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PathHistory {
    /// The most recent commit SHA that touched this path, if any commit
    /// has touched it yet (a brand-new untracked file has none).
    pub last_commit: Option<String>,
    /// How many commits in HEAD's history have touched this path.
    pub change_count: usize,
}

/// Read-only git context for one repository, opened once per indexing
/// run and queried per file.
pub struct GitMetadata {
    repo: Repository,
    /// Cache of path -> history, populated by [`GitMetadata::history_for`]
    /// so a full-repo index does not re-walk history for every file
    /// independently more than once per path.
    cache: HashMap<String, PathHistory>,
}

impl GitMetadata {
    /// Open `repo_root` as a git repository. Returns `Ok(None)` (not an
    /// error) when `repo_root` is not inside a git working tree -- the
    /// indexer must still be able to run over a non-git directory (e.g.
    /// a test fixture) and simply omit git-derived fields.
    pub fn open(repo_root: &Path) -> Result<Option<Self>, git2::Error> {
        match Repository::discover(repo_root) {
            Ok(repo) => Ok(Some(Self {
                repo,
                cache: HashMap::new(),
            })),
            Err(err) if err.code() == git2::ErrorCode::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// The current HEAD commit SHA, if HEAD resolves to a commit (an
    /// empty repository with no commits yet has none).
    pub fn head_commit(&self) -> Option<String> {
        let head = self.repo.head().ok()?;
        let commit = head.peel_to_commit().ok()?;
        Some(commit.id().to_string())
    }

    /// Last commit + change count for `rel_path` (repo-root-relative,
    /// forward-slash-normalized). Walks HEAD's history once per unique
    /// path per [`GitMetadata`] instance; results are cached.
    pub fn history_for(&mut self, rel_path: &str) -> PathHistory {
        if let Some(cached) = self.cache.get(rel_path) {
            return cached.clone();
        }
        let history = self.compute_history(rel_path).unwrap_or_default();
        self.cache.insert(rel_path.to_string(), history.clone());
        history
    }

    fn compute_history(&self, rel_path: &str) -> Result<PathHistory, git2::Error> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let mut last_commit = None;
        let mut change_count = 0usize;

        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let tree = commit.tree()?;
            let touched = match commit.parent(0) {
                Ok(parent) => {
                    let parent_tree = parent.tree()?;
                    let diff =
                        self.repo
                            .diff_tree_to_tree(Some(&parent_tree), Some(&tree), None)?;
                    diff_touches_path(&diff, rel_path)
                }
                // Root commit: every path present in its tree counts as
                // touched by it.
                Err(_) => tree.get_path(Path::new(rel_path)).is_ok(),
            };
            if touched {
                change_count += 1;
                if last_commit.is_none() {
                    last_commit = Some(oid.to_string());
                }
            }
        }

        Ok(PathHistory {
            last_commit,
            change_count,
        })
    }
}

fn diff_touches_path(diff: &git2::Diff<'_>, rel_path: &str) -> bool {
    let mut touched = false;
    let normalized = rel_path.replace('\\', "/");
    // `foreach` cannot capture a mutable outer variable through a
    // fallible closure while also returning `Result` ergonomically, so
    // collect delta paths first, then check membership.
    for idx in 0..diff.deltas().count() {
        if let Some(delta) = diff.get_delta(idx) {
            let matches = delta
                .old_file()
                .path()
                .map(|p| p.to_string_lossy().replace('\\', "/") == normalized)
                .unwrap_or(false)
                || delta
                    .new_file()
                    .path()
                    .map(|p| p.to_string_lossy().replace('\\', "/") == normalized)
                    .unwrap_or(false);
            if matches {
                touched = true;
                break;
            }
        }
    }
    touched
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::fs;
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
}
