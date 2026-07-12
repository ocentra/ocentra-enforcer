use std::path::{Path, PathBuf};

use enforcer_memory::code_graph::{CodeGraph, IndexMode, IndexOptions, Manifest};
use enforcer_memory::error::MemoryError;
use enforcer_memory::ids::repo_root;
use enforcer_memory::search::search_graph::{search_graph, SearchGraphSpec};
use enforcer_memory::store::{manifest::write_index_manifest, sqlite::OperationalGraph, Store};
use serde::Serialize;

use crate::project_registry::memory_index_available;
use crate::{desktop_workspace_root, store_timestamp, walk_repo_files};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GraphSearchHitPayload {
    node_id: String,
    name: String,
    qualified_name: String,
    label: String,
    file_path: String,
    rank: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GraphSearchPayload {
    total: usize,
    has_more: bool,
    results: Vec<GraphSearchHitPayload>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IndexProjectPayload {
    root: String,
    files_indexed: usize,
    nodes: usize,
    edges: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryIndexStatusPayload {
    available: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RetrievalSummary {
    available: bool,
    status: String,
    rows_total: u64,
    rows_green: u64,
    rows_degraded: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LearningSummary {
    available: bool,
    status: String,
    lessons: u64,
    blockers: u64,
    follow_ups: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelSummary {
    available: bool,
    runtime_mode: String,
    allow_network: bool,
    cache_root: String,
    observations: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParityRowSummary {
    tool: String,
    verdict: String,
    reason: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParitySummary {
    pub(crate) available: bool,
    pub(crate) tools_total: u64,
    pub(crate) equal: u64,
    pub(crate) better: u64,
    pub(crate) worse: u64,
    pub(crate) incomparable: u64,
    pub(crate) unrunnable: u64,
    pub(crate) rows: Vec<ParityRowSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryEvidenceProvenance {
    pub(crate) scope: &'static str,
    pub(crate) selected_project_root: String,
    pub(crate) artifact_root: String,
    pub(crate) generated_at_unix_secs: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemorySummaryPayload {
    pub(crate) provenance: MemoryEvidenceProvenance,
    pub(crate) retrieval: RetrievalSummary,
    pub(crate) learning: LearningSummary,
    pub(crate) models: ModelSummary,
    pub(crate) parity: ParitySummary,
}

#[tauri::command]
pub(crate) fn search_memory_graph(
    root: String,
    query: String,
) -> Result<enforcer_ui::memory_explorer::GraphSearchPayload, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("project root is not a directory: {root}"));
    }
    let normalized_root =
        repo_root(&root).map_err(|error| format!("invalid project root: {error}"))?;
    let store = Store::open(&root_path.join(".enforce").join("memory"), &normalized_root)
        .map_err(|error| format!("no memory projection for this project: {error}"))?;
    let operational = OperationalGraph::open_read_only(&store.sqlite_path())
        .map_err(|error| format!("cannot open memory projection: {error}"))?;
    let nodes = operational
        .nodes_snapshot()
        .map_err(|error| format!("cannot read projected graph nodes: {error}"))?;
    let edges = operational
        .edges_snapshot()
        .map_err(|error| format!("cannot read projected graph edges: {error}"))?;
    let graph = CodeGraph::from_store_projection(&nodes, &edges);
    let result = search_graph(
        &graph,
        &SearchGraphSpec {
            query: Some(query.clone()),
            limit: Some(32),
            ..Default::default()
        },
    )
    .map_err(|error| format!("memory graph search failed: {error}"))?;
    Ok(enforcer_ui::memory_explorer::GraphSearchPayload {
        total: result.total,
        has_more: result.has_more,
        query,
        project_scope: root,
        results: result
            .results
            .into_iter()
            .map(|hit| enforcer_ui::memory_explorer::GraphSearchHitPayload {
                node_id: hit.node_id,
                name: hit.name,
                qualified_name: hit.qualified_name,
                label: hit.label.to_owned(),
                file_path: hit.file_path,
                evidence_kind: enforcer_ui::memory_explorer::EvidenceKind::CodeGraph,
                rank: hit.rank.map(|rank| format!("{rank:.4}")),
            })
            .collect(),
    })
}

#[tauri::command]
pub(crate) fn memory_index_status(root: String) -> Result<MemoryIndexStatusPayload, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("project root is not a directory: {root}"));
    }
    Ok(MemoryIndexStatusPayload {
        available: memory_index_available(&root_path),
    })
}

#[tauri::command]
pub(crate) async fn create_memory_index(root: String) -> Result<IndexProjectPayload, String> {
    tauri::async_runtime::spawn_blocking(move || create_memory_index_sync(root))
        .await
        .map_err(|error| format!("memory index task failed: {error}"))?
}

fn create_memory_index_sync(root: String) -> Result<IndexProjectPayload, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("project root is not a directory: {root}"));
    }
    let store_root = root_path.join(".enforce").join("memory");
    let normalized_root =
        repo_root(&root).map_err(|error| format!("invalid project root: {error}"))?;
    let timestamp = store_timestamp();
    let mut store = match Store::open(&store_root, &normalized_root) {
        Ok(store) => {
            let existing = store
                .read_graph_event_entries()
                .map_err(|error| format!("cannot inspect existing memory projection: {error}"))?;
            if !existing.entries.is_empty() {
                return Err("this project already has a memory projection; incremental refresh is not implemented yet, so Enforcer refuses to append a stale duplicate graph".to_owned());
            }
            store
        }
        Err(MemoryError::UnknownProject { .. }) => {
            Store::init(&store_root, &normalized_root, &timestamp)
                .map_err(|error| format!("cannot initialize memory Store: {error}"))?
        }
        Err(error) => return Err(format!("cannot open memory Store: {error}")),
    };

    let files = walk_repo_files(&root_path)?;
    let mut graph = CodeGraph::new();
    graph
        .index_repository_with_options(
            &root_path,
            &files,
            &Manifest::default(),
            IndexOptions {
                mode: IndexMode::Fast,
                ..IndexOptions::default()
            },
        )
        .map_err(|error| format!("memory index failed: {error}"))?;
    graph
        .append_store_projection_events(&mut store, &timestamp)
        .map_err(|error| format!("cannot persist memory projection: {error}"))?;
    let entries = store
        .read_graph_event_entries()
        .map_err(|error| format!("cannot read persisted memory projection: {error}"))?;
    let sqlite_path = store.sqlite_path();
    let store_path = store.root().to_path_buf();
    drop(store);
    let mut operational = OperationalGraph::open(&sqlite_path)
        .map_err(|error| format!("cannot open memory operational graph: {error}"))?;
    operational
        .rebuild(&entries.entries)
        .map_err(|error| format!("cannot rebuild memory operational graph: {error}"))?;
    write_index_manifest(
        &store_path.join("graph-events.index-manifest.json"),
        "graph-event",
        entries.entries.len() as u64,
        &timestamp,
    )
    .map_err(|error| format!("cannot write memory index manifest: {error}"))?;

    Ok(IndexProjectPayload {
        root,
        files_indexed: files.len(),
        nodes: graph.nodes().len(),
        edges: graph.imports().len() + graph.calls().len() + graph.routes().len(),
    })
}

#[tauri::command]
pub(crate) fn load_memory_summary(
    root: String,
) -> Result<enforcer_ui::memory_explorer::MemoryExplorerPayload, String> {
    let selected_project_root = PathBuf::from(&root);
    if !selected_project_root.is_dir() {
        return Err(format!("project root is not a directory: {root}"));
    }
    Ok(enforcer_ui::memory_explorer::render_memory_explorer(
        enforcer_ui::memory_explorer::RunMode::Human,
        &selected_project_root,
        &desktop_workspace_root(),
    ))
}

fn latest_modified_unix_secs(paths: &[&Path]) -> Option<u64> {
    paths
        .iter()
        .filter_map(|path| {
            std::fs::metadata(path)
                .ok()?
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|duration| duration.as_secs())
        })
        .max()
}

fn read_json(path: &Path) -> Option<serde_json::Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
}

fn string_field(value: Option<&serde_json::Value>, key: &str) -> String {
    value
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn number_field(value: Option<&serde_json::Value>, key: &str) -> u64 {
    value
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default()
}

fn array_len(value: Option<&serde_json::Value>, key: &str) -> u64 {
    value
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_array)
        .map_or(0, |items| items.len() as u64)
}

fn parity_summary(value: Option<&serde_json::Value>) -> ParitySummary {
    let rows = value
        .and_then(|value| value.get("rows"))
        .and_then(serde_json::Value::as_array)
        .map_or_else(Vec::new, |items| {
            items
                .iter()
                .filter_map(|row| {
                    let verdict = row.get("comparison_verdict")?.as_str()?.to_owned();
                    let tool = row.get("tool")?.as_str()?.to_owned();
                    let reason = row
                        .get("better_because")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    Some(ParityRowSummary {
                        tool,
                        verdict,
                        reason,
                    })
                })
                .collect()
        });
    ParitySummary {
        available: value.is_some(),
        tools_total: number_field(value, "tools_total"),
        equal: number_field(value, "tools_equal"),
        better: number_field(value, "tools_better"),
        worse: number_field(value, "tools_worse"),
        incomparable: number_field(value, "tools_incomparable"),
        unrunnable: number_field(value, "tools_unrunnable"),
        rows,
    }
}
