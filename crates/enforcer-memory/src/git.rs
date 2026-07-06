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
