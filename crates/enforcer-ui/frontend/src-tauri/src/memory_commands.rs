use std::path::{Path, PathBuf};

use enforcer_domain::memory_types::{IndexMode, MemoryPathInput};
use enforcer_domain::paths::RepoRoot;
use enforcer_memory::code_graph::{CodeGraph, IndexOptions, Manifest};
use enforcer_memory::error::MemoryError;
use enforcer_memory::ids::repo_root;
use enforcer_memory::search::search_graph::{search_graph, SearchGraphSpec};
use enforcer_memory::store::{manifest::write_index_manifest, sqlite::OperationalGraph, Store};
use serde::Serialize;

use crate::project_registry::memory_index_available;
use crate::{resolve_pack_root, store_timestamp, walk_repo_files};

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

#[tauri::command]
pub(crate) fn search_memory_graph(
    root: String,
    query: String,
) -> Result<enforcer_ui::memory_explorer::GraphSearchResponse, String> {
    let root = project_root(root)?;
    let root_path = project_path(&root);
    let raw_root = MemoryPathInput::from(root.as_str().to_owned());
    let normalized_root =
        repo_root(&raw_root).map_err(|error| format!("invalid project root: {error}"))?;
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
            query: Some(query.clone().into()),
            limit: Some(32.into()),
            ..Default::default()
        },
    )
    .map_err(|error| format!("memory graph search failed: {error}"))?;
    let response = enforcer_ui::memory_explorer::GraphSearchResponse {
        total: result.total.into(),
        has_more: result.has_more.into(),
        query,
        project_scope: root.as_str().to_owned(),
        results: result
            .results
            .into_iter()
            .map(|hit| enforcer_ui::memory_explorer::GraphSearchHitResponse {
                node_id: hit.node_id.into(),
                name: hit.name.into(),
                qualified_name: hit.qualified_name.into(),
                label: hit.label.as_str().to_owned(),
                file_path: hit.file_path.into(),
                evidence_kind: enforcer_ui::memory_explorer::EvidenceKind::CodeGraph,
                rank: hit.rank.map(|rank| format!("{:.4}", rank.get())),
            })
            .collect(),
    };
    response
        .validate_domain()
        .map_err(|error| format!("memory graph search returned invalid response: {error}"))?;
    Ok(response)
}

#[tauri::command]
pub(crate) fn memory_index_status(root: String) -> Result<MemoryIndexStatusPayload, String> {
    let root = project_root(root)?;
    Ok(MemoryIndexStatusPayload {
        available: memory_index_available(&root),
    })
}

pub(crate) fn create_memory_index_sync(root: String) -> Result<IndexProjectPayload, String> {
    let root = project_root(root)?;
    let root_path = project_path(&root);
    let store_root = root_path.join(".enforce").join("memory");
    let raw_root = MemoryPathInput::from(root.as_str().to_owned());
    let normalized_root =
        repo_root(&raw_root).map_err(|error| format!("invalid project root: {error}"))?;
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

    let files = walk_repo_files(root_path)?;
    let mut graph = CodeGraph::new();
    graph
        .index_repository_with_options(
            root_path,
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
        u64::try_from(entries.entries.len()).unwrap_or(u64::MAX),
        &timestamp,
    )
    .map_err(|error| format!("cannot write memory index manifest: {error}"))?;

    Ok(IndexProjectPayload {
        root: root.as_str().to_owned(),
        files_indexed: files.len(),
        nodes: graph.nodes().len(),
        edges: graph.imports().len() + graph.calls().len() + graph.routes().len(),
    })
}

#[tauri::command]
pub(crate) fn load_memory_summary(
    root: String,
) -> Result<enforcer_ui::memory_explorer::MemoryExplorerResponse, String> {
    let selected_project_root = project_root(root)?;
    Ok(enforcer_ui::memory_explorer::render_memory_explorer(
        enforcer_domain::ui_types::UiRunMode::Human,
        project_path(&selected_project_root),
        &resolve_pack_root()?,
    ))
}

fn project_root(root: String) -> Result<RepoRoot, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("project root is not a directory: {root}"));
    }
    let canonical = root_path
        .canonicalize()
        .map_err(|error| format!("cannot resolve project root: {error}"))?;
    RepoRoot::try_from(canonical.as_path())
        .map_err(|error| format!("invalid project root: {error}"))
}

fn project_path(root: &RepoRoot) -> &Path {
    Path::new(root.as_str())
}
