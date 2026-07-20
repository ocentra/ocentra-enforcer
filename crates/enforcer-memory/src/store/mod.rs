//! The x06.1 store: one directory per project (keyed by
//! [`enforcer_domain::memory_types::ProjectId`]) holding the append-only observation and
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

use enforcer_domain::paths::RepoRoot;

use crate::boundary::log_schema::{
    GraphEventLogEntryDto, ModelObservationLogEntryDto, ObservationLogEntryDto,
    ProceduralLogEntryDto, RouteTraceLogEntryDto, SCHEMA_VERSION,
};
use crate::error::{MemoryError, Result};
use crate::ingest::Observation;
use crate::log::{read_verified, AppendLog, ReadOutcome};
use crate::model_observations::ModelRuntimeObservationRecordDto;
use crate::observations::{ProceduralRecord, RouteTrace};
use crate::owned_boundary::Retained;
use enforcer_domain::memory_types::{
    GraphEventKind, MemoryObservationTimestamp, MemoryProjectStoreRoot, MemoryStorePath,
    MemoryStoresDirectory, ProjectId, Seq,
};

/// Marker file name written by [`Store::init`], whose presence is the
/// sole signal that a project's store directory is a real,
/// deliberately-initialized store rather than an incidental empty
/// directory.
const STORE_MARKER_FILE: &str = "store.json";

/// One project's on-disk store root plus its two append-only logs.
pub struct Store {
    root: MemoryProjectStoreRoot,
    project_id: ProjectId,
    observation_log: AppendLog<ObservationLogEntryDto>,
    procedural_log: AppendLog<ProceduralLogEntryDto>,
    route_trace_log: AppendLog<RouteTraceLogEntryDto>,
    model_observation_log: AppendLog<ModelObservationLogEntryDto>,
    graph_event_log: AppendLog<GraphEventLogEntryDto>,
}

impl std::fmt::Debug for Store {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Store")
            .field("root", &self.root)
            .field("project_id", &self.project_id)
            .finish_non_exhaustive()
    }
}

/// The `store.json` marker contents: minimal, just enough to prove this
/// directory was deliberately initialized and to record which repo root
/// it belongs to (for diagnostics, not for trust -- trust comes from the
/// marker's mere existence plus the project id derived independently
/// from the caller's own repo root at open time).
use crate::boundary::store::StoreMarkerDto;

impl Store {
    /// Initialize a fresh store for `repo_root` under `stores_dir`,
    /// creating the project directory and its marker if they do not
    /// already exist. Idempotent: calling this again on an
    /// already-initialized project is a no-op that still returns a
    /// usable [`Store`].
    pub fn init<'a>(
        stores_dir: impl Into<MemoryStoresDirectory<'a>>,
        repo_root: &RepoRoot,
        now: impl Into<MemoryObservationTimestamp>,
    ) -> Result<Self> {
        let stores_dir = stores_dir.into();
        let now = now.into();
        let project_id = ProjectId::from_repo_root(repo_root);
        let root = stores_dir.as_path().join(project_id.as_str());
        std::fs::create_dir_all(&root).map_err(|source| MemoryError::Io {
            path: root.retained().into(),
            source,
        })?;

        let marker_path = root.join(STORE_MARKER_FILE);
        if !marker_path.exists() {
            let marker = StoreMarkerDto {
                schema_version: crate::boundary::log_schema::SCHEMA_VERSION.into(),
                project_id: project_id.as_str().into(),
                repo_root: repo_root.as_str().retained(),
                initialized_at: now.as_str().retained(),
            };
            let json = serde_json::to_string_pretty(&marker)?;
            std::fs::write(&marker_path, json).map_err(|source| MemoryError::Io {
                path: marker_path.retained().into(),
                source,
            })?;
        }

        Self::open_initialized(root.into(), project_id)
    }

    /// Open an EXISTING store for `repo_root` under `stores_dir`. Fails
    /// with [`MemoryError::UnknownProject`] if the project directory or
    /// its `store.json` marker does not exist -- this is the "no ghost
    /// project database" contract: opening never creates.
    pub fn open<'a>(
        stores_dir: impl Into<MemoryStoresDirectory<'a>>,
        repo_root: &RepoRoot,
    ) -> Result<Self> {
        let stores_dir = stores_dir.into();
        let project_id = ProjectId::from_repo_root(repo_root);
        let root = stores_dir.as_path().join(project_id.as_str());
        let marker_path = root.join(STORE_MARKER_FILE);
        if !marker_path.exists() {
            return Err(MemoryError::UnknownProject { root: root.into() });
        }
        Self::open_initialized(root.into(), project_id)
    }

    fn open_initialized(root: MemoryProjectStoreRoot, project_id: ProjectId) -> Result<Self> {
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

    pub fn root(&self) -> &MemoryProjectStoreRoot {
        &self.root
    }

    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub fn observation_log_mut(&mut self) -> &mut AppendLog<ObservationLogEntryDto> {
        &mut self.observation_log
    }

    pub fn procedural_log_mut(&mut self) -> &mut AppendLog<ProceduralLogEntryDto> {
        &mut self.procedural_log
    }

    pub fn route_trace_log_mut(&mut self) -> &mut AppendLog<RouteTraceLogEntryDto> {
        &mut self.route_trace_log
    }

    pub fn model_observation_log_mut(&mut self) -> &mut AppendLog<ModelObservationLogEntryDto> {
        &mut self.model_observation_log
    }

    pub fn graph_event_log_mut(&mut self) -> &mut AppendLog<GraphEventLogEntryDto> {
        &mut self.graph_event_log
    }

    pub fn append_observation_entry(
        &mut self,
        build_entry: impl FnOnce(Seq) -> ObservationLogEntryDto,
    ) -> Result<Seq> {
        self.observation_log.append_with_seq(build_entry)
    }

    pub fn append_graph_event_entry(
        &mut self,
        build_entry: impl FnOnce(Seq) -> GraphEventLogEntryDto,
    ) -> Result<Seq> {
        self.graph_event_log.append_with_seq(build_entry)
    }

    pub fn read_observation_entries(&self) -> Result<ReadOutcome<ObservationLogEntryDto>> {
        read_verified(
            self.observation_log_path().as_path(),
            |entry: &ObservationLogEntryDto| entry.seq,
        )
    }

    pub fn read_procedural_entries(&self) -> Result<ReadOutcome<ProceduralLogEntryDto>> {
        read_verified(
            self.procedural_log_path().as_path(),
            |entry: &ProceduralLogEntryDto| entry.seq,
        )
    }

    pub fn read_route_trace_entries(&self) -> Result<ReadOutcome<RouteTraceLogEntryDto>> {
        read_verified(
            self.route_trace_log_path().as_path(),
            |entry: &RouteTraceLogEntryDto| entry.seq,
        )
    }

    pub fn read_model_observation_entries(
        &self,
    ) -> Result<ReadOutcome<ModelObservationLogEntryDto>> {
        read_verified(
            self.model_observation_log_path().as_path(),
            |entry: &ModelObservationLogEntryDto| entry.seq,
        )
    }

    pub fn read_graph_event_entries(&self) -> Result<ReadOutcome<GraphEventLogEntryDto>> {
        read_verified(
            self.graph_event_log_path().as_path(),
            |entry: &GraphEventLogEntryDto| entry.seq,
        )
    }

    pub fn observation_log_path(&self) -> MemoryStorePath {
        self.root.join("observations.ndjson").into()
    }

    pub fn procedural_log_path(&self) -> MemoryStorePath {
        self.root.join("procedural-observations.ndjson").into()
    }

    pub fn route_trace_log_path(&self) -> MemoryStorePath {
        self.root.join("route-traces.ndjson").into()
    }

    pub fn model_observation_log_path(&self) -> MemoryStorePath {
        self.root.join("model-observations.ndjson").into()
    }

    pub fn graph_event_log_path(&self) -> MemoryStorePath {
        self.root.join("graph-events.ndjson").into()
    }

    pub fn sqlite_path(&self) -> MemoryStorePath {
        self.root.join("operational.sqlite3").into()
    }

    pub fn append_observation(
        &mut self,
        observation: Observation,
    ) -> Result<ObservationLogEntryDto> {
        let seq = self.observation_log.high_watermark();
        let id = format!("obs-{}-{seq:04}", observation.source_surface);
        let entry = ObservationLogEntryDto {
            schema_version: SCHEMA_VERSION,
            seq: seq.into(),
            id: id.into(),
            lesson_id: observation.lesson_id.into(),
            rule_id: observation.rule_id.map(Into::into),
            fault_class: observation.fault_class.map(Into::into),
            repo_context: observation.repo_context.into(),
            clean: observation.clean,
            source_surface: observation.source_surface.into(),
            ts: observation.ts.into(),
            supersedes_seq: None,
            payload_kind: None,
            payload: None,
        };
        self.observation_log.append_with_seq(|next_seq| {
            debug_assert_eq!(next_seq, seq);
            entry.retained()
        })?;
        Ok(entry)
    }

    pub fn append_procedural(&mut self, record: ProceduralRecord) -> Result<ProceduralLogEntryDto> {
        let seq = self.procedural_log.high_watermark();
        let entry = ProceduralLogEntryDto {
            schema_version: SCHEMA_VERSION,
            seq: seq.into(),
            id: record.id.into(),
            lesson_id: record.lesson_id.into(),
            outcome: record.outcome,
            detail: record.detail.into(),
            ts: record.ts.into(),
            supersedes_seq: None,
        };
        self.procedural_log.append_with_seq(|next_seq| {
            debug_assert_eq!(next_seq, seq);
            entry.retained()
        })?;
        Ok(entry)
    }

    pub fn append_route_trace(&mut self, trace: RouteTrace) -> Result<RouteTraceLogEntryDto> {
        let seq = self.route_trace_log.high_watermark();
        let entry = RouteTraceLogEntryDto {
            schema_version: SCHEMA_VERSION,
            seq: seq.into(),
            id: trace.id.into(),
            query: trace.query.into(),
            route: trace.route.into(),
            confidence: trace.confidence.into(),
            ts: trace.ts.into(),
            supersedes_seq: None,
        };
        self.route_trace_log.append_with_seq(|next_seq| {
            debug_assert_eq!(next_seq, seq);
            entry.retained()
        })?;
        Ok(entry)
    }

    pub fn append_model_observation(
        &mut self,
        record: ModelRuntimeObservationRecordDto,
    ) -> Result<ModelObservationLogEntryDto> {
        let seq = self.model_observation_log.high_watermark();
        let entry = ModelObservationLogEntryDto {
            schema_version: enforcer_domain::memory_types::MemoryLogSchemaVersion::try_new(
                record.schema_version,
            )
            .map_err(|source| crate::error::MemoryError::InvalidLogSchemaVersion { source })?,
            seq: seq.into(),
            observed_at: record.observed_at.into(),
            source: record.source.into(),
            run_id: record.run_id.into(),
            candidate: record.candidate,
            supersedes_seq: None,
        };
        self.model_observation_log.append_with_seq(|next_seq| {
            debug_assert_eq!(next_seq, seq);
            entry.retained()
        })?;
        Ok(entry)
    }

    pub fn append_graph_event(
        &mut self,
        event: GraphEventKind,
        ts: impl Into<MemoryObservationTimestamp>,
    ) -> Result<GraphEventLogEntryDto> {
        let seq = self.graph_event_log.high_watermark();
        let entry = GraphEventLogEntryDto {
            schema_version: SCHEMA_VERSION,
            seq: seq.into(),
            id: format!("evt-{seq:04}").into(),
            event,
            ts: ts.into().into(),
            supersedes_seq: None,
        };
        self.graph_event_log.append_with_seq(|next_seq| {
            debug_assert_eq!(next_seq, seq);
            entry.retained()
        })?;
        Ok(entry)
    }
}
