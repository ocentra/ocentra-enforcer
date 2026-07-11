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
//!
//! [`get_graph_schema_with_similarity`] additionally folds in
//! `SIMILAR_TO`/`SEMANTICALLY_RELATED` row counts from
//! [`crate::similarity`]'s post-index pass -- kept as a separate
//! function (not merged into [`get_graph_schema`] itself) because that
//! pass is O(n²) over callable symbols, not a free introspection over
//! already-stored edges like every other row here.

use std::collections::BTreeMap;

use crate::code_graph::{CodeGraph, CodeNode};

/// One node label's presence in the graph: the label name and how many
/// nodes carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelCount {
    pub label: String,
    pub count: usize,
    pub properties: Vec<String>,
}

/// One edge type's presence in the graph: the edge type name and how
/// many edges of that type exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeTypeCount {
    pub edge_type: String,
    pub count: usize,
    pub properties: Vec<String>,
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
        CodeNode::Method(_) => "Method",
        CodeNode::Class(_) => "Class",
        CodeNode::Struct(_) => "Struct",
        CodeNode::Interface(_) => "Interface",
        CodeNode::Enum(_) => "Enum",
        CodeNode::TypeAlias(_) => "TypeAlias",
        CodeNode::Module(_) => "Module",
        CodeNode::Lambda(_) => "Lambda",
        CodeNode::Variable(_) => "Variable",
        CodeNode::Constant(_) => "Constant",
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
    let mut label_properties: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for node in graph.nodes() {
        let label = node_label(node);
        *label_counts.entry(label).or_insert(0) += 1;
        if let Some(properties) = node_schema_properties(node) {
            let entry = label_properties.entry(label).or_default();
            for property in properties {
                if !entry.iter().any(|existing| existing == property) {
                    entry.push((*property).to_owned());
                }
            }
            entry.sort();
        }
    }

    let mut edge_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut edge_properties: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    if !graph.imports().is_empty() {
        edge_counts.insert("Imports", graph.imports().len());
        edge_properties.insert(
            "Imports",
            vec![
                "from_file_id".to_owned(),
                "module_path".to_owned(),
                "line".to_owned(),
            ],
        );
    }
    if !graph.calls().is_empty() {
        edge_counts.insert("Calls", graph.calls().len());
        edge_properties.insert(
            "Calls",
            vec![
                "from_file_id".to_owned(),
                "callee".to_owned(),
                "line".to_owned(),
            ],
        );
    }
    if !graph.routes().is_empty() {
        edge_counts.insert("Route", graph.routes().len());
        edge_properties.insert(
            "Route",
            vec![
                "from_file_id".to_owned(),
                "method".to_owned(),
                "path".to_owned(),
                "line".to_owned(),
            ],
        );
    }
    if !graph.inherits().is_empty() {
        edge_counts.insert("INHERITS", graph.inherits().len());
    }
    if !graph.implements().is_empty() {
        edge_counts.insert("IMPLEMENTS", graph.implements().len());
    }
    if !graph.decorates().is_empty() {
        edge_counts.insert("DECORATES", graph.decorates().len());
    }
    if !graph.type_refs().is_empty() {
        edge_counts.insert("TYPE_REF", graph.type_refs().len());
    }
    if !graph.defines().is_empty() {
        edge_counts.insert("DEFINES", graph.defines().len());
    }

    let mut labels: Vec<LabelCount> = label_counts
        .into_iter()
        .map(|(label, count)| LabelCount {
            label: label.to_owned(),
            count,
            properties: label_properties.remove(label).unwrap_or_default(),
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
            properties: edge_properties.remove(edge_type).unwrap_or_default(),
        })
        .collect();
    edge_types.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.edge_type.cmp(&b.edge_type))
    });

    GraphSchema { labels, edge_types }
}

/// [`get_graph_schema`] plus `SIMILAR_TO`/`SEMANTICALLY_RELATED` rows,
/// for callers that have already run [`crate::similarity::similar_to`]/
/// [`crate::similarity::semantically_related`] (both an O(n²) post-index
/// pass over callable symbols -- see that module's doc comment -- so
/// this function takes the results rather than recomputing them itself,
/// keeping [`get_graph_schema`] itself a cheap O(n) introspection over
/// [`CodeGraph`]'s already-stored edges). A zero-length edge list omits
/// its row entirely, matching [`get_graph_schema`]'s "never a zero-count
/// row" contract.
pub fn get_graph_schema_with_similarity(
    graph: &CodeGraph,
    similar_to_edges: &[crate::similarity::SimilarToEdge],
    semantically_related_edges: &[crate::similarity::SemanticallyRelatedEdge],
) -> GraphSchema {
    get_graph_schema_with_similarity_modes(graph, similar_to_edges, &[], semantically_related_edges)
}

pub fn get_graph_schema_with_similarity_modes(
    graph: &CodeGraph,
    baseline_similar_to_edges: &[crate::similarity::SimilarToEdge],
    rust_identifier_similar_to_edges: &[crate::similarity::SimilarToEdge],
    semantically_related_edges: &[crate::similarity::SemanticallyRelatedEdge],
) -> GraphSchema {
    let mut schema = get_graph_schema(graph);
    let baseline_minhash_count = baseline_similar_to_edges
        .iter()
        .filter(|edge| edge.mode == crate::similarity::SimilarityMode::MinHashFingerprint)
        .count();
    let body_shingle_count = baseline_similar_to_edges
        .iter()
        .filter(|edge| edge.mode == crate::similarity::SimilarityMode::BodyShingle)
        .count();
    if baseline_minhash_count > 0 {
        schema.edge_types.push(EdgeTypeCount {
            edge_type: "SIMILAR_TO".to_owned(),
            count: baseline_minhash_count,
            properties: vec![
                "source_id".to_owned(),
                "target_id".to_owned(),
                "jaccard".to_owned(),
                "same_file".to_owned(),
            ],
        });
    }
    if body_shingle_count > 0 {
        schema.edge_types.push(EdgeTypeCount {
            edge_type: "BODY_SHINGLE_SIMILAR_TO".to_owned(),
            count: body_shingle_count,
            properties: vec![
                "source_id".to_owned(),
                "target_id".to_owned(),
                "jaccard".to_owned(),
                "same_file".to_owned(),
            ],
        });
    }
    if !rust_identifier_similar_to_edges.is_empty() {
        schema.edge_types.push(EdgeTypeCount {
            edge_type: "RUST_IDENTIFIER_SIMILAR_TO".to_owned(),
            count: rust_identifier_similar_to_edges.len(),
            properties: vec![
                "source_id".to_owned(),
                "target_id".to_owned(),
                "jaccard".to_owned(),
                "same_file".to_owned(),
            ],
        });
    }
    if !semantically_related_edges.is_empty() {
        schema.edge_types.push(EdgeTypeCount {
            edge_type: "SEMANTICALLY_RELATED".to_owned(),
            count: semantically_related_edges.len(),
            properties: vec![
                "source_id".to_owned(),
                "target_id".to_owned(),
                "score".to_owned(),
                "same_file".to_owned(),
            ],
        });
    }
    schema.edge_types.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.edge_type.cmp(&b.edge_type))
    });
    schema
}

fn node_schema_properties(node: &CodeNode) -> Option<&'static [&'static str]> {
    match node {
        CodeNode::Function(sym)
        | CodeNode::Method(sym)
        | CodeNode::Test(sym)
        | CodeNode::Lambda(sym)
            if sym.source_body_fingerprint.is_some() =>
        {
            Some(&["fp", "k"])
        }
        _ => None,
    }
}
