//! X06.3: graph algorithms over [`crate::code_graph::CodeGraph`].
//!
//! `code_graph` deliberately stores nodes as a flat vec plus typed edge
//! lists (import/call/route) that reference other nodes only by string
//! id -- resolution and traversal are this module's job (see
//! `code_graph`'s own module docs: "resolved lazily by callers that
//! need traversal (X06.3's concern)").
//!
//! This module builds a `petgraph::graph::DiGraph` adjacency view once
//! per query session ([`CodeAdjacency::build`]) and layers the
//! workpack's required algorithms on top of it:
//!
//! - [`CodeAdjacency::related`] -- bounded-depth related-node walk;
//! - [`CodeAdjacency::trace_calls`] -- call-path tracing (in/out/both);
//! - [`CodeAdjacency::reverse_dependents`] -- reverse dependency
//!   traversal (upstream callers/importers);
//! - [`CodeAdjacency::hotspots`] -- centrality/hotspot detection
//!   (betweenness-style degree ranking via petgraph);
//!
//! plus the submodules [`query`] (the read-only Cypher-subset DSL,
//! D-05) that sits on top of the same adjacency.
//!
//! # Why petgraph
//!
//! BORROW_POLICY + DECISIONS D-04 context name petgraph as an approved
//! dependency for exactly this class of algorithm (dijkstra,
//! centrality, community detection) rather than hand-rolling BFS/DFS by
//! hand for every traversal shape. This module is a from-scratch
//! re-expression over enforcer's own node/edge model -- no code is
//! copied from any harvested source (harvested-from: none; petgraph
//! itself is the borrowed *dependency*, not borrowed *code*, per
//! MEMORY_RETRIEVAL_BORROW_POLICY §2 TabAgentServer row: "Cargo
//! dependency CHOICES may be adopted directly").

pub mod query;

use crate::code_graph::{CodeGraph, CodeNode};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::{HashMap, HashSet, VecDeque};

/// The kind of relationship an [`AdjacencyEdge`] represents, carried
/// alongside the petgraph edge so traversal results can explain *why*
/// two nodes are connected (the workpack's "traversal reasoning" idea,
/// harvested-idea-only from the MIA framework digest: "found via 2-hop:
/// A -> implies -> B" translated to enforcer's own edge kinds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    /// `from` imports `to` (resolved import -> file target).
    Imports,
    /// `from` calls a symbol resolved to `to`.
    Calls,
    /// `from` declares an HTTP route (synthetic route node -> file).
    Route,
    /// `from` file contains symbol `to` (structural containment).
    Contains,
}

/// One traversal step: the edge kind plus the node id reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedNode {
    pub node_id: String,
    pub depth: usize,
    pub via: EdgeKind,
}

/// A single hop in a traced call/import path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathHop {
    pub node_id: String,
    pub via: EdgeKind,
}

/// One node's computed hotspot score: raw in+out degree, used as the
/// centrality proxy (see [`CodeAdjacency::hotspots`] docs for why
/// degree rather than full betweenness is the right first metric here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotspotScore {
    pub node_id: String,
    pub in_degree: usize,
    pub out_degree: usize,
}

impl HotspotScore {
    pub fn total_degree(&self) -> usize {
        self.in_degree + self.out_degree
    }
}

/// Direction to traverse call/import edges in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceDirection {
    /// Follow edges as declared (caller -> callee, importer -> imported).
    Out,
    /// Follow edges backwards (callee -> caller, imported -> importer).
    In,
    /// Follow both directions.
    Both,
}

/// A resolved import/call adjacency view over one [`CodeGraph`]
/// snapshot. Import module paths and call callee names are resolved to
/// concrete file/symbol node ids on a best-effort basis (exact
/// qualified-path resolution across an arbitrary build system is out of
/// scope, matching `code_graph`'s own module docs); an edge whose target
/// cannot be resolved to a node already in the graph is dropped rather
/// than fabricating a dangling node.
pub struct CodeAdjacency {
    graph: DiGraph<String, EdgeKind>,
    index_of: HashMap<String, NodeIndex>,
}

impl CodeAdjacency {
    /// Build the adjacency view from a [`CodeGraph`] snapshot. `O(nodes
    /// + edges)`; callers that need repeated queries against the same
    /// snapshot should build once and reuse.
    pub fn build(graph: &CodeGraph) -> Self {
        let mut g = DiGraph::<String, EdgeKind>::new();
        let mut index_of: HashMap<String, NodeIndex> = HashMap::new();

        for node in graph.nodes() {
            let id = node.id().to_string();
            index_of.entry(id.clone()).or_insert_with(|| g.add_node(id));
        }

        // File -> symbol containment edges (a symbol's file_id is
        // always a node already inserted above).
        for symbol in graph.symbol_nodes() {
            if let (Some(&from), Some(&to)) =
                (index_of.get(&symbol.file_id), index_of.get(&symbol.id))
            {
                g.add_edge(from, to, EdgeKind::Contains);
            }
        }

        // Import edges: resolve `module_path` to a file node by suffix
        // match against every known rel_path (best-effort, see struct
        // docs) -- e.g. `module_path` "fs" or "./util" matching a file
        // ending in the same stem.
        let file_ids: Vec<(&str, NodeIndex)> = graph
            .file_nodes()
            .filter_map(|f| index_of.get(f.id.as_str()).map(|&idx| (f.rel_path.as_str(), idx)))
            .collect();

        for import in graph.imports() {
            let Some(&from) = index_of.get(&import.from_file_id) else {
                continue;
            };
            if let Some(&to) = resolve_module_path(&import.module_path, &file_ids) {
                if to != from {
                    g.add_edge(from, to, EdgeKind::Imports);
                }
            }
        }

        // Call edges: resolve `callee` to a symbol node by exact-name
        // match (best-effort; see struct docs).
        let symbol_ids: Vec<(&str, NodeIndex)> = graph
            .symbol_nodes()
            .filter_map(|s| index_of.get(s.id.as_str()).map(|&idx| (s.name.as_str(), idx)))
            .collect();

        for call in graph.calls() {
            let Some(&from) = index_of.get(&call.from_file_id) else {
                continue;
            };
            if let Some(&to) = resolve_callee(&call.callee, &symbol_ids) {
                g.add_edge(from, to, EdgeKind::Calls);
            }
        }

        // Route edges: synthetic edge from the declaring file to itself
        // is meaningless, so routes are modeled as a self-loop-free
        // marker: skipped here (route metadata lives on `CodeGraph`
        // directly; `architecture::route_map` reads it from there).
        let _ = graph.routes();

        Self { graph: g, index_of }
    }

    pub fn contains_node(&self, node_id: &str) -> bool {
        self.index_of.contains_key(node_id)
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Bounded-depth related-node walk (BFS) from `start`, following
    /// edges in both directions. `max_depth` is a hard ceiling -- the
    /// workpack's "graph depth limit" hard test asserts nodes beyond
    /// this depth are never returned, even on a graph that connects
    /// further.
    pub fn related(&self, start: &str, max_depth: usize) -> Vec<RelatedNode> {
        let Some(&start_idx) = self.index_of.get(start) else {
            return Vec::new();
        };
        let mut visited: HashSet<NodeIndex> = HashSet::new();
        visited.insert(start_idx);
        let mut frontier: VecDeque<(NodeIndex, usize)> = VecDeque::new();
        frontier.push_back((start_idx, 0));
        let mut out = Vec::new();

        while let Some((idx, depth)) = frontier.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for edge in self.graph.edges_directed(idx, Direction::Outgoing) {
                push_related(&self.graph, &mut visited, &mut frontier, &mut out, edge, depth);
            }
            for edge in self.graph.edges_directed(idx, Direction::Incoming) {
                push_related(&self.graph, &mut visited, &mut frontier, &mut out, edge, depth);
            }
        }
        out
    }

    /// Trace a call/import path from `start` up to `max_depth` hops in
    /// `direction`. Returns every distinct path as a list of hops (the
    /// first hop is the first edge taken away from `start`); `start`
    /// itself is not included as a hop.
    pub fn trace_calls(
        &self,
        start: &str,
        direction: TraceDirection,
        max_depth: usize,
    ) -> Vec<Vec<PathHop>> {
        let Some(&start_idx) = self.index_of.get(start) else {
            return Vec::new();
        };
        let mut paths = Vec::new();
        let mut current = Vec::new();
        self.dfs_paths(start_idx, direction, max_depth, &mut current, &mut paths, &mut HashSet::new());
        paths
    }

    #[allow(clippy::only_used_in_recursion)]
    fn dfs_paths(
        &self,
        idx: NodeIndex,
        direction: TraceDirection,
        remaining: usize,
        current: &mut Vec<PathHop>,
        paths: &mut Vec<Vec<PathHop>>,
        on_path: &mut HashSet<NodeIndex>,
    ) {
        if remaining == 0 {
            if !current.is_empty() {
                paths.push(current.clone());
            }
            return;
        }
        on_path.insert(idx);
        let mut extended = false;

        let directions: &[Direction] = match direction {
            TraceDirection::Out => &[Direction::Outgoing],
            TraceDirection::In => &[Direction::Incoming],
            TraceDirection::Both => &[Direction::Outgoing, Direction::Incoming],
        };

        for &dir in directions {
            for edge in self.graph.edges_directed(idx, dir) {
                let target = other_end(dir, &edge);
                if on_path.contains(&target) {
                    continue;
                }
                extended = true;
                current.push(PathHop {
                    node_id: self.graph[target].clone(),
                    via: *edge.weight(),
                });
                self.dfs_paths(target, direction, remaining - 1, current, paths, on_path);
                current.pop();
            }
        }

        if !extended && !current.is_empty() {
            paths.push(current.clone());
        }
        on_path.remove(&idx);
    }

    /// Reverse dependency traversal: every node that (transitively, up
    /// to `max_depth`) imports or calls into `target`. This is
    /// [`Self::trace_calls`] with [`TraceDirection::In`], flattened to
    /// the unique set of upstream node ids (the workpack's "reverse
    /// dependency traversal" / "upstream callers" hard requirement).
    pub fn reverse_dependents(&self, target: &str, max_depth: usize) -> Vec<String> {
        let mut seen = HashSet::new();
        for path in self.trace_calls(target, TraceDirection::In, max_depth) {
            for hop in path {
                seen.insert(hop.node_id);
            }
        }
        let mut out: Vec<String> = seen.into_iter().collect();
        out.sort();
        out
    }

    /// Centrality/hotspot detection: rank every node by total (in +
    /// out) degree, descending. Degree centrality is the workpack's
    /// minimum-viable hotspot metric (a file called/imported from many
    /// places, or that calls/imports many others, is structurally
    /// significant) and is exact and O(V+E) -- no sampling, no
    /// approximation -- unlike betweenness/eigenvector variants which
    /// this module does not (yet) need for the hard tests.
    pub fn hotspots(&self, limit: usize) -> Vec<HotspotScore> {
        let mut scores: Vec<HotspotScore> = self
            .graph
            .node_indices()
            .map(|idx| HotspotScore {
                node_id: self.graph[idx].clone(),
                in_degree: self.graph.edges_directed(idx, Direction::Incoming).count(),
                out_degree: self.graph.edges_directed(idx, Direction::Outgoing).count(),
            })
            .collect();
        scores.sort_by(|a, b| {
            b.total_degree()
                .cmp(&a.total_degree())
                .then_with(|| a.node_id.cmp(&b.node_id))
        });
        scores.truncate(limit);
        scores
    }

    fn node_ids(&self) -> impl Iterator<Item = &str> {
        self.graph.node_indices().map(move |idx| self.graph[idx].as_str())
    }
}

fn other_end(dir: Direction, edge: &petgraph::graph::EdgeReference<'_, EdgeKind>) -> NodeIndex {
    match dir {
        Direction::Outgoing => edge.target(),
        Direction::Incoming => edge.source(),
    }
}

fn push_related(
    graph: &DiGraph<String, EdgeKind>,
    visited: &mut HashSet<NodeIndex>,
    frontier: &mut VecDeque<(NodeIndex, usize)>,
    out: &mut Vec<RelatedNode>,
    edge: petgraph::graph::EdgeReference<'_, EdgeKind>,
    depth: usize,
) {
    // `edges_directed(idx, Outgoing)` gives edges where `idx` is the
    // source; `edges_directed(idx, Incoming)` gives edges where `idx`
    // is the target. Either way the "other" node is whichever endpoint
    // is not the current frontier node -- petgraph always reports
    // `source()`/`target()` per the edge's own direction, so pick the
    // one that is not already visited/self.
    let a = edge.source();
    let b = edge.target();
    let other = if visited.contains(&a) && a != b { b } else { a };
    let other = if visited.contains(&other) { if other == a { b } else { a } } else { other };
    if visited.insert(other) {
        out.push(RelatedNode {
            node_id: graph[other].clone(),
            depth: depth + 1,
            via: *edge.weight(),
        });
        frontier.push_back((other, depth + 1));
    }
}

/// Best-effort import-path resolution: a `module_path` resolves to a
/// file whose `rel_path` ends with the module path (normalized) or
/// whose file stem equals the module path's final segment. Returns the
/// first match in `file_ids` order (deterministic: `file_ids` is built
/// from `CodeGraph::file_nodes()`, itself insertion-ordered).
fn resolve_module_path<'a>(
    module_path: &str,
    file_ids: &'a [(&'a str, NodeIndex)],
) -> Option<&'a NodeIndex> {
    let needle = module_path.trim_start_matches("./").trim_start_matches("../");
    let last_segment = needle.rsplit(['/', ':', '.']).next().unwrap_or(needle);
    if last_segment.is_empty() {
        return None;
    }
    file_ids
        .iter()
        .find(|(rel_path, _)| {
            let stem = rel_path.rsplit('/').next().unwrap_or(rel_path);
            let stem = stem.split('.').next().unwrap_or(stem);
            stem == last_segment || rel_path.ends_with(last_segment)
        })
        .map(|(_, idx)| idx)
}

/// Best-effort call resolution: `callee` resolves to a symbol node with
/// an exactly-matching name, or whose name is the callee's final
/// path/method segment (`module::func` or `obj.method` -> `func`/`method`).
fn resolve_callee<'a>(
    callee: &str,
    symbol_ids: &'a [(&'a str, NodeIndex)],
) -> Option<&'a NodeIndex> {
    let last_segment = callee.rsplit(['.', ':']).next().unwrap_or(callee);
    symbol_ids
        .iter()
        .find(|(name, _)| *name == callee || *name == last_segment)
        .map(|(_, idx)| idx)
}

/// A minimal read-only iterator surface [`query`] needs to evaluate
/// MATCH patterns without depending on petgraph's internals directly.
pub(crate) struct AdjacencyView<'a> {
    pub(crate) adjacency: &'a CodeAdjacency,
    pub(crate) graph: &'a CodeGraph,
}

impl<'a> AdjacencyView<'a> {
    pub(crate) fn new(adjacency: &'a CodeAdjacency, graph: &'a CodeGraph) -> Self {
        Self { adjacency, graph }
    }

    pub(crate) fn all_node_ids(&self) -> Vec<&str> {
        self.adjacency.node_ids().collect()
    }

    pub(crate) fn code_node(&self, id: &str) -> Option<&CodeNode> {
        self.graph.nodes().iter().find(|n| n.id() == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_graph::Manifest;
    use std::error::Error;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    type TestResult<T = ()> = Result<T, Box<dyn Error>>;

    fn run_git(dir: &Path, args: &[&str]) -> TestResult {
        let status = Command::new("git").args(args).current_dir(dir).status()?;
        if !status.success() {
            return Err(format!("git {args:?} failed").into());
        }
        Ok(())
    }

    fn init_repo(dir: &Path) -> TestResult {
        run_git(dir, &["init", "--quiet"])?;
        run_git(dir, &["config", "user.email", "test@example.com"])?;
        run_git(dir, &["config", "user.name", "Test"])?;
        Ok(())
    }

    fn commit_all(dir: &Path, message: &str) -> TestResult {
        run_git(dir, &["add", "-A"])?;
        run_git(dir, &["commit", "--quiet", "-m", message])?;
        Ok(())
    }

    /// A tiny multi-file fixture repo: `a.rs` calls `helper` (defined in
    /// `b.rs`), `b.rs` imports nothing interesting, `c.rs` is unrelated.
    fn build_fixture_graph(dir: &Path) -> TestResult<CodeGraph> {
        init_repo(dir)?;
        fs::write(dir.join("a.rs"), "fn caller() { helper(); }\n")?;
        fs::write(dir.join("b.rs"), "fn helper() {}\n")?;
        fs::write(dir.join("c.rs"), "fn unrelated() {}\n")?;
        commit_all(dir, "first")?;

        let mut graph = CodeGraph::new();
        let files = vec![
            dir.join("a.rs"),
            dir.join("b.rs"),
            dir.join("c.rs"),
        ];
        graph.index_repository(dir, &files, &Manifest::default())?;
        Ok(graph)
    }

    #[test]
    fn related_walk_finds_connected_tests_within_depth() -> TestResult<()> {
        let dir = tempfile::tempdir()?;
        let graph = build_fixture_graph(dir.path())?;
        let adjacency = CodeAdjacency::build(&graph);

        let file_a = "file:a.rs";
        assert!(adjacency.contains_node(file_a), "expected a.rs file node in adjacency");

        let related = adjacency.related(file_a, 3);
        let ids: HashSet<&str> = related.iter().map(|r| r.node_id.as_str()).collect();
        assert!(
            ids.iter().any(|id| id.contains("caller")),
            "expected a.rs's own caller symbol reachable via Contains edge, got {ids:?}"
        );
        Ok(())
    }

    #[test]
    fn graph_depth_limit_is_enforced() -> TestResult<()> {
        let dir = tempfile::tempdir()?;
        let graph = build_fixture_graph(dir.path())?;
        let adjacency = CodeAdjacency::build(&graph);

        let file_a = "file:a.rs";
        let depth0 = adjacency.related(file_a, 0);
        assert!(depth0.is_empty(), "depth 0 must return no related nodes");

        let depth1 = adjacency.related(file_a, 1);
        for node in &depth1 {
            assert!(node.depth <= 1, "node {:?} exceeded requested depth", node);
        }
        Ok(())
    }

    #[test]
    fn upstream_callers_are_found_via_reverse_dependents() -> TestResult<()> {
        let dir = tempfile::tempdir()?;
        let graph = build_fixture_graph(dir.path())?;
        let adjacency = CodeAdjacency::build(&graph);

        // helper's symbol id is sym:b.rs:1:helper -- find it.
        let helper_id = graph
            .symbol_nodes()
            .find(|s| s.name == "helper")
            .map(|s| s.id.clone())
            .ok_or("expected a helper symbol node")?;

        let upstream = adjacency.reverse_dependents(&helper_id, 3);
        assert!(
            upstream.iter().any(|id| id == "file:a.rs"),
            "expected file:a.rs (the caller) among upstream dependents of helper, got {upstream:?}"
        );
        Ok(())
    }

    #[test]
    fn hotspots_rank_by_total_degree_descending() -> TestResult<()> {
        let dir = tempfile::tempdir()?;
        let graph = build_fixture_graph(dir.path())?;
        let adjacency = CodeAdjacency::build(&graph);

        let scores = adjacency.hotspots(5);
        assert!(!scores.is_empty());
        for i in 1..scores.len() {
            assert!(scores[i - 1].total_degree() >= scores[i].total_degree());
        }
        Ok(())
    }

    #[test]
    fn unknown_start_node_returns_empty_not_panic() -> TestResult<()> {
        let dir = tempfile::tempdir()?;
        let graph = build_fixture_graph(dir.path())?;
        let adjacency = CodeAdjacency::build(&graph);

        assert!(adjacency.related("file:does-not-exist.rs", 5).is_empty());
        assert!(adjacency
            .trace_calls("file:does-not-exist.rs", TraceDirection::Out, 5)
            .is_empty());
        Ok(())
    }
}
