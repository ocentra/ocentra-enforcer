//! The x06.1 store: one directory per project (keyed by
//! [`crate::ids::ProjectId`]) holding the append-only observation and
//! graph-event logs, the SQLite operational read model, the
//! content-addressed artifact manifest, and index manifests.
//!
//! # No ghost project database creation
//!
//! [`Store::open`] on a project directory that has never been
//! initialized (no `store.json` marker) returns
//! [`crate::error::MemoryError::UnknownProject`] rather than silently
//! creating an empty store. Only [`Store::init`] may create a new
//! project's store, and it does so idempotently (safe to call again on
//! an already-initialized project).

pub mod analytics;
pub mod manifest;
pub mod sqlite;

use std::path::{Path, PathBuf};

use enforcer_domain::paths::RepoRoot;

use crate::error::{MemoryError, Result};
use crate::ids::ProjectId;
use crate::log::AppendLog;
use crate::schema::{GraphEventLogEntry, ObservationLogEntry};

/// Marker file name written by [`Store::init`], whose presence is the
/// sole signal that a project's store directory is a real,
/// deliberately-initialized store rather than an incidental empty
/// directory.
const STORE_MARKER_FILE: &str = "store.json";

/// One project's on-disk store root plus its two append-only logs.
pub struct Store {
    root: PathBuf,
    project_id: ProjectId,
    observation_log: AppendLog<ObservationLogEntry>,
    graph_event_log: AppendLog<GraphEventLogEntry>,
}

/// The `store.json` marker contents: minimal, just enough to prove this
/// directory was deliberately initialized and to record which repo root
/// it belongs to (for diagnostics, not for trust -- trust comes from the
/// marker's mere existence plus the project id derived independently
/// from the caller's own repo root at open time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StoreMarker {
    schema_version: u32,
    project_id: String,
    repo_root: String,
    initialized_at: String,
}

impl Store {
    /// Initialize a fresh store for `repo_root` under `stores_dir`,
    /// creating the project directory and its marker if they do not
    /// already exist. Idempotent: calling this again on an
    /// already-initialized project is a no-op that still returns a
    /// usable [`Store`].
    pub fn init(stores_dir: &Path, repo_root: &RepoRoot, now: &str) -> Result<Self> {
        let project_id = ProjectId::from_repo_root(repo_root);
        let root = stores_dir.join(project_id.as_str());
        std::fs::create_dir_all(&root).map_err(|source| MemoryError::Io {
            path: root.clone(),
            source,
        })?;

        let marker_path = root.join(STORE_MARKER_FILE);
        if !marker_path.exists() {
            let marker = StoreMarker {
                schema_version: crate::schema::SCHEMA_VERSION,
                project_id: project_id.as_str().to_owned(),
                repo_root: repo_root.as_str().to_owned(),
                initialized_at: now.to_owned(),
            };
            let json = serde_json::to_string_pretty(&marker)?;
            std::fs::write(&marker_path, json).map_err(|source| MemoryError::Io {
                path: marker_path.clone(),
                source,
            })?;
        }

        Self::open_initialized(root, project_id)
    }

    /// Open an EXISTING store for `repo_root` under `stores_dir`. Fails
    /// with [`MemoryError::UnknownProject`] if the project directory or
    /// its `store.json` marker does not exist -- this is the "no ghost
    /// project database" contract: opening never creates.
    pub fn open(stores_dir: &Path, repo_root: &RepoRoot) -> Result<Self> {
        let project_id = ProjectId::from_repo_root(repo_root);
        let root = stores_dir.join(project_id.as_str());
        let marker_path = root.join(STORE_MARKER_FILE);
        if !marker_path.exists() {
            return Err(MemoryError::UnknownProject { root });
        }
        Self::open_initialized(root, project_id)
    }

    fn open_initialized(root: PathBuf, project_id: ProjectId) -> Result<Self> {
        let observation_log = AppendLog::open(&root.join("observations.ndjson"))?;
        let graph_event_log = AppendLog::open(&root.join("graph-events.ndjson"))?;
        Ok(Self {
            root,
            project_id,
            observation_log,
            graph_event_log,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub fn observation_log_mut(&mut self) -> &mut AppendLog<ObservationLogEntry> {
        &mut self.observation_log
    }

    pub fn graph_event_log_mut(&mut self) -> &mut AppendLog<GraphEventLogEntry> {
        &mut self.graph_event_log
    }

    pub fn observation_log_path(&self) -> PathBuf {
        self.root.join("observations.ndjson")
    }

    pub fn graph_event_log_path(&self) -> PathBuf {
        self.root.join("graph-events.ndjson")
    }

    pub fn sqlite_path(&self) -> PathBuf {
        self.root.join("operational.sqlite3")
    }
}
