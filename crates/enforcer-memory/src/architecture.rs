//! X06.3: architecture overview / repo mind map.
//!
//! Answers the "architecture overview" and "repo mind map" hard
//! requirements by grouping [`crate::code_graph::CodeGraph`] file nodes
//! into a crate/module map, layering in the hotspot scores from
//! [`crate::analysis::CodeAdjacency::hotspots`], and reporting basic
//! layer/language composition -- the enforcer-scoped analogue of the
//! codebase-memory-mcp baseline's `get_architecture` tool (scout digest
//! §1: "aspects incl. Leiden/Louvain clustering, hotspots, layers,
//! file_tree"). This slice covers file_tree (via crate grouping) and
//! hotspots exactly; full graph-clustering (Leiden/Louvain) is not
//! implemented -- degree-based grouping by top-level directory is the
//! floor this pack's hard test ("architecture sections") requires.

use crate::analysis::CodeAdjacency;
use crate::code_graph::CodeGraph;

/// One crate/top-level-directory section of the architecture overview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateSection {
    /// The top-level path segment (e.g. `crates/enforcer-memory` or the
    /// crate name if the repo root itself contains `Cargo.toml` files
    /// one level down) this section groups.
    pub name: String,
    pub file_count: usize,
    pub symbol_count: usize,
    pub rel_paths: Vec<String>,
}

/// The full architecture overview: crate map + hotspots + language
/// composition. Constructed by [`build_overview`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchitectureOverview {
    pub sections: Vec<CrateSection>,
    pub hotspots: Vec<crate::analysis::HotspotScore>,
    pub language_counts: Vec<(String, usize)>,
    pub total_files: usize,
    pub total_symbols: usize,
}

/// Build the architecture overview for `graph`. `hotspot_limit` bounds
/// how many top hotspot entries are retained (the workpack does not
/// mandate a specific number; callers pick per their MCP/CLI surface).
pub fn build_overview(graph: &CodeGraph, hotspot_limit: usize) -> ArchitectureOverview {
    let mut sections: std::collections::BTreeMap<String, CrateSection> =
        std::collections::BTreeMap::new();

    for file in graph.file_nodes() {
        let crate_name = crate_map_key(&file.rel_path);
        let section = sections
            .entry(crate_name.clone())
            .or_insert_with(|| CrateSection {
                name: crate_name,
                file_count: 0,
                symbol_count: 0,
                rel_paths: Vec::new(),
            });
        section.file_count += 1;
        section.rel_paths.push(file.rel_path.clone());
    }

    for symbol in graph.symbol_nodes() {
        // symbol.file_id is `file:<rel_path>`; strip the prefix to
        // recover the path this symbol belongs to for section grouping.
        let rel_path = symbol
            .file_id
            .strip_prefix("file:")
            .unwrap_or(&symbol.file_id);
        let crate_name = crate_map_key(rel_path);
        if let Some(section) = sections.get_mut(&crate_name) {
            section.symbol_count += 1;
        }
    }

    let mut language_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for file in graph.file_nodes() {
        *language_counts
            .entry(format!("{:?}", file.language))
            .or_insert(0) += 1;
    }

    let adjacency = CodeAdjacency::build(graph);
    let hotspots = adjacency.hotspots(hotspot_limit);

    let total_files = graph.file_nodes().count();
    let total_symbols = graph.symbol_nodes().count();

    ArchitectureOverview {
        sections: sections.into_values().collect(),
        hotspots,
        language_counts: language_counts.into_iter().collect(),
        total_files,
        total_symbols,
    }
}

/// Group a repo-relative path into a crate/section key: everything up
/// to (and including) the second path segment when the first segment
/// is `crates` (e.g. `crates/enforcer-memory/src/lib.rs` ->
/// `crates/enforcer-memory`), otherwise the first path segment, or
/// `"."` for a root-level file with no directory.
fn crate_map_key(rel_path: &str) -> String {
    let segments: Vec<&str> = rel_path.split('/').collect();
    match segments.as_slice() {
        [] => ".".to_string(),
        [_only] => ".".to_string(),
        ["crates", crate_name, ..] => format!("crates/{crate_name}"),
        [first, ..] => (*first).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_graph::{CodeGraph, Manifest};
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

    #[test]
    fn architecture_overview_groups_files_into_crate_sections() -> TestResult<()> {
        let dir = tempfile::tempdir()?;
        init_repo(dir.path())?;
        fs::create_dir_all(dir.path().join("crates/foo/src"))?;
        fs::create_dir_all(dir.path().join("crates/bar/src"))?;
        fs::write(dir.path().join("crates/foo/src/lib.rs"), "fn a() {}\n")?;
        fs::write(dir.path().join("crates/bar/src/lib.rs"), "fn b() {}\n")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        let files = vec![
            dir.path().join("crates/foo/src/lib.rs"),
            dir.path().join("crates/bar/src/lib.rs"),
        ];
        graph.index_repository(dir.path(), &files, &Manifest::default())?;

        let overview = build_overview(&graph, 10);
        let names: Vec<&str> = overview.sections.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"crates/foo"));
        assert!(names.contains(&"crates/bar"));
        assert_eq!(overview.total_files, 2);
        assert_eq!(overview.total_symbols, 2);
        Ok(())
    }

    #[test]
    fn architecture_overview_reports_language_composition_and_hotspots() -> TestResult<()> {
        let dir = tempfile::tempdir()?;
        init_repo(dir.path())?;
        fs::write(dir.path().join("a.rs"), "fn caller() { helper(); }\n")?;
        fs::write(dir.path().join("b.rs"), "fn helper() {}\n")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        let files = vec![dir.path().join("a.rs"), dir.path().join("b.rs")];
        graph.index_repository(dir.path(), &files, &Manifest::default())?;

        let overview = build_overview(&graph, 5);
        assert!(overview
            .language_counts
            .iter()
            .any(|(lang, count)| lang == "Rust" && *count == 2));
        assert!(
            !overview.hotspots.is_empty(),
            "expected at least one hotspot entry"
        );
        Ok(())
    }

    #[test]
    fn empty_graph_produces_empty_overview_not_panic() {
        let graph = CodeGraph::new();
        let overview = build_overview(&graph, 5);
        assert!(overview.sections.is_empty());
        assert_eq!(overview.total_files, 0);
        assert_eq!(overview.total_symbols, 0);
    }
}
