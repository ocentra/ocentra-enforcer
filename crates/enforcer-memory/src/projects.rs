//! X06.P1: the project registry -- `list_projects`/`delete_project`/
//! `index_status` over the X06.1 [`crate::store::Store`] layout (one
//! store directory per project under a caller-supplied `stores_dir`),
//! matching the codebase-memory-mcp parity baseline's `list_projects`/
//! `delete_project`/`index_status` tools (scout digest Â§1, rows 9-11).
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
//! (once its Â§9-12 follow-up pass lands) and the orchestrator's
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

use std::path::Path;

use crate::boundary::store::ProjectStoreMarkerDto;

use crate::error::MemoryError;
use crate::owned_boundary::Retained;
use crate::store::manifest::check_index_freshness;
use enforcer_domain::memory_types::{
    FreshnessState, IndexManifestWatermark, MemoryProjectCount, MemoryProjectId,
    MemoryProjectInitializedAt, MemoryProjectLogName, MemoryProjectRepoRoot,
    MemoryProjectStoreRoot, MemoryStorePath, MemoryStoresDirectory, MemoryStoresRoot,
    ProjectStatus,
};

const STORE_MARKER_FILE: &str = "store.json";

/// The subset of `store.json` this module reads back out (mirrors
/// `crate::store::StoreMarkerDto`, which is private to `store::mod` -- this
/// is a deliberately separate, read-only projection rather than a
/// visibility change to that module, per this lane's "consume Store API,
/// smallest additive accessor" scope). Field names match
/// `crate::store::StoreMarkerDto`'s exactly (plain snake_case on the wire --
/// that struct has no `#[serde(rename_all)]`), so this projection reads
/// the same `store.json` byte-for-byte without a separate wire format.
/// One project's registry entry, as returned by [`list_projects`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSummary {
    pub project_id: MemoryProjectId,
    pub repo_root: MemoryProjectRepoRoot,
    pub initialized_at: MemoryProjectInitializedAt,
    pub store_root: MemoryProjectStoreRoot,
}

/// Per-log staleness detail inside an [`IndexStatusSummary`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogIndexStatus {
    pub log_name: MemoryProjectLogName,
    pub log_length: MemoryProjectCount,
    pub state: FreshnessState,
}

/// Whether a log's derived index (if any) is safe to trust for reads.
/// Baseline-aligned coarse status (module docs, "scope note"): `Ready`
/// when the operational graph has at least one node, `Empty` otherwise
/// -- literally `nodes > 0` on the baseline, reproduced the same way
/// here.
/// The staleness summary for one project, as returned by
/// [`index_status`]. `nodes`/`edges`/`status` are the baseline-aligned
/// fields (module docs, "scope note"); `logs` is this module's own
/// staleness-summary contribution, kept as an extension alongside them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexStatusSummary {
    pub project_id: MemoryProjectId,
    pub nodes: MemoryProjectCount,
    pub edges: MemoryProjectCount,
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
        project_id: MemoryProjectId,
        stores_dir: MemoryStoresRoot,
    },

    /// `delete_project`'s path-containment check failed: the resolved
    /// project directory is not inside `stores_dir`. Fail-closed --
    /// never deletes anything in this case.
    #[error("refusing to delete {resolved:?}: it is not inside stores_dir {stores_dir:?}")]
    PathTraversal {
        resolved: MemoryProjectStoreRoot,
        stores_dir: MemoryStoresRoot,
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
            path: stores_dir.to_path_buf().into(),
            source,
        })
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| {
            ProjectsError::Memory(MemoryError::Io {
                path: stores_dir.to_path_buf().into(),
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
                store_root: path.into(),
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
fn read_marker(project_root: &Path) -> ProjectsResult<Option<ProjectStoreMarkerDto>> {
    let marker_path = project_root.join(STORE_MARKER_FILE);
    if !marker_path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&marker_path).map_err(|source| {
        ProjectsError::Memory(MemoryError::Io {
            path: marker_path.retained().into(),
            source,
        })
    })?;
    let marker: ProjectStoreMarkerDto = crate::boundary::json::decode(&raw)
        .map_err(|source| ProjectsError::Memory(MemoryError::Json(source)))?;
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
pub fn index_status<'a>(
    stores_dir: impl Into<MemoryStoresDirectory<'a>>,
    project_id: impl Into<MemoryProjectId>,
) -> ProjectsResult<IndexStatusSummary> {
    let stores_dir = stores_dir.into();
    let project_id = project_id.into();
    let store_root = resolve_existing_project(stores_dir, &project_id)?;

    let sqlite_path = store_root.join("operational.sqlite3");
    let (nodes, edges) = if sqlite_path.exists() {
        let graph = crate::store::sqlite::OperationalGraph::open(&sqlite_path)
            .map_err(ProjectsError::Memory)?;
        (
            graph.node_count().map_err(ProjectsError::Memory)?.get(),
            graph.edge_count().map_err(ProjectsError::Memory)?.get(),
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
        let log_path: MemoryStorePath = store_root.join(format!("{log_name}.ndjson")).into();
        let log_length = ndjson_line_count(&log_path)?;

        let manifest_path = store_root.join(format!("{log_name}.index-manifest.json"));
        let state = match check_index_freshness(&manifest_path, log_length) {
            Ok(None) => FreshnessState::NoIndexBuilt,
            Ok(Some(manifest)) => FreshnessState::Fresh {
                built_at: manifest.built_at.into(),
                watermark: manifest.source_high_watermark.get(),
            },
            Err(MemoryError::StaleIndex {
                manifest_watermark,
                log_length,
                ..
            }) => FreshnessState::Stale {
                watermark: manifest_watermark.get(),
                log_length: log_length.get(),
            },
            Err(other) => return Err(ProjectsError::Memory(other)),
        };

        logs.push(LogIndexStatus {
            log_name: log_name.into(),
            log_length: log_length.get().into(),
            state,
        });
    }

    Ok(IndexStatusSummary {
        project_id,
        nodes: nodes.into(),
        edges: edges.into(),
        status,
        logs,
    })
}

fn ndjson_line_count(path: &MemoryStorePath) -> ProjectsResult<IndexManifestWatermark> {
    if !path.exists() {
        return Ok(0.into());
    }
    let content = std::fs::read_to_string(path).map_err(|source| {
        ProjectsError::Memory(MemoryError::Io {
            path: path.to_path_buf().into(),
            source,
        })
    })?;
    Ok(u64::try_from(
        content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count(),
    )
    .unwrap_or(u64::MAX)
    .into())
}

/// Delete project `project_id`'s store directory under `stores_dir`,
/// removing ONLY the derived store -- never a path outside `stores_dir`
/// (see module docs' path-traversal rejection), and never a directory
/// that is not a real, marker-bearing store.
pub fn delete_project<'a>(
    stores_dir: impl Into<MemoryStoresDirectory<'a>>,
    project_id: impl Into<MemoryProjectId>,
) -> ProjectsResult<()> {
    let stores_dir = stores_dir.into();
    let project_id = project_id.into();
    let store_root = resolve_existing_project(stores_dir, &project_id)?;

    let canonical_stores_dir = std::fs::canonicalize(stores_dir.as_path()).map_err(|source| {
        ProjectsError::Memory(MemoryError::Io {
            path: stores_dir.as_path().to_path_buf().into(),
            source,
        })
    })?;
    let canonical_store_root = std::fs::canonicalize(store_root.as_path()).map_err(|source| {
        ProjectsError::Memory(MemoryError::Io {
            path: store_root.as_path().to_path_buf().into(),
            source,
        })
    })?;

    if !canonical_store_root.starts_with(&canonical_stores_dir) {
        return Err(ProjectsError::PathTraversal {
            resolved: canonical_store_root.into(),
            stores_dir: canonical_stores_dir.into(),
        });
    }
    // Refuse to ever delete stores_dir itself (a project_id of "" or
    // "." would otherwise resolve straight back to it).
    if canonical_store_root == canonical_stores_dir {
        return Err(ProjectsError::PathTraversal {
            resolved: canonical_store_root.into(),
            stores_dir: canonical_stores_dir.into(),
        });
    }

    std::fs::remove_dir_all(store_root.as_path()).map_err(|source| {
        ProjectsError::Memory(MemoryError::Io {
            path: store_root.as_path().to_path_buf().into(),
            source,
        })
    })
}

/// Resolve `project_id` to its store directory, verifying a `store.json`
/// marker is actually present -- shared by [`index_status`] and
/// [`delete_project`] so both fail closed on an unknown project id in
/// exactly the same way [`list_projects`] would have omitted it.
fn resolve_existing_project(
    stores_dir: MemoryStoresDirectory<'_>,
    project_id: &MemoryProjectId,
) -> ProjectsResult<MemoryProjectStoreRoot> {
    let candidate: MemoryProjectStoreRoot = stores_dir.as_path().join(project_id.as_str()).into();
    let marker = read_marker(&candidate)?;
    if marker.is_none() {
        return Err(ProjectsError::UnknownProject {
            project_id: project_id.retained(),
            stores_dir: stores_dir.as_path().to_path_buf().into(),
        });
    }
    Ok(candidate)
}
