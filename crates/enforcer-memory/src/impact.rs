//! X06.3: impact analysis from a git diff.
//!
//! Answers the workpack's "impact analysis from git diff" hard
//! requirement and mirrors the baseline `detect_changes` tool shape
//! (scout digest §1: "git diff -> affected symbols + risk
//! classification; base_branch/since") without shelling into git
//! itself -- this module takes an already-computed list of changed
//! repo-relative paths (the caller's job: `git diff --name-only
//! base...HEAD` or [`crate::git`] once it grows a diff-listing helper)
//! and walks [`crate::analysis::CodeAdjacency`] to find every node
//! transitively impacted.

use crate::analysis::CodeAdjacency;
use crate::code_graph::CodeGraph;
use std::collections::BTreeSet;

/// One changed file's blast radius.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpactedFile {
    pub rel_path: String,
    /// Node ids of every symbol/file that transitively depends on this
    /// file (reverse dependents), up to the analysis depth.
    pub affected_node_ids: Vec<String>,
    pub risk: RiskLevel,
}

/// A coarse risk classification: how many nodes are in the blast
/// radius. Thresholds are a deliberately simple, documented starting
/// point (not the baseline's exact classifier, which is closed-source
/// C -- BORROW_POLICY treats it as behavior-spec-only, not code to
/// copy) -- tunable later without changing the shape callers see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

fn classify_risk(affected_count: usize) -> RiskLevel {
    match affected_count {
        0..=2 => RiskLevel::Low,
        3..=10 => RiskLevel::Medium,
        _ => RiskLevel::High,
    }
}

/// The full impact report for one diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpactReport {
    pub changed_paths: Vec<String>,
    pub impacted: Vec<ImpactedFile>,
    /// The union of every impacted node id across all changed files.
    pub total_affected_node_ids: Vec<String>,
}

/// Analyze the impact of `changed_paths` (repo-relative,
/// forward-slash-normalized, matching [`crate::code_graph::FileNode::rel_path`])
/// against `graph`. `max_depth` bounds the reverse-dependency walk
/// (same depth-limit contract as [`CodeAdjacency::related`]).
pub fn analyze_diff_impact(graph: &CodeGraph, changed_paths: &[String], max_depth: usize) -> ImpactReport {
    let adjacency = CodeAdjacency::build(graph);
    let mut impacted = Vec::new();
    let mut total: BTreeSet<String> = BTreeSet::new();

    for rel_path in changed_paths {
        let file_id = format!("file:{rel_path}");
        let affected = adjacency.reverse_dependents(&file_id, max_depth);
        for id in &affected {
            total.insert(id.clone());
        }
        let risk = classify_risk(affected.len());
        impacted.push(ImpactedFile {
            rel_path: rel_path.clone(),
            affected_node_ids: affected,
            risk,
        });
    }

    ImpactReport {
        changed_paths: changed_paths.to_vec(),
        impacted,
        total_affected_node_ids: total.into_iter().collect(),
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

    type TestResult = Result<(), Box<dyn Error>>;

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

    /// `a.rs` calls `helper` defined in `b.rs`; changing `b.rs` should
    /// mark `a.rs` as impacted via the CALLS edge's reverse direction.
    #[test]
    fn diff_impact_finds_transitively_affected_files() -> TestResult<()> {
        let dir = tempfile::tempdir()?;
        init_repo(dir.path())?;
        fs::write(dir.path().join("a.rs"), "fn caller() { helper(); }\n")?;
        fs::write(dir.path().join("b.rs"), "fn helper() {}\n")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        let files = vec![dir.path().join("a.rs"), dir.path().join("b.rs")];
        graph.index_repository(dir.path(), &files, &Manifest::default())?;

        let report = analyze_diff_impact(&graph, &["b.rs".to_string()], 3);
        assert_eq!(report.impacted.len(), 1);
        let impacted_b = &report.impacted[0];
        assert_eq!(impacted_b.rel_path, "b.rs");
        assert!(
            impacted_b.affected_node_ids.iter().any(|id| id == "file:a.rs"),
            "expected file:a.rs among impacted nodes for changing b.rs, got {:?}",
            impacted_b.affected_node_ids
        );
        Ok(())
    }

    #[test]
    fn risk_classification_scales_with_affected_count() {
        assert_eq!(classify_risk(0), RiskLevel::Low);
        assert_eq!(classify_risk(2), RiskLevel::Low);
        assert_eq!(classify_risk(3), RiskLevel::Medium);
        assert_eq!(classify_risk(10), RiskLevel::Medium);
        assert_eq!(classify_risk(11), RiskLevel::High);
    }

    #[test]
    fn unknown_changed_path_is_reported_with_zero_impact_not_panic() -> TestResult<()> {
        let dir = tempfile::tempdir()?;
        init_repo(dir.path())?;
        fs::write(dir.path().join("a.rs"), "fn a() {}\n")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[dir.path().join("a.rs")], &Manifest::default())?;

        let report = analyze_diff_impact(&graph, &["does-not-exist.rs".to_string()], 3);
        assert_eq!(report.impacted.len(), 1);
        assert!(report.impacted[0].affected_node_ids.is_empty());
        assert_eq!(report.impacted[0].risk, RiskLevel::Low);
        Ok(())
    }
}
