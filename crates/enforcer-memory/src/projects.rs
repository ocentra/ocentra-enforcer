//! X06.P1: the project registry -- `list_projects`/`delete_project`/
//! `index_status` over the X06.1 [`crate::store::Store`] layout (one
//! store directory per project under a caller-supplied `stores_dir`),
//! matching the codebase-memory-mcp parity baseline's `list_projects`/
//! `delete_project`/`index_status` tools (scout digest §1, rows 9-11).
//!
//! [`crate::store::Store`] itself only knows how to open/init exactly
//! one project at a time (see its module docs on the "no ghost project
//! database" contract) -- it has no method to enumerate every project
//! under a `stores_dir`, and no delete method at all. This module adds
//! that multi-project layer on top, reading only the `store.json` marker
//! [`crate::store::Store::init`] already writes (never opening a
//! project's logs/SQLite just to list it).
//!
//! # Delete safety (path-traversal rejection)
//!
//! [`delete_project`] resolves the caller-supplied `project_id` to
//! `stores_dir.join(project_id)`, then refuses to delete anything unless
//! the *canonicalized* resolved path is still inside the *canonicalized*
//! `stores_dir` -- a `project_id` containing `..` segments (or an
//! absolute path masquerading as one) can never escape `stores_dir` to
//! delete an arbitrary directory elsewhere on disk. It also refuses to
//! delete a directory that is not a real, marker-bearing store (the same
//! "no ghost project" honesty [`crate::store::Store::open`] enforces for
//! reads applies here for deletes too).
//!
//! # Scope note: this is the library layer, not the MCP wire shape
//!
//! `docs/plans/enforcer-selfhost-plan/refs/x06-baseline-tool-schemas.md`
//! (once its §9-12 follow-up pass lands) and the orchestrator's
//! subsequent verified-shape directive describe `list_projects`/
//! `index_status`/`delete_project` as MCP tool *responses* -- JSON field
//! names, project-argument aliases (`project`/`project_name`/
//! `project_id`/`projectName`), and per-project git metadata
//! (`is_git`/`branch`/`head_sha`/etc.). That wire-shaping is X06.7's
//! (`src/mcp.rs`/`src/cli.rs`, out of this lane's file claims) job of
//! wrapping this module's plain Rust return types -- this module stays
//! the library layer those handlers call into, per this lane's mission
//! ("MCP/CLI wrapper is another lane"). Two library-level facts from
//! that directive DO belong here and are applied below:
//!
//! - **deterministic ordering**: the baseline's own project enumeration
//!   order is unspecified (raw directory-listing order); this module
//!   sorts by project id, a documented enforcer improvement over an
//!   unspecified baseline, not a divergence to reconcile.
//! - **`nodes`/`edges`/status enrichment**: [`index_status`] reports the
//!   operational graph's node/edge counts and a `ready`/`empty` derived
//!   status (`edges`/`nodes` from [`crate::store::sqlite::OperationalGraph`],
//!   already this crate's public API for that count) alongside this
//!   module's own log-staleness summary -- the staleness summary is this
//!   module's original, still-kept contribution; the ready/empty status
//!   is the baseline-aligned addition.
//!
//! Per-project git metadata (`is_git`/`branch`/`head_sha`/worktree
//! detection) is explicitly NOT added here: producing it correctly
//! requires new `git2` traversal logic belonging to `src/git.rs`
//! internals, which this lane's file claims exclude ("code_graph/
//! parsers/languages/git internals" is listed as untouchable). X06.7
//! (or a future `git.rs` accessor added by its own owning lane) is where
//! that enrichment belongs.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::MemoryError;
use crate::store::manifest::check_index_freshness;

const STORE_MARKER_FILE: &str = "store.json";

/// The subset of `store.json` this module reads back out (mirrors
/// `crate::store::StoreMarker`, which is private to `store::mod` -- this
/// is a deliberately separate, read-only projection rather than a
/// visibility change to that module, per this lane's "consume Store API,
/// smallest additive accessor" scope). Field names match
/// `crate::store::StoreMarker`'s exactly (plain snake_case on the wire --
/// that struct has no `#[serde(rename_all)]`), so this projection reads
/// the same `store.json` byte-for-byte without a separate wire format.
#[derive(Debug, Clone, Deserialize)]
struct StoreMarker {
    project_id: String,
    repo_root: String,
    initialized_at: String,
}

/// One project's registry entry, as returned by [`list_projects`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSummary {
    pub project_id: String,
    pub repo_root: String,
    pub initialized_at: String,
    pub store_root: PathBuf,
}

/// Per-log staleness detail inside an [`IndexStatusSummary`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogIndexStatus {
    pub log_name: String,
    pub log_length: u64,
    pub state: FreshnessState,
}

/// Whether a log's derived index (if any) is safe to trust for reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreshnessState {
    /// No index manifest has been written for this log yet -- there is
    /// nothing to be stale relative to (matches
    /// [`crate::store::manifest::check_index_freshness`]'s `Ok(None)`
    /// case).
    NoIndexBuilt,
    /// An index manifest exists and its recorded high-watermark matches
    /// (or exceeds -- cannot happen by construction, but tolerated) the
    /// log's current length.
    Fresh { built_at: String, watermark: u64 },
    /// An index manifest exists but is behind the log's current length.
    Stale { watermark: u64, log_length: u64 },
}

/// Baseline-aligned coarse status (module docs, "scope note"): `Ready`
/// when the operational graph has at least one node, `Empty` otherwise
/// -- literally `nodes > 0` on the baseline, reproduced the same way
/// here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectStatus {
    Ready,
    Empty,
}

/// The staleness summary for one project, as returned by
/// [`index_status`]. `nodes`/`edges`/`status` are the baseline-aligned
/// fields (module docs, "scope note"); `logs` is this module's own
/// staleness-summary contribution, kept as an extension alongside them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexStatusSummary {
    pub project_id: String,
    pub nodes: u64,
    pub edges: u64,
    pub status: ProjectStatus,
    pub logs: Vec<LogIndexStatus>,
}

/// Errors specific to this module, layered on [`crate::error::MemoryError`]
/// via [`ProjectsError::Memory`] for anything that is already one of that
/// enum's cases (log open failures, JSON codec failures, I/O).
#[derive(Debug, thiserror::Error)]
pub enum ProjectsError {
    #[error(transparent)]
    Memory(#[from] MemoryError),

    /// `list_projects`/`index_status`/`delete_project` was asked about a
    /// `project_id` with no `store.json` marker under `stores_dir` --
    /// same "no ghost project" honesty as [`MemoryError::UnknownProject`],
    /// kept as its own variant here because the caller-facing identity is
    /// a project id string, not yet a resolved store root.
    #[error("no project {project_id:?} found under {stores_dir:?}")]
    UnknownProject {
        project_id: String,
        stores_dir: PathBuf,
    },

    /// `delete_project`'s path-containment check failed: the resolved
    /// project directory is not inside `stores_dir`. Fail-closed --
    /// never deletes anything in this case.
    #[error("refusing to delete {resolved:?}: it is not inside stores_dir {stores_dir:?}")]
    PathTraversal {
        resolved: PathBuf,
        stores_dir: PathBuf,
    },
}

/// This module's `Result` alias.
pub type ProjectsResult<T> = std::result::Result<T, ProjectsError>;

/// Enumerate every initialized project under `stores_dir`: each
/// subdirectory carrying a `store.json` marker becomes one
/// [`ProjectSummary`]. A subdirectory without the marker (an incidental
/// empty directory, a partially-cleaned-up delete, scratch space) is
/// silently excluded -- it is not a project, by the same definition
/// [`crate::store::Store::open`] uses. Returns an empty vec (not an
/// error) if `stores_dir` does not exist yet or has no projects.
pub fn list_projects(stores_dir: &Path) -> ProjectsResult<Vec<ProjectSummary>> {
    if !stores_dir.exists() {
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();
    let entries = std::fs::read_dir(stores_dir).map_err(|source| {
        ProjectsError::Memory(MemoryError::Io {
            path: stores_dir.to_path_buf(),
            source,
        })
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| {
            ProjectsError::Memory(MemoryError::Io {
                path: stores_dir.to_path_buf(),
                source,
            })
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Some(marker) = read_marker(&path)? {
            projects.push(ProjectSummary {
                project_id: marker.project_id,
                repo_root: marker.repo_root,
                initialized_at: marker.initialized_at,
                store_root: path,
            });
        }
    }

    // Deterministic ordering regardless of the OS's directory-listing
    // order.
    projects.sort_by(|a, b| a.project_id.cmp(&b.project_id));
    Ok(projects)
}

/// Read `<project_root>/store.json`, returning `None` (not an error) if
/// the marker does not exist -- this is how a non-project directory is
/// distinguished from a real one throughout this module.
fn read_marker(project_root: &Path) -> ProjectsResult<Option<StoreMarker>> {
    let marker_path = project_root.join(STORE_MARKER_FILE);
    if !marker_path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&marker_path).map_err(|source| {
        ProjectsError::Memory(MemoryError::Io {
            path: marker_path.clone(),
            source,
        })
    })?;
    let marker: StoreMarker =
        serde_json::from_str(&raw).map_err(|source| ProjectsError::Memory(MemoryError::Json(source)))?;
    Ok(Some(marker))
}

/// Report index staleness for the project identified by `project_id`
/// under `stores_dir`, for each of the store's two append-only logs
/// (`observations.ndjson`, `graph-events.ndjson`). For each log this
/// checks a conventional `<log-name>.index-manifest.json` sidecar via
/// [`check_index_freshness`] -- if no such manifest has ever been
/// written, the log's status is [`FreshnessState::NoIndexBuilt`] (never
/// an error: an unindexed-but-otherwise-healthy project is a normal
/// state, not a fault). `nodes`/`edges`/`status` (module docs, "scope
/// note") come from opening the project's operational SQLite graph
/// read-only via [`crate::store::sqlite::OperationalGraph::open`] -- a
/// project whose SQLite file does not exist yet (never indexed) reports
/// `nodes: 0, edges: 0, status: Empty` rather than an error, matching
/// the baseline's `nodes>0 ? ready : empty` derivation exactly.
pub fn index_status(stores_dir: &Path, project_id: &str) -> ProjectsResult<IndexStatusSummary> {
    let store_root = resolve_existing_project(stores_dir, project_id)?;

    let sqlite_path = store_root.join("operational.sqlite3");
    let (nodes, edges) = if sqlite_path.exists() {
        let graph = crate::store::sqlite::OperationalGraph::open(&sqlite_path)
            .map_err(ProjectsError::Memory)?;
        (
            graph.node_count().map_err(ProjectsError::Memory)?,
            graph.edge_count().map_err(ProjectsError::Memory)?,
        )
    } else {
        (0, 0)
    };
    let status = if nodes > 0 {
        ProjectStatus::Ready
    } else {
        ProjectStatus::Empty
    };

    let mut logs = Vec::new();
    for log_name in ["observations", "graph-events"] {
        let log_path = store_root.join(format!("{log_name}.ndjson"));
        let log_length = ndjson_line_count(&log_path)?;

        let manifest_path = store_root.join(format!("{log_name}.index-manifest.json"));
        let state = match check_index_freshness(&manifest_path, log_length) {
            Ok(None) => FreshnessState::NoIndexBuilt,
            Ok(Some(manifest)) => FreshnessState::Fresh {
                built_at: manifest.built_at,
                watermark: manifest.source_high_watermark,
            },
            Err(MemoryError::StaleIndex {
                manifest_watermark,
                log_length,
                ..
            }) => FreshnessState::Stale {
                watermark: manifest_watermark,
                log_length,
            },
            Err(other) => return Err(ProjectsError::Memory(other)),
        };

        logs.push(LogIndexStatus {
            log_name: log_name.to_owned(),
            log_length,
            state,
        });
    }

    Ok(IndexStatusSummary {
        project_id: project_id.to_owned(),
        nodes,
        edges,
        status,
        logs,
    })
}

fn ndjson_line_count(path: &Path) -> ProjectsResult<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let content = std::fs::read_to_string(path).map_err(|source| {
        ProjectsError::Memory(MemoryError::Io {
            path: path.to_path_buf(),
            source,
        })
    })?;
    Ok(content.lines().filter(|line| !line.trim().is_empty()).count() as u64)
}

/// Delete project `project_id`'s store directory under `stores_dir`,
/// removing ONLY the derived store -- never a path outside `stores_dir`
/// (see module docs' path-traversal rejection), and never a directory
/// that is not a real, marker-bearing store.
pub fn delete_project(stores_dir: &Path, project_id: &str) -> ProjectsResult<()> {
    let store_root = resolve_existing_project(stores_dir, project_id)?;

    let canonical_stores_dir = std::fs::canonicalize(stores_dir).map_err(|source| {
        ProjectsError::Memory(MemoryError::Io {
            path: stores_dir.to_path_buf(),
            source,
        })
    })?;
    let canonical_store_root = std::fs::canonicalize(&store_root).map_err(|source| {
        ProjectsError::Memory(MemoryError::Io {
            path: store_root.clone(),
            source,
        })
    })?;

    if !canonical_store_root.starts_with(&canonical_stores_dir) {
        return Err(ProjectsError::PathTraversal {
            resolved: canonical_store_root,
            stores_dir: canonical_stores_dir,
        });
    }
    // Refuse to ever delete stores_dir itself (a project_id of "" or
    // "." would otherwise resolve straight back to it).
    if canonical_store_root == canonical_stores_dir {
        return Err(ProjectsError::PathTraversal {
            resolved: canonical_store_root,
            stores_dir: canonical_stores_dir,
        });
    }

    std::fs::remove_dir_all(&store_root).map_err(|source| {
        ProjectsError::Memory(MemoryError::Io {
            path: store_root,
            source,
        })
    })
}

/// Resolve `project_id` to its store directory, verifying a `store.json`
/// marker is actually present -- shared by [`index_status`] and
/// [`delete_project`] so both fail closed on an unknown project id in
/// exactly the same way [`list_projects`] would have omitted it.
fn resolve_existing_project(stores_dir: &Path, project_id: &str) -> ProjectsResult<PathBuf> {
    let candidate = stores_dir.join(project_id);
    let marker = read_marker(&candidate)?;
    if marker.is_none() {
        return Err(ProjectsError::UnknownProject {
            project_id: project_id.to_owned(),
            stores_dir: stores_dir.to_path_buf(),
        });
    }
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let unique = format!(
            "enforcer-memory-projects-{}-{}-{name}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        );
        std::env::temp_dir().join(unique)
    }

    fn init_repo_root(raw: &str) -> enforcer_domain::paths::RepoRoot {
        raw.parse().unwrap_or_else(|_| {
            // Every literal used by this test module is a well-formed
            // Windows/POSIX absolute path; a parse failure here is a
            // test-authoring bug, not a runtime case to handle.
            unreachable!("test literal {raw:?} must parse as a RepoRoot")
        })
    }

    #[test]
    fn list_projects_returns_empty_for_a_missing_stores_dir() -> ProjectsResult<()> {
        let stores_dir = temp_dir("missing");
        let projects = list_projects(&stores_dir)?;
        assert!(projects.is_empty());
        Ok(())
    }

    #[test]
    fn list_projects_reports_every_initialized_project_and_skips_non_projects() -> ProjectsResult<()> {
        let stores_dir = temp_dir("list");
        let root_a = init_repo_root("C:/Projects/alpha");
        let root_b = init_repo_root("C:/Projects/beta");

        Store::init(&stores_dir, &root_a, "2026-07-05T00:00:00Z")?;
        Store::init(&stores_dir, &root_b, "2026-07-05T00:00:01Z")?;

        // An incidental non-project directory (no store.json) must be
        // skipped, not reported and not erroring the whole call.
        std::fs::create_dir_all(stores_dir.join("not-a-project"))
            .map_err(|source| MemoryError::Io { path: stores_dir.clone(), source })?;

        let projects = list_projects(&stores_dir)?;
        assert_eq!(projects.len(), 2);
        let repo_roots: Vec<&str> = projects.iter().map(|p| p.repo_root.as_str()).collect();
        assert!(repo_roots.contains(&"C:/Projects/alpha"));
        assert!(repo_roots.contains(&"C:/Projects/beta"));

        std::fs::remove_dir_all(&stores_dir)
            .map_err(|source| MemoryError::Io { path: stores_dir, source })?;
        Ok(())
    }

    #[test]
    fn index_status_reports_no_index_built_when_no_manifest_exists() -> ProjectsResult<()> {
        let stores_dir = temp_dir("status-fresh");
        let root = init_repo_root("C:/Projects/gamma");
        let store = Store::init(&stores_dir, &root, "2026-07-05T00:00:00Z")?;
        let project_id = store.project_id().as_str().to_owned();

        let status = index_status(&stores_dir, &project_id)?;
        assert_eq!(status.project_id, project_id);
        assert_eq!(status.logs.len(), 2);
        assert!(status
            .logs
            .iter()
            .all(|l| matches!(l.state, FreshnessState::NoIndexBuilt)));
        // A brand-new project has no SQLite file yet -- baseline-aligned
        // (nodes>0 ? ready : empty) must report Empty, not error.
        assert_eq!(status.nodes, 0);
        assert_eq!(status.edges, 0);
        assert_eq!(status.status, ProjectStatus::Empty);

        std::fs::remove_dir_all(&stores_dir)
            .map_err(|source| MemoryError::Io { path: stores_dir, source })?;
        Ok(())
    }

    #[test]
    fn index_status_reports_ready_once_the_operational_graph_has_nodes() -> ProjectsResult<()> {
        let stores_dir = temp_dir("status-ready");
        let root = init_repo_root("C:/Projects/zeta");
        let store = Store::init(&stores_dir, &root, "2026-07-05T00:00:00Z")?;
        let project_id = store.project_id().as_str().to_owned();
        let sqlite_path = store.sqlite_path();
        drop(store);

        {
            let mut graph = crate::store::sqlite::OperationalGraph::open(&sqlite_path)
                .map_err(ProjectsError::Memory)?;
            graph
                .apply(&crate::schema::GraphEventLogEntry {
                    schema_version: crate::schema::SCHEMA_VERSION,
                    seq: 0,
                    id: "evt-0000".to_owned(),
                    event: crate::schema::GraphEventKind::NodeAdded {
                        node_id: "file:lib.rs".to_owned(),
                        node_kind: "file".to_owned(),
                    },
                    ts: "2026-07-05T00:00:00Z".to_owned(),
                    supersedes_seq: None,
                })
                .map_err(ProjectsError::Memory)?;
        }

        let status = index_status(&stores_dir, &project_id)?;
        assert_eq!(status.nodes, 1);
        assert_eq!(status.status, ProjectStatus::Ready);

        std::fs::remove_dir_all(&stores_dir)
            .map_err(|source| MemoryError::Io { path: stores_dir, source })?;
        Ok(())
    }

    #[test]
    fn index_status_detects_a_stale_index_after_the_log_grows() -> ProjectsResult<()> {
        let stores_dir = temp_dir("status-stale");
        let root = init_repo_root("C:/Projects/delta");
        let mut store = Store::init(&stores_dir, &root, "2026-07-05T00:00:00Z")?;
        let project_id = store.project_id().as_str().to_owned();
        let store_root = store.root().to_path_buf();

        // Append one entry so the log length advances past a manifest
        // that will be written for length 0.
        store
            .observation_log_mut()
            .append_with_seq(|seq| crate::schema::ObservationLogEntry {
                schema_version: crate::schema::SCHEMA_VERSION,
                seq,
                id: "obs-test-0000".to_owned(),
                lesson_id: "L1".to_owned(),
                rule_id: None,
                fault_class: None,
                repo_context: "crates/enforcer-memory".to_owned(),
                clean: true,
                source_surface: "test".to_owned(),
                ts: "2026-07-05T00:00:00Z".to_owned(),
                supersedes_seq: None,
            })
            .map_err(ProjectsError::Memory)?;

        crate::store::manifest::write_index_manifest(
            &store_root.join("observations.index-manifest.json"),
            "observations",
            0,
            "2026-07-05T00:00:00Z",
        )
        .map_err(ProjectsError::Memory)?;

        let status = index_status(&stores_dir, &project_id)?;
        let observations = status
            .logs
            .iter()
            .find(|l| l.log_name == "observations")
            .expect("observations log status present");
        assert!(matches!(observations.state, FreshnessState::Stale { .. }));
        assert_eq!(observations.log_length, 1);

        std::fs::remove_dir_all(&stores_dir)
            .map_err(|source| MemoryError::Io { path: stores_dir, source })?;
        Ok(())
    }

    #[test]
    fn delete_project_removes_only_the_derived_store_directory() -> ProjectsResult<()> {
        let stores_dir = temp_dir("delete-happy");
        let root = init_repo_root("C:/Projects/epsilon");
        let store = Store::init(&stores_dir, &root, "2026-07-05T00:00:00Z")?;
        let project_id = store.project_id().as_str().to_owned();
        let store_root = store.root().to_path_buf();
        drop(store);

        assert!(store_root.exists());
        delete_project(&stores_dir, &project_id)?;
        assert!(!store_root.exists(), "the project's own directory must be gone");
        assert!(stores_dir.exists(), "stores_dir itself must survive");

        std::fs::remove_dir_all(&stores_dir)
            .map_err(|source| MemoryError::Io { path: stores_dir, source })?;
        Ok(())
    }

    #[test]
    fn delete_project_rejects_an_unknown_project_id() -> ProjectsResult<()> {
        let stores_dir = temp_dir("delete-unknown");
        std::fs::create_dir_all(&stores_dir)
            .map_err(|source| MemoryError::Io { path: stores_dir.clone(), source })?;

        let outcome = delete_project(&stores_dir, "never-initialized");
        assert!(matches!(outcome, Err(ProjectsError::UnknownProject { .. })));

        std::fs::remove_dir_all(&stores_dir)
            .map_err(|source| MemoryError::Io { path: stores_dir, source })?;
        Ok(())
    }

    #[test]
    fn delete_project_rejects_path_traversal_via_dotdot_project_id() -> ProjectsResult<()> {
        let parent = temp_dir("traversal-parent");
        let stores_dir = parent.join("stores");
        std::fs::create_dir_all(&stores_dir)
            .map_err(|source| MemoryError::Io { path: stores_dir.clone(), source })?;

        // Plant a directory OUTSIDE stores_dir that a `..`-laden
        // project_id could reach, and give it a store.json marker so it
        // would pass the "is this a real store" check if the
        // containment check were missing or buggy.
        let escape_target = parent.join("victim");
        std::fs::create_dir_all(&escape_target)
            .map_err(|source| MemoryError::Io { path: escape_target.clone(), source })?;
        std::fs::write(
            escape_target.join(STORE_MARKER_FILE),
            r#"{"schema_version":1,"project_id":"victim","repo_root":"C:/victim","initialized_at":"2026-07-05T00:00:00Z"}"#,
        )
        .map_err(|source| MemoryError::Io { path: escape_target.clone(), source })?;

        let traversal_id = "../victim";
        let outcome = delete_project(&stores_dir, traversal_id);
        assert!(
            matches!(outcome, Err(ProjectsError::PathTraversal { .. })),
            "expected PathTraversal, got {outcome:?}"
        );
        assert!(
            escape_target.exists(),
            "the directory outside stores_dir must survive the rejected delete"
        );

        std::fs::remove_dir_all(&parent)
            .map_err(|source| MemoryError::Io { path: parent, source })?;
        Ok(())
    }

    #[test]
    fn delete_project_rejects_deleting_stores_dir_itself() -> ProjectsResult<()> {
        let stores_dir = temp_dir("delete-self");
        std::fs::create_dir_all(&stores_dir)
            .map_err(|source| MemoryError::Io { path: stores_dir.clone(), source })?;
        std::fs::write(
            stores_dir.join(STORE_MARKER_FILE),
            r#"{"schema_version":1,"project_id":"self","repo_root":"C:/self","initialized_at":"2026-07-05T00:00:00Z"}"#,
        )
        .map_err(|source| MemoryError::Io { path: stores_dir.clone(), source })?;

        let outcome = delete_project(&stores_dir, ".");
        assert!(matches!(outcome, Err(ProjectsError::PathTraversal { .. })));
        assert!(stores_dir.exists());

        std::fs::remove_dir_all(&stores_dir)
            .map_err(|source| MemoryError::Io { path: stores_dir, source })?;
        Ok(())
    }
}
