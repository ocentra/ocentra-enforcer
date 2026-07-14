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
use std::path::{Path, PathBuf};
use std::process::Command;

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
    /// The repository's working directory (or, for a linked worktree,
    /// this worktree's own working directory -- NOT the main repo's).
    /// Used only by the [`Self::compute_history_via_cli`] fallback,
    /// which shells into `git log` rooted at this directory.
    workdir: Option<PathBuf>,
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
        let normalized = strip_extended_length_prefix(repo_root);
        match Repository::discover(&normalized) {
            Ok(repo) => {
                let workdir = repo.workdir().map(Path::to_path_buf);
                Ok(Some(Self {
                    repo,
                    workdir,
                    cache: HashMap::new(),
                }))
            }
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
        let history = match self.compute_history(rel_path) {
            Ok(history) => history,
            // git2's revwalk is known to fail with `ObjectNotFound` (libgit2
            // error code -3) over a LINKED git worktree whose main repo's
            // object store uses a multi-pack-index (MIDX) -- this is exactly
            // this workspace's own on-disk layout (see module docs). git2
            // stays the primary path everywhere it works (including the
            // tempdir-repo fixtures every other test in this module uses);
            // this is a narrow, documented CLI fallback for that one known
            // gap, not a general replacement.
            Err(_) => self.compute_history_via_cli(rel_path).unwrap_or_default(),
        };
        self.cache.insert(rel_path.to_string(), history.clone());
        history
    }

    /// Fallback for [`Self::compute_history`] that shells into the `git`
    /// CLI instead of walking history through libgit2. Used only when
    /// git2's revwalk errors (observed with a linked worktree over a
    /// multi-pack-index object store); `git log`'s own revision walk does
    /// not share that gap because it goes through git's full odb/midx
    /// resolution rather than libgit2's.
    fn compute_history_via_cli(&self, rel_path: &str) -> Result<PathHistory, std::io::Error> {
        let Some(workdir) = self.workdir.as_deref() else {
            // A bare repository has no working directory to run `git log`
            // relative to; nothing more this fallback can do.
            return Ok(PathHistory::default());
        };

        let output = Command::new("git")
            .args(["log", "--format=%H", "--follow", "--", rel_path])
            .current_dir(workdir)
            .output()?;

        if !output.status.success() {
            return Ok(PathHistory::default());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut shas = stdout.lines().filter(|line| !line.is_empty());
        let last_commit = shas.next().map(str::to_string);
        let change_count = last_commit.is_some() as usize + shas.count();

        Ok(PathHistory {
            last_commit,
            change_count,
        })
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

/// Strip the Windows extended-length path prefix (`\\?\` and the UNC
/// variant `\\?\UNC\`) that [`Path::canonicalize`] adds on Windows.
/// `git2::Repository::discover` fails to resolve repos through that
/// verbatim prefix, so callers that hand it a canonicalized path (e.g.
/// the feature-parity test harness's `workspace_root()`) need this
/// normalization first. On non-Windows platforms `canonicalize` never
/// produces this prefix, so this is a no-op there.
///
/// Pure string/OsStr handling -- no new dependency, and safe to run on
/// a path that was never prefixed in the first place (returned as-is).
fn strip_extended_length_prefix(path: &Path) -> PathBuf {
    const UNC_PREFIX: &str = r"\\?\UNC\";
    const VERBATIM_PREFIX: &str = r"\\?\";

    let Some(text) = path.to_str() else {
        // Not valid UTF-8; extended-length prefixes are always ASCII,
        // so a non-UTF8 path was never prefixed. Hand it through
        // unchanged rather than lossily rewriting it.
        return path.to_path_buf();
    };

    if let Some(rest) = text.strip_prefix(UNC_PREFIX) {
        // `\\?\UNC\server\share\...` -> `\\server\share\...`
        return PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = text.strip_prefix(VERBATIM_PREFIX) {
        return PathBuf::from(rest);
    }
    path.to_path_buf()
}

fn diff_touches_path(diff: &git2::Diff<'_>, rel_path: &str) -> bool {
    let normalized = rel_path.replace('\\', "/");
    diff.deltas().any(|delta| {
        delta
            .old_file()
            .path()
            .map(|path| path.to_string_lossy().replace('\\', "/") == normalized)
            .unwrap_or(false)
            || delta
                .new_file()
                .path()
                .map(|path| path.to_string_lossy().replace('\\', "/") == normalized)
                .unwrap_or(false)
    })
}
