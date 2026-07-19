//! X06.P4: community clustering ("de-facto modules") over
//! [`crate::code_graph::CodeGraph`].
//!
//! Answers the parity-push gap the scout digest flags for
//! `get_architecture` (scout digest Â§1: "aspects incl. Leiden/Louvain
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
//! [`enforcer_domain::memory_types::MemoryEdgeKind`]/directed-traversal machinery -- label
//! propagation only needs undirected connectivity -- so this module
//! builds its own minimal undirected adjacency map directly from
//! [`crate::code_graph::CodeGraph`]'s already-public edge accessors
//! (`imports()`, `calls()`, `symbol_nodes()`, `file_nodes()`), using the
//! same best-effort suffix/name resolution approach `CodeAdjacency`
//! documents for import/call edges (harvested-idea-only from that
//! module's own docs, re-expressed here as a standalone, undirected
//! resolver -- no code copied).

use crate::code_graph::CodeGraph;
use crate::owned_boundary::RetainedDisplay;
use enforcer_domain::memory_types::{
    CodeSearchPath, CodeSearchSymbolName, MemoryClusterFileId, MemoryClusterId,
    MemoryClusterIterationLimit, MemoryClusterNodeId, MemoryClusterSize, MemoryClusterSymbolId,
    MemoryInterClusterEdgeCount, ParserSourceText,
};
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
    pub id: MemoryClusterId,
    /// Every node id (file, symbol, or text-only file) in this
    /// cluster, sorted for determinism.
    pub member_node_ids: Vec<MemoryClusterNodeId>,
    /// File node ids among the members (subset of `member_node_ids`).
    pub file_ids: Vec<MemoryClusterFileId>,
    /// Symbol node ids among the members (subset of `member_node_ids`).
    pub symbol_ids: Vec<MemoryClusterSymbolId>,
}

impl Cluster {
    pub fn size(&self) -> MemoryClusterSize {
        self.member_node_ids.len().into()
    }
}

/// One directed inter-cluster edge count: `from_cluster` -> `to_cluster`
/// had `count` resolved edges crossing the cluster boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterClusterEdge {
    pub from_cluster: MemoryClusterId,
    pub to_cluster: MemoryClusterId,
    pub count: MemoryInterClusterEdgeCount,
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
    node_ids: Vec<MemoryClusterNodeId>,
    neighbors: BTreeMap<MemoryClusterNodeId, BTreeSet<MemoryClusterNodeId>>,
    /// Directed edges as resolved (from, to) pairs, kept separately from
    /// `neighbors` so [`inter_cluster_edges`] can count direction
    /// instead of the symmetrized undirected view label propagation
    /// itself uses.
    directed_edges: Vec<(MemoryClusterNodeId, MemoryClusterNodeId)>,
}

impl UndirectedAdjacency {
    fn build(graph: &CodeGraph) -> Self {
        let mut node_ids: BTreeSet<MemoryClusterNodeId> = BTreeSet::new();
        for node in graph.nodes() {
            node_ids.insert(node.id().retained_display().into());
        }
        let node_ids: Vec<MemoryClusterNodeId> = node_ids.into_iter().collect();

        let mut neighbors: BTreeMap<MemoryClusterNodeId, BTreeSet<MemoryClusterNodeId>> =
            BTreeMap::new();
        let mut directed_edges: Vec<(MemoryClusterNodeId, MemoryClusterNodeId)> = Vec::new();
        let mut add_edge = |from: MemoryClusterNodeId, to: MemoryClusterNodeId| {
            if from == to {
                return;
            }
            neighbors
                // CLONE-JUSTIFICATION: adjacency map owns source key while edge tuple retains source.
                .entry(from.clone())
                .or_default()
                // CLONE-JUSTIFICATION: adjacency set owns target while directed edge retains target.
                .insert(to.clone());
            neighbors
                // CLONE-JUSTIFICATION: reverse adjacency map owns target key while edge tuple retains target.
                .entry(to.clone())
                .or_default()
                // CLONE-JUSTIFICATION: reverse adjacency set owns source while directed edge retains source.
                .insert(from.clone());
            directed_edges.push((from, to));
        };

        // File -> symbol containment (structural).
        for symbol in graph.symbol_nodes() {
            // CLONE-JUSTIFICATION: owned adjacency graph outlives borrowed symbol iterator.
            add_edge(symbol.file_id.as_str().into(), symbol.id.as_str().into());
        }

        // Import edges: same best-effort suffix match `CodeAdjacency`
        // uses, re-derived here over `file_nodes()` directly (see
        // module docs for why this is not shared code).
        let file_paths: Vec<(CodeSearchPath, MemoryClusterFileId)> = graph
            .file_nodes()
            .map(|f| (f.rel_path.as_str().into(), f.id.as_str().into()))
            .collect();
        for import in graph.imports() {
            if let Some(to_id) = resolve_module_path(
                ParserSourceText::from(import.module_path.as_str()),
                &file_paths,
            ) {
                // CLONE-JUSTIFICATION: owned adjacency graph outlives borrowed import iterator.
                add_edge(import.from_file_id.as_str().into(), to_id.as_str().into());
            }
        }

        // Call edges: exact/suffix name match against symbol names.
        let symbol_names: Vec<(CodeSearchSymbolName, MemoryClusterNodeId)> = graph
            .symbol_nodes()
            .map(|s| (s.name.as_str().into(), s.id.as_str().into()))
            .collect();
        for call in graph.calls() {
            if let Some(to_id) =
                resolve_callee(ParserSourceText::from(call.callee.as_str()), &symbol_names)
            {
                // CLONE-JUSTIFICATION: owned adjacency graph outlives borrowed call iterator.
                add_edge(call.from_file_id.as_str().into(), to_id);
            }
        }

        Self {
            node_ids,
            neighbors,
            directed_edges,
        }
    }

    fn neighbors_of(
        &self,
        node_id: &MemoryClusterNodeId,
    ) -> impl Iterator<Item = &MemoryClusterNodeId> {
        self.neighbors
            .get(node_id)
            .into_iter()
            .flat_map(|set| set.iter())
    }
}

fn resolve_module_path(
    module_path: ParserSourceText<'_>,
    file_paths: &[(CodeSearchPath, MemoryClusterFileId)],
) -> Option<MemoryClusterFileId> {
    let needle = module_path
        .as_str()
        .trim_start_matches("./")
        .trim_start_matches("../");
    let last_segment = needle.rsplit(['/', ':', '.']).next().unwrap_or(needle);
    if last_segment.is_empty() {
        return None;
    }
    file_paths
        .iter()
        .find(|(rel_path, _)| {
            let stem = rel_path
                .as_str()
                .rsplit('/')
                .next()
                .unwrap_or(rel_path.as_str());
            let stem = stem.split('.').next().unwrap_or(stem);
            stem == last_segment || rel_path.as_str().ends_with(last_segment)
        })
        .map(|(_, id)| id.as_str().into())
}

fn resolve_callee(
    callee: ParserSourceText<'_>,
    symbol_names: &[(CodeSearchSymbolName, MemoryClusterNodeId)],
) -> Option<MemoryClusterNodeId> {
    let last_segment = callee
        .as_str()
        .rsplit(['.', ':'])
        .next()
        .unwrap_or(callee.as_str());
    symbol_names
        .iter()
        .find(|(name, _)| name.as_str() == callee.as_str() || name.as_str() == last_segment)
        .map(|(_, id)| id.as_str().into())
}

/// Detect communities ("de-facto modules") in `graph` via deterministic
/// label propagation. `max_iterations` bounds the refinement loop (the
/// algorithm can converge before this; a graph that oscillates between
/// two labelings -- rare but possible for perfectly symmetric
/// fixtures -- stops after this many passes rather than looping
/// forever). Returns an empty [`ClusteringResult`] for an empty graph,
/// never panics.
pub fn detect_clusters(
    graph: &CodeGraph,
    max_iterations: impl Into<MemoryClusterIterationLimit>,
) -> ClusteringResult {
    let max_iterations = max_iterations.into().get();
    let adjacency = UndirectedAdjacency::build(graph);
    if adjacency.node_ids.is_empty() {
        return ClusteringResult::default();
    }

    // Every node starts as its own label -- singleton clusters.
    let mut labels: BTreeMap<MemoryClusterNodeId, MemoryClusterNodeId> = adjacency
        .node_ids
        .iter()
        // CLONE-JUSTIFICATION: labels own independent key and value entries after adjacency borrow.
        .map(|id| (id.clone(), id.clone()))
        .collect();

    for _ in std::iter::repeat_n((), max_iterations) {
        let mut changed = false;
        // Fixed, stable visitation order: sorted node ids, never
        // insertion/hash order and never randomized -- the determinism
        // contract the hard tests require.
        for node_id in &adjacency.node_ids {
            let mut counts: BTreeMap<&MemoryClusterNodeId, usize> = BTreeMap::new();
            for neighbor in adjacency.neighbors_of(node_id) {
                if let Some(label) = labels.get(neighbor) {
                    *counts.entry(label).or_insert(0) += 1;
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
                .map(|(label, _)| label.as_str().into());
            if let Some(best_label) = best_label {
                let Some(current) = labels.get(node_id).cloned() else {
                    // Every adjacency node is seeded above; tolerate a future
                    // index mismatch by leaving the node unchanged rather
                    // than manufacturing an invalid empty domain id.
                    continue;
                };
                if best_label != current {
                    // CLONE-JUSTIFICATION: label map owns node id beyond adjacency iteration.
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
    let mut groups: BTreeMap<MemoryClusterNodeId, BTreeSet<MemoryClusterNodeId>> = BTreeMap::new();
    for (node_id, label) in &labels {
        groups
            // CLONE-JUSTIFICATION: community map owns label beyond borrowed labels iteration.
            .entry(label.clone())
            .or_default()
            // CLONE-JUSTIFICATION: community set owns member id beyond borrowed labels iteration.
            .insert(node_id.clone());
    }

    let file_id_set: BTreeSet<&str> = graph.file_nodes().map(|f| f.id.as_str()).collect();
    let symbol_id_set: BTreeSet<&str> = graph.symbol_nodes().map(|s| s.id.as_str()).collect();

    let mut clusters: Vec<Cluster> = groups
        .into_values()
        .filter_map(|members| {
            let member_node_ids: Vec<MemoryClusterNodeId> = members.into_iter().collect();
            let id = member_node_ids
                .first()
                .map(|member| member.as_str().into())?;
            let file_ids = member_node_ids
                .iter()
                .filter(|id| file_id_set.contains(id.as_str()))
                .map(|id| id.as_str().into())
                .collect();
            let symbol_ids = member_node_ids
                .iter()
                .filter(|id| symbol_id_set.contains(id.as_str()))
                .map(|id| id.as_str().into())
                .collect();
            Some(Cluster {
                id,
                member_node_ids,
                file_ids,
                symbol_ids,
            })
        })
        .collect();
    clusters.sort_by(|a, b| a.id.cmp(&b.id));

    let inter_cluster_edges = inter_cluster_edges(&adjacency, &clusters);

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
    clusters: &[Cluster],
) -> Vec<InterClusterEdge> {
    // raw label -> renormalized cluster id.
    let mut cluster_of_node: BTreeMap<&str, &MemoryClusterId> = BTreeMap::new();
    for cluster in clusters {
        for member in &cluster.member_node_ids {
            cluster_of_node.insert(member.as_str(), &cluster.id);
        }
    }
    let mut counts: BTreeMap<(MemoryClusterId, MemoryClusterId), usize> = BTreeMap::new();
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
            .entry((from_cluster.as_str().into(), to_cluster.as_str().into()))
            .or_insert(0) += 1;
    }

    counts
        .into_iter()
        .map(|((from_cluster, to_cluster), count)| InterClusterEdge {
            from_cluster,
            to_cluster,
            count: count.into(),
        })
        .collect()
}
