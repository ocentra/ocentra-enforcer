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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_graph::{CodeGraph, Manifest};
    use std::error::Error;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    type TestResult = std::result::Result<(), Box<dyn Error>>;

    fn init_git_repo(dir: &Path) -> TestResult {
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

    fn run_git(dir: &Path, args: &[&str]) -> TestResult {
        let status = Command::new("git").args(args).current_dir(dir).status()?;
        if !status.success() {
            return Err(format!("git {args:?} failed").into());
        }
        Ok(())
    }

    #[test]
    fn empty_graph_has_no_labels_and_no_edge_types() {
        let graph = CodeGraph::new();
        let schema = get_graph_schema(&graph);
        assert!(schema.labels.is_empty());
        assert!(schema.edge_types.is_empty());
        assert_eq!(schema.total_nodes(), 0);
        assert_eq!(schema.total_edges(), 0);
    }

    #[test]
    fn schema_counts_match_a_mixed_repository_exactly() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;

        let rust_path = dir.path().join("lib.rs");
        fs::write(
            &rust_path,
            "use std::fs;\nstruct Foo;\nfn helper() { fs::read(\"x\"); }\n#[test]\nfn a_test() {}\n",
        )?;
        let js_path = dir.path().join("server.js");
        fs::write(&js_path, "app.get(\"/health\", (req, res) => {});")?;
        let text_only_path = dir.path().join("NOTES.qux");
        fs::write(&text_only_path, "free text")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(
            dir.path(),
            &[rust_path, js_path, text_only_path],
            &Manifest::default(),
        )?;

        let schema = get_graph_schema(&graph);

        let label = |name: &str| -> usize {
            schema
                .labels
                .iter()
                .find(|l| l.label == name)
                .map(|l| l.count)
                .unwrap_or(0)
        };
        // 2 real files + 1 TextOnly file = 3 File-ish nodes, split by
        // label: File=2 (lib.rs, server.js), TextOnly=1 (NOTES.qux).
        assert_eq!(label("File"), 2);
        assert_eq!(label("TextOnly"), 1);
        assert_eq!(label("Function"), 1, "helper");
        assert_eq!(label("Type"), 1, "Foo");
        assert_eq!(label("Test"), 1, "a_test");
        assert_eq!(label("Tombstone"), 0, "omitted at zero count");

        let edge = |name: &str| -> usize {
            schema
                .edge_types
                .iter()
                .find(|e| e.edge_type == name)
                .map(|e| e.count)
                .unwrap_or(0)
        };
        assert_eq!(edge("Imports"), 1);
        // 2 calls: Rust's `fs::read("x")` inside helper(), PLUS the JS
        // route file's `app.get(...)` call expression itself is ALSO
        // recorded as a plain CallEdge (callee "app.get") in addition to
        // being recognized as a RouteEdge -- the two extractions are
        // independent, not mutually exclusive (see
        // crate::languages::typescript's call_expression arm).
        assert_eq!(edge("Calls"), 2);
        assert_eq!(edge("Route"), 1);

        assert_eq!(schema.total_nodes(), graph.nodes().len());
        assert_eq!(
            schema.total_edges(),
            graph.imports().len() + graph.calls().len() + graph.routes().len()
        );
        Ok(())
    }

    #[test]
    fn zero_count_labels_and_edge_types_are_never_emitted() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("plain.rs");
        fs::write(&file_path, "// no symbols, no imports, no calls, no routes\n")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

        let schema = get_graph_schema(&graph);
        assert_eq!(schema.labels.len(), 1);
        assert_eq!(schema.labels[0].label, "File");
        assert!(schema.edge_types.is_empty());
        Ok(())
    }

    #[test]
    fn output_ordering_matches_the_baseline_descending_by_count() -> TestResult {
        // docs/plans/enforcer-selfhost-plan/refs/x06-baseline-tool-schemas.md
        // §3.2: the baseline's get_schema_impl orders node_labels/
        // edge_types by descending row count, not alphabetically. Three
        // functions, one type, one file -- Function (3) must sort
        // before both File (1) and Type (1), and the two count-1 labels
        // must tie-break alphabetically (File < Type).
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("lib.rs");
        fs::write(
            &file_path,
            "struct Foo;\nfn a() {}\nfn b() {}\nfn c() {}\n",
        )?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

        let schema = get_graph_schema(&graph);
        let labels: Vec<(&str, usize)> = schema
            .labels
            .iter()
            .map(|l| (l.label.as_str(), l.count))
            .collect();
        assert_eq!(
            labels,
            vec![("Function", 3), ("File", 1), ("Type", 1)],
            "Function (highest count) must sort first; File/Type (tied at 1) tie-break alphabetically"
        );
        Ok(())
    }

    #[test]
    fn node_label_covers_every_codenode_variant() {
        // Sanity: node_label never panics/falls through on any variant
        // this crate defines -- exercised indirectly by the mixed-repo
        // test above via Tombstone too.
        use crate::code_graph::TombstoneNode;
        let tomb = CodeNode::Tombstone(TombstoneNode {
            id: "tomb:x.rs".to_owned(),
            rel_path: "x.rs".to_owned(),
            last_commit: None,
            change_count: 0,
            prior_chunk_ids: Vec::new(),
        });
        assert_eq!(node_label(&tomb), "Tombstone");
    }
}
