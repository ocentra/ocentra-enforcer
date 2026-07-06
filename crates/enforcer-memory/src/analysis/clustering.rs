//! X06.P4: community clustering ("de-facto modules") over
//! [`crate::code_graph::CodeGraph`].
//!
//! Answers the parity-push gap the scout digest flags for
//! `get_architecture` (scout digest §1: "aspects incl. Leiden/Louvain
//! clustering, hotspots, layers, file_tree") -- the baseline's
//! community-detection aspect groups files/symbols into de-facto
//! modules that do not necessarily match the on-disk directory
//! structure [`crate::architecture::build_overview`] already covers.
//!
//! # Algorithm choice
//!
//! Louvain/Leiden are randomized-tie-break, iterative-refinement
//! algorithms whose modularity-optimization step commonly relies on a
//! random node visitation order to escape local optima. This module
//! implements deterministic **label propagation** instead: every node
//! starts in its own singleton cluster (its node id is its label), then
//! nodes are visited in a fixed, stable order (sorted node id --
//! never insertion order, never a `HashMap` iteration order, never an
//! RNG) and each node adopts the label held by the plurality of its
//! neighbors, ties broken by the lexicographically smallest label. This
//! converges to the same partition on every run over the same input
//! graph, which is the pack's explicit determinism requirement ("same
//! input -> same clusters across 2 runs") -- a property true Louvain
//! does not give for free without pinning its own RNG seed and visit
//! order, which would just be a slower way to reach the same
//! determinism contract this module gives directly.
//!
//! Label propagation is a recognized, published community-detection
//! algorithm in its own right (Raghavan/Albert/Kumara 2007) rather than
//! an approximation of Louvain -- it is the right tool here, not a
//! placeholder for a future Louvain implementation.
//!
//! # Why a fresh adjacency, not [`crate::analysis::CodeAdjacency`]
//!
//! `CodeAdjacency`'s petgraph-backed fields are private to
//! `crate::analysis`, and this lane's file claim on `analysis/mod.rs`
//! is limited to the single `pub mod clustering;` wiring line (a
//! sibling parity lane owns the rest of that file's diff). Community
//! detection also does not need `CodeAdjacency`'s typed
//! [`crate::analysis::EdgeKind`]/directed-traversal machinery -- label
//! propagation only needs undirected connectivity -- so this module
//! builds its own minimal undirected adjacency map directly from
//! [`crate::code_graph::CodeGraph`]'s already-public edge accessors
//! (`imports()`, `calls()`, `symbol_nodes()`, `file_nodes()`), using the
//! same best-effort suffix/name resolution approach `CodeAdjacency`
//! documents for import/call edges (harvested-idea-only from that
//! module's own docs, re-expressed here as a standalone, undirected
//! resolver -- no code copied).

use crate::code_graph::CodeGraph;
use std::collections::{BTreeMap, BTreeSet};

/// One detected community: a de-facto module grouping files/symbols
/// that are more densely connected to each other than to the rest of
/// the graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cluster {
    /// Stable cluster id: the lexicographically smallest node id among
    /// the cluster's members (deterministic given the member set --
    /// never an arbitrary counter, so re-running clustering on the same
    /// graph reproduces the same cluster ids).
    pub id: String,
    /// Every node id (file, symbol, or text-only file) in this
    /// cluster, sorted for determinism.
    pub member_node_ids: Vec<String>,
    /// File node ids among the members (subset of `member_node_ids`).
    pub file_ids: Vec<String>,
    /// Symbol node ids among the members (subset of `member_node_ids`).
    pub symbol_ids: Vec<String>,
}

impl Cluster {
    pub fn size(&self) -> usize {
        self.member_node_ids.len()
    }
}

/// One directed inter-cluster edge count: `from_cluster` -> `to_cluster`
/// had `count` resolved edges crossing the cluster boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterClusterEdge {
    pub from_cluster: String,
    pub to_cluster: String,
    pub count: usize,
}

/// The full clustering result: every detected [`Cluster`] plus the
/// inter-cluster edge counts describing how densely the de-facto
/// modules depend on each other.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClusteringResult {
    pub clusters: Vec<Cluster>,
    pub inter_cluster_edges: Vec<InterClusterEdge>,
}

/// A minimal undirected adjacency map: node id -> sorted, deduplicated
/// set of directly connected node ids. Built once from `graph` and
/// reused by both the label-propagation loop and the inter-cluster edge
/// count (which replays the same directed edge list against the
/// converged partition).
struct UndirectedAdjacency {
    /// Every node id known to the graph, sorted (the label-propagation
    /// visitation order and the deterministic tie-break domain).
    node_ids: Vec<String>,
    neighbors: BTreeMap<String, BTreeSet<String>>,
    /// Directed edges as resolved (from, to) pairs, kept separately from
    /// `neighbors` so [`inter_cluster_edges`] can count direction
    /// instead of the symmetrized undirected view label propagation
    /// itself uses.
    directed_edges: Vec<(String, String)>,
}

impl UndirectedAdjacency {
    fn build(graph: &CodeGraph) -> Self {
        let mut node_ids: BTreeSet<String> = BTreeSet::new();
        for node in graph.nodes() {
            node_ids.insert(node.id().to_string());
        }
        let node_ids: Vec<String> = node_ids.into_iter().collect();

        let mut neighbors: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut directed_edges: Vec<(String, String)> = Vec::new();
        let mut add_edge = |from: String, to: String| {
            if from == to {
                return;
            }
            neighbors
                .entry(from.clone())
                .or_default()
                .insert(to.clone());
            neighbors
                .entry(to.clone())
                .or_default()
                .insert(from.clone());
            directed_edges.push((from, to));
        };

        // File -> symbol containment (structural).
        for symbol in graph.symbol_nodes() {
            add_edge(symbol.file_id.clone(), symbol.id.clone());
        }

        // Import edges: same best-effort suffix match `CodeAdjacency`
        // uses, re-derived here over `file_nodes()` directly (see
        // module docs for why this is not shared code).
        let file_paths: Vec<(&str, &str)> = graph
            .file_nodes()
            .map(|f| (f.rel_path.as_str(), f.id.as_str()))
            .collect();
        for import in graph.imports() {
            if let Some(to_id) = resolve_module_path(&import.module_path, &file_paths) {
                add_edge(import.from_file_id.clone(), to_id.to_string());
            }
        }

        // Call edges: exact/suffix name match against symbol names.
        let symbol_names: Vec<(&str, &str)> = graph
            .symbol_nodes()
            .map(|s| (s.name.as_str(), s.id.as_str()))
            .collect();
        for call in graph.calls() {
            if let Some(to_id) = resolve_callee(&call.callee, &symbol_names) {
                add_edge(call.from_file_id.clone(), to_id.to_string());
            }
        }

        Self {
            node_ids,
            neighbors,
            directed_edges,
        }
    }

    fn neighbors_of(&self, node_id: &str) -> impl Iterator<Item = &String> {
        self.neighbors
            .get(node_id)
            .into_iter()
            .flat_map(|set| set.iter())
    }
}

fn resolve_module_path<'a>(
    module_path: &str,
    file_paths: &[(&'a str, &'a str)],
) -> Option<&'a str> {
    let needle = module_path
        .trim_start_matches("./")
        .trim_start_matches("../");
    let last_segment = needle.rsplit(['/', ':', '.']).next().unwrap_or(needle);
    if last_segment.is_empty() {
        return None;
    }
    file_paths
        .iter()
        .find(|(rel_path, _)| {
            let stem = rel_path.rsplit('/').next().unwrap_or(rel_path);
            let stem = stem.split('.').next().unwrap_or(stem);
            stem == last_segment || rel_path.ends_with(last_segment)
        })
        .map(|(_, id)| *id)
}

fn resolve_callee<'a>(callee: &str, symbol_names: &[(&'a str, &'a str)]) -> Option<&'a str> {
    let last_segment = callee.rsplit(['.', ':']).next().unwrap_or(callee);
    symbol_names
        .iter()
        .find(|(name, _)| *name == callee || *name == last_segment)
        .map(|(_, id)| *id)
}

/// Detect communities ("de-facto modules") in `graph` via deterministic
/// label propagation. `max_iterations` bounds the refinement loop (the
/// algorithm can converge before this; a graph that oscillates between
/// two labelings -- rare but possible for perfectly symmetric
/// fixtures -- stops after this many passes rather than looping
/// forever). Returns an empty [`ClusteringResult`] for an empty graph,
/// never panics.
pub fn detect_clusters(graph: &CodeGraph, max_iterations: usize) -> ClusteringResult {
    let adjacency = UndirectedAdjacency::build(graph);
    if adjacency.node_ids.is_empty() {
        return ClusteringResult::default();
    }

    // Every node starts as its own label -- singleton clusters.
    let mut labels: BTreeMap<String, String> = adjacency
        .node_ids
        .iter()
        .map(|id| (id.clone(), id.clone()))
        .collect();

    for _ in 0..max_iterations {
        let mut changed = false;
        // Fixed, stable visitation order: sorted node ids, never
        // insertion/hash order and never randomized -- the determinism
        // contract the hard tests require.
        for node_id in &adjacency.node_ids {
            let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
            for neighbor in adjacency.neighbors_of(node_id) {
                if let Some(label) = labels.get(neighbor) {
                    *counts.entry(label.as_str()).or_insert(0) += 1;
                }
            }
            if counts.is_empty() {
                continue;
            }
            let best_count = counts.values().copied().max().unwrap_or(0);
            // Deterministic tie-break: smallest label string among
            // every label tied for the highest neighbor count.
            // `BTreeMap` iteration is already key-sorted, so the first
            // match at `best_count` is the lexicographically smallest.
            let best_label = counts
                .iter()
                .find(|(_, &count)| count == best_count)
                .map(|(label, _)| (*label).to_string());
            if let Some(best_label) = best_label {
                let current = labels.get(node_id).map(String::as_str).unwrap_or("");
                if best_label != current {
                    labels.insert(node_id.clone(), best_label);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    // Group nodes by their converged label into clusters, then
    // renormalize each cluster's id to the lexicographically smallest
    // member id (label propagation's raw labels are arbitrary member
    // ids that happened to win -- renormalizing makes cluster identity
    // depend only on membership, not on which node's original id the
    // propagation happened to converge to).
    let mut groups: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (node_id, label) in &labels {
        groups
            .entry(label.clone())
            .or_default()
            .insert(node_id.clone());
    }

    let file_id_set: BTreeSet<&str> = graph.file_nodes().map(|f| f.id.as_str()).collect();
    let symbol_id_set: BTreeSet<&str> = graph.symbol_nodes().map(|s| s.id.as_str()).collect();

    let mut clusters: Vec<Cluster> = groups
        .into_values()
        .map(|members| {
            let member_node_ids: Vec<String> = members.into_iter().collect();
            let id = member_node_ids.first().cloned().unwrap_or_default();
            let file_ids = member_node_ids
                .iter()
                .filter(|id| file_id_set.contains(id.as_str()))
                .cloned()
                .collect();
            let symbol_ids = member_node_ids
                .iter()
                .filter(|id| symbol_id_set.contains(id.as_str()))
                .cloned()
                .collect();
            Cluster {
                id,
                member_node_ids,
                file_ids,
                symbol_ids,
            }
        })
        .collect();
    clusters.sort_by(|a, b| a.id.cmp(&b.id));

    let inter_cluster_edges = inter_cluster_edges(&adjacency, &labels, &clusters);

    ClusteringResult {
        clusters,
        inter_cluster_edges,
    }
}

/// Count directed edges crossing cluster boundaries, keyed by the
/// (from, to) cluster id pair. `labels` maps node id -> raw
/// label-propagation label; `clusters` supplies the renormalized
/// cluster id for each raw label via its member set.
fn inter_cluster_edges(
    adjacency: &UndirectedAdjacency,
    labels: &BTreeMap<String, String>,
    clusters: &[Cluster],
) -> Vec<InterClusterEdge> {
    // raw label -> renormalized cluster id.
    let mut cluster_of_node: BTreeMap<&str, &str> = BTreeMap::new();
    for cluster in clusters {
        for member in &cluster.member_node_ids {
            cluster_of_node.insert(member.as_str(), cluster.id.as_str());
        }
    }
    let _ = labels;

    let mut counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (from, to) in &adjacency.directed_edges {
        let (Some(&from_cluster), Some(&to_cluster)) = (
            cluster_of_node.get(from.as_str()),
            cluster_of_node.get(to.as_str()),
        ) else {
            continue;
        };
        if from_cluster == to_cluster {
            continue;
        }
        *counts
            .entry((from_cluster.to_string(), to_cluster.to_string()))
            .or_insert(0) += 1;
    }

    counts
        .into_iter()
        .map(|((from_cluster, to_cluster), count)| InterClusterEdge {
            from_cluster,
            to_cluster,
            count,
        })
        .collect()
}
