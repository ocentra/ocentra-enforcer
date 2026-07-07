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
use crate::ids::Seq;
use crate::ingest::Observation;
use crate::log::{read_verified, AppendLog, ReadOutcome};
use crate::model_observations::ModelRuntimeObservationRecord;
use crate::observations::{ProceduralOutcome, ProceduralRecord, RouteTrace};
use crate::schema::{
    GraphEventKind, GraphEventLogEntry, ModelObservationLogEntry, ObservationLogEntry,
    ProceduralLogEntry, ProceduralOutcomeWire, RouteTraceLogEntry, SCHEMA_VERSION,
};

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
    procedural_log: AppendLog<ProceduralLogEntry>,
    route_trace_log: AppendLog<RouteTraceLogEntry>,
    model_observation_log: AppendLog<ModelObservationLogEntry>,
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
        let procedural_log = AppendLog::open(&root.join("procedural-observations.ndjson"))?;
        let route_trace_log = AppendLog::open(&root.join("route-traces.ndjson"))?;
        let model_observation_log = AppendLog::open(&root.join("model-observations.ndjson"))?;
        let graph_event_log = AppendLog::open(&root.join("graph-events.ndjson"))?;
        Ok(Self {
            root,
            project_id,
            observation_log,
            procedural_log,
            route_trace_log,
            model_observation_log,
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

    pub fn procedural_log_mut(&mut self) -> &mut AppendLog<ProceduralLogEntry> {
        &mut self.procedural_log
    }

    pub fn route_trace_log_mut(&mut self) -> &mut AppendLog<RouteTraceLogEntry> {
        &mut self.route_trace_log
    }

    pub fn model_observation_log_mut(&mut self) -> &mut AppendLog<ModelObservationLogEntry> {
        &mut self.model_observation_log
    }

    pub fn graph_event_log_mut(&mut self) -> &mut AppendLog<GraphEventLogEntry> {
        &mut self.graph_event_log
    }

    pub fn append_observation_entry(
        &mut self,
        build_entry: impl FnOnce(u64) -> ObservationLogEntry,
    ) -> Result<Seq> {
        self.observation_log.append_with_seq(build_entry)
    }

    pub fn append_graph_event_entry(
        &mut self,
        build_entry: impl FnOnce(u64) -> GraphEventLogEntry,
    ) -> Result<Seq> {
        self.graph_event_log.append_with_seq(build_entry)
    }

    pub fn read_observation_entries(&self) -> Result<ReadOutcome<ObservationLogEntry>> {
        read_verified(
            &self.observation_log_path(),
            |entry: &ObservationLogEntry| entry.seq,
        )
    }

    pub fn read_procedural_entries(&self) -> Result<ReadOutcome<ProceduralLogEntry>> {
        read_verified(&self.procedural_log_path(), |entry: &ProceduralLogEntry| {
            entry.seq
        })
    }

    pub fn read_route_trace_entries(&self) -> Result<ReadOutcome<RouteTraceLogEntry>> {
        read_verified(
            &self.route_trace_log_path(),
            |entry: &RouteTraceLogEntry| entry.seq,
        )
    }

    pub fn read_model_observation_entries(&self) -> Result<ReadOutcome<ModelObservationLogEntry>> {
        read_verified(
            &self.model_observation_log_path(),
            |entry: &ModelObservationLogEntry| entry.seq,
        )
    }

    pub fn read_graph_event_entries(&self) -> Result<ReadOutcome<GraphEventLogEntry>> {
        read_verified(
            &self.graph_event_log_path(),
            |entry: &GraphEventLogEntry| entry.seq,
        )
    }

    pub fn observation_log_path(&self) -> PathBuf {
        self.root.join("observations.ndjson")
    }

    pub fn procedural_log_path(&self) -> PathBuf {
        self.root.join("procedural-observations.ndjson")
    }

    pub fn route_trace_log_path(&self) -> PathBuf {
        self.root.join("route-traces.ndjson")
    }

    pub fn model_observation_log_path(&self) -> PathBuf {
        self.root.join("model-observations.ndjson")
    }

    pub fn graph_event_log_path(&self) -> PathBuf {
        self.root.join("graph-events.ndjson")
    }

    pub fn sqlite_path(&self) -> PathBuf {
        self.root.join("operational.sqlite3")
    }

    pub fn append_observation(&mut self, observation: Observation) -> Result<ObservationLogEntry> {
        let seq = self.observation_log.high_watermark();
        let id = format!("obs-{}-{seq:04}", observation.source_surface);
        let entry = ObservationLogEntry {
            schema_version: SCHEMA_VERSION,
            seq,
            id,
            lesson_id: observation.lesson_id,
            rule_id: observation.rule_id,
            fault_class: observation.fault_class,
            repo_context: observation.repo_context,
            clean: observation.clean,
            source_surface: observation.source_surface,
            ts: observation.ts,
            supersedes_seq: None,
            payload_kind: None,
            payload: None,
        };
        self.observation_log.append_with_seq(|next_seq| {
            debug_assert_eq!(next_seq, seq);
            entry.clone()
        })?;
        Ok(entry)
    }

    pub fn append_procedural(&mut self, record: ProceduralRecord) -> Result<ProceduralLogEntry> {
        let seq = self.procedural_log.high_watermark();
        let entry = ProceduralLogEntry {
            schema_version: SCHEMA_VERSION,
            seq,
            id: record.id,
            lesson_id: record.lesson_id,
            outcome: record.outcome.into(),
            detail: record.detail,
            ts: record.ts,
            supersedes_seq: None,
        };
        self.procedural_log.append_with_seq(|next_seq| {
            debug_assert_eq!(next_seq, seq);
            entry.clone()
        })?;
        Ok(entry)
    }

    pub fn append_route_trace(&mut self, trace: RouteTrace) -> Result<RouteTraceLogEntry> {
        let seq = self.route_trace_log.high_watermark();
        let entry = RouteTraceLogEntry {
            schema_version: SCHEMA_VERSION,
            seq,
            id: trace.id,
            query: trace.query,
            route: trace.route,
            confidence: trace.confidence,
            ts: trace.ts,
            supersedes_seq: None,
        };
        self.route_trace_log.append_with_seq(|next_seq| {
            debug_assert_eq!(next_seq, seq);
            entry.clone()
        })?;
        Ok(entry)
    }

    pub fn append_model_observation(
        &mut self,
        record: ModelRuntimeObservationRecord,
    ) -> Result<ModelObservationLogEntry> {
        let seq = self.model_observation_log.high_watermark();
        let entry = ModelObservationLogEntry {
            schema_version: record.schema_version,
            seq,
            observed_at: record.observed_at,
            source: record.source,
            run_id: record.run_id,
            candidate: record.candidate,
            supersedes_seq: None,
        };
        self.model_observation_log.append_with_seq(|next_seq| {
            debug_assert_eq!(next_seq, seq);
            entry.clone()
        })?;
        Ok(entry)
    }

    pub fn append_graph_event(
        &mut self,
        event: GraphEventKind,
        ts: String,
    ) -> Result<GraphEventLogEntry> {
        let seq = self.graph_event_log.high_watermark();
        let entry = GraphEventLogEntry {
            schema_version: SCHEMA_VERSION,
            seq,
            id: format!("evt-{seq:04}"),
            event,
            ts,
            supersedes_seq: None,
        };
        self.graph_event_log.append_with_seq(|next_seq| {
            debug_assert_eq!(next_seq, seq);
            entry.clone()
        })?;
        Ok(entry)
    }
}

impl From<ProceduralOutcome> for ProceduralOutcomeWire {
    fn from(value: ProceduralOutcome) -> Self {
        match value {
            ProceduralOutcome::RetrievalSuccess => ProceduralOutcomeWire::RetrievalSuccess,
            ProceduralOutcome::RetrievalFailure => ProceduralOutcomeWire::RetrievalFailure,
            ProceduralOutcome::FixSuccess => ProceduralOutcomeWire::FixSuccess,
            ProceduralOutcome::FixFailure => ProceduralOutcomeWire::FixFailure,
        }
    }
}
