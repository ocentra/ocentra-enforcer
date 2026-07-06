//! X06.P1: `get_graph_schema` -- node labels and edge types present in a
//! [`CodeGraph`], with counts, matching the codebase-memory-mcp parity
//! baseline's `get_graph_schema` tool (scout digest §1, row 6: "node
//! labels + edge types").
//!
//! [`CodeGraph`] only exposes its contents as flat `nodes()`/
//! `imports()`/`calls()`/`routes()` slices (see its module docs) -- this
//! module is pure introspection over those, grouping by discriminant and
//! counting, with no traversal and no mutation.
//!
//! # Ordering matches the baseline: descending by count
//!
//! `docs/plans/enforcer-selfhost-plan/refs/x06-baseline-tool-schemas.md`
//! §3.2 (ground-truth extraction of codebase-memory-mcp's C
//! `get_schema_impl`) confirms the baseline orders both `node_labels`
//! and `edge_types` by **descending row count**, not alphabetically --
//! "dynamic introspection... ordered by descending row count (not
//! alphabetical, not a fixed enum)". This module matches that ordering
//! (ties broken alphabetically by name for determinism, since the
//! baseline's tie-break was not independently verified and equal counts
//! must still produce a reproducible order here).

use std::collections::BTreeMap;

use crate::code_graph::{CodeGraph, CodeNode};

/// One node label's presence in the graph: the label name and how many
/// nodes carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelCount {
    pub label: String,
    pub count: usize,
}

/// One edge type's presence in the graph: the edge type name and how
/// many edges of that type exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeTypeCount {
    pub edge_type: String,
    pub count: usize,
}

/// The full schema summary: every node label and edge type present,
/// each with its count, both in deterministic (alphabetical-by-name)
/// order regardless of the graph's internal node/edge insertion order --
/// two calls against graphs with the same content always produce
/// byte-identical output.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GraphSchema {
    pub labels: Vec<LabelCount>,
    pub edge_types: Vec<EdgeTypeCount>,
}

impl GraphSchema {
    /// Total node count across all labels.
    pub fn total_nodes(&self) -> usize {
        self.labels.iter().map(|l| l.count).sum()
    }

    /// Total edge count across all edge types.
    pub fn total_edges(&self) -> usize {
        self.edge_types.iter().map(|e| e.count).sum()
    }
}

/// The canonical label name for one [`CodeNode`] variant. Kept as a free
/// function (rather than a `Display`/`AsRef<str>` impl on `CodeNode`
/// itself, which this module does not own) so the label vocabulary is
/// defined once, here, and every other place that needs a node's label
/// string (e.g. [`crate::code_search`]) can reuse it instead of
/// re-deriving its own string from a `match`.
pub fn node_label(node: &CodeNode) -> &'static str {
    match node {
        CodeNode::File(_) => "File",
        CodeNode::Function(_) => "Function",
        CodeNode::Type(_) => "Type",
        CodeNode::Test(_) => "Test",
        CodeNode::TextOnly(_) => "TextOnly",
        CodeNode::Tombstone(_) => "Tombstone",
    }
}

/// Compute the schema summary for `graph`: every node label present with
/// its count, and every edge type present ("Imports"/"Calls"/"Route",
/// matching [`crate::code_graph::ImportEdge`]/[`crate::code_graph::CallEdge`]/
/// [`crate::code_graph::RouteEdge`]) with its count. A label/edge type
/// with zero occurrences is omitted entirely (never a zero-count row) --
/// the schema describes what the graph actually contains, not the full
/// static vocabulary of possible labels.
pub fn get_graph_schema(graph: &CodeGraph) -> GraphSchema {
    let mut label_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for node in graph.nodes() {
        *label_counts.entry(node_label(node)).or_insert(0) += 1;
    }

    let mut edge_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    if !graph.imports().is_empty() {
        edge_counts.insert("Imports", graph.imports().len());
    }
    if !graph.calls().is_empty() {
        edge_counts.insert("Calls", graph.calls().len());
    }
    if !graph.routes().is_empty() {
        edge_counts.insert("Route", graph.routes().len());
    }

    let mut labels: Vec<LabelCount> = label_counts
        .into_iter()
        .map(|(label, count)| LabelCount {
            label: label.to_owned(),
            count,
        })
        .collect();
    // Descending by count (baseline parity, see module docs); ties
    // broken alphabetically for a reproducible order the baseline's
    // own tie-break was never independently confirmed to define.
    labels.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.label.cmp(&b.label)));

    let mut edge_types: Vec<EdgeTypeCount> = edge_counts
        .into_iter()
        .map(|(edge_type, count)| EdgeTypeCount {
            edge_type: edge_type.to_owned(),
            count,
        })
        .collect();
    edge_types.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.edge_type.cmp(&b.edge_type))
    });

    GraphSchema { labels, edge_types }
}
