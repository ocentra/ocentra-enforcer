//! X06.9: the parity/benchmark harness SKELETON.
//!
//! This module tree parses the QA-001..QA-250 benchmark rows out of the
//! two binding docs
//! (`docs/plans/enforcer-selfhost-plan/MEMORY_RETRIEVAL_QA_BENCHMARKS.md`
//! and `MEMORY_RETRIEVAL_QA_PROOF_GATE.md`), defines the metric family
//! the QA gate requires (`metrics.rs`), wires a [`RowRunner`] registry
//! against whatever the LANDED library can already answer (`runners.rs`),
//! defines the honest [`baseline::BaselineAdapter`] seam for the future
//! live `codebase-memory-mcp` comparison (`baseline.rs`), and emits the
//! two required proof artifacts (`proof.rs`).
//!
//! # Scope honesty (owner-set: fabricated green is failure)
//!
//! Parallel x06 lanes are still landing MCP/CLI/federation surfaces (see
//! `MEMORY_RETRIEVAL_STATE_BOARD.md`). A full green QA-250 run is not
//! possible today. This harness is a SKELETON that later runs fill in:
//! it must parse every row, define every metric honestly, and record a
//! truthful wired-vs-unrunnable split -- never fabricate a passing
//! result for a capability that does not exist yet. Rows without a
//! wired [`RowRunner`] execute through [`runners::unrunnable`], which
//! records `verdict: "unrunnable: <missing capability>"` -- the QA gate
//! (`MEMORY_RETRIEVAL_QA_PROOF_GATE.md`) treats unrecorded/unrunnable
//! rows as FAILING, not pending, and this harness's proof emitters
//! preserve that failure rather than hiding it.
//!
//! Workspace lints (`unwrap_used`/`expect_used`/`panic` = deny) apply to
//! test code too, matching this crate's existing test convention (see
//! `tests/retrieval_stack.rs`) -- every fallible helper here returns
//! `Result` and propagates with `?` rather than `.expect(...)`.

pub mod baseline;
pub mod metrics;
pub mod proof;
pub mod queryset;
pub mod runners;
pub mod tool_diff;

use enforcer_domain::memory_types::DocumentKind;
use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::embed::{Embedder, HashingEmbedder};
use enforcer_memory::fulltext::FullTextIndex;
use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::ingest;
use enforcer_memory::rerank::FusionScoreReranker;
use enforcer_memory::search::document::SearchDocument;
use enforcer_memory::vector::{embed_documents, VectorIndex};
use runners::Fixtures;
use std::error::Error;
use std::path::PathBuf;
use std::process::Command;

pub type BoxError = Box<dyn Error>;

/// One fixture memory record (a landed lesson) the [`runners::LessonsRunner`]
/// recalls against. Kept as a literal NDJSON string (not a
/// `MemoryRecord` struct literal) so it exercises the same
/// `ingest::parse_ndjson` path production callers use, per this
/// crate's existing test convention (see `tests/ingest_and_recall.rs`).
const FIXTURE_LESSON_NDJSON: &str = r#"{"schemaVersion":1,"id":"mem-x06-9-fixture-0001","ts":"2026-07-05T00:00:00Z","kind":"lesson","domain":"code","statement":"Always parse boundary lesson: validate config paths at the crate boundary before use.","why":"Fixture lesson for the X06.9 harness skeleton's LessonsRunner.","howToApply":"See parse_config_file in the fixture repo.","landedAt":["tests/fixtures/memory/feature_parity/repo/lib.rs"],"provenance":{"writer":"x06-9-harness"}}
{"schemaVersion":1,"id":"mem-x06-9-fixture-0002","ts":"2026-07-05T00:01:00Z","kind":"lesson","domain":"code","statement":"Domain type issue fix: parse raw strings at the boundary into branded newtypes before domain logic uses them.","why":"Raw String ids crossing into domain code caused repeated boundary bugs.","howToApply":"Use branded newtype constructors at parser and CLI/MCP boundaries.","landedAt":["crates/enforcer-memory/src/ids.rs"],"provenance":{"writer":"x06-9-harness"}}
"#;

/// Repo-relative path (from the workspace root) to the harness's own
/// small, deterministic synthetic fixture repo -- separate from X06.2's
/// `tests/fixtures/memory/code_graph/` per this lane's file claims.
const FIXTURE_REPO_DIR: &str = "crates/enforcer-memory/tests/fixtures/memory/feature_parity/repo";

fn run_git(dir: &std::path::Path, args: &[&str]) -> Result<(), BoxError> {
    let status = Command::new("git").args(args).current_dir(dir).status()?;
    if !status.success() {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(())
}

/// Copy the harness's fixture repo into a real, throwaway git working
/// tree and index it, exactly matching X06.2's own
/// `tests/code_graph_indexer.rs` pattern (content hashes/history
/// summaries are meaningless without a real git repo). Returns the
/// resulting [`CodeGraph`] plus the tempdir (kept alive for the
/// caller's lifetime -- dropping it deletes the working tree).
fn build_code_graph() -> Result<(CodeGraph, tempfile::TempDir), BoxError> {
    let dir = tempfile::tempdir()?;
    let workspace_root = queryset::workspace_root();
    let fixture_root = workspace_root.join(FIXTURE_REPO_DIR);

    run_git(dir.path(), &["init", "--quiet"])?;
    run_git(dir.path(), &["config", "user.email", "test@example.com"])?;
    run_git(dir.path(), &["config", "user.name", "Test"])?;

    let mut files: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(&fixture_root)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let dest = dir.path().join(entry.file_name());
            std::fs::copy(entry.path(), &dest)?;
            files.push(dest);
        }
    }
    run_git(dir.path(), &["add", "-A"])?;
    run_git(
        dir.path(),
        &["commit", "--quiet", "-m", "x06.9 fixture repo import"],
    )?;

    let mut graph = CodeGraph::new();
    let (_manifest, _report) = graph.index_repository(dir.path(), &files, &Manifest::default())?;
    Ok((graph, dir))
}

fn build_memory_graph() -> Result<MemoryGraph, BoxError> {
    let mut graph = MemoryGraph::new();
    ingest::ingest_ndjson_into(&mut graph, FIXTURE_LESSON_NDJSON)?;
    Ok(graph)
}

fn build_search_corpus() -> Vec<SearchDocument> {
    vec![
        SearchDocument::new(
            "sym:lib.rs:1:parse_config_file",
            DocumentKind::Function,
            "fn parse_config_file(path: &str) -> Config { read and parse the config file from disk }",
        ),
        SearchDocument::new(
            "sym:widget.rs:1:load_widget_settings",
            DocumentKind::Function,
            "fn load_widget_settings(path: &str) -> Settings { read widget configuration settings from disk }",
        ),
        SearchDocument::new(
            "file:lib.rs",
            DocumentKind::File,
            "the whole lib.rs file, containing parse_config_file and Config",
        ),
    ]
}

/// Build the full [`Fixtures`] environment every [`runners::RowRunner`]
/// executes against: a real indexed fixture repo, an ingested memory
/// graph, and a built full-text/vector/rerank retrieval stack. This is
/// intentionally the ONLY place that constructs [`Fixtures`] so every
/// test/row sees the identical corpus.
pub fn build_fixtures() -> Result<Fixtures, BoxError> {
    let (code_graph, _tempdir_dropped_here) = build_code_graph()?;
    // The tempdir is dropped at the end of this statement, which is
    // fine: `code_graph.index_repository` already consumed the
    // on-disk files into in-memory nodes, and nothing in `Fixtures`
    // holds a path into the deleted directory.
    let memory_graph = build_memory_graph()?;
    let search_corpus = build_search_corpus();

    let fulltext = FullTextIndex::build(&search_corpus)?;
    let embedder = HashingEmbedder::new();
    let doc_texts: Vec<(String, String)> = search_corpus
        .iter()
        .map(|doc| (doc.id.to_string(), doc.text.to_string()))
        .collect();
    let entries = embed_documents(&embedder, &doc_texts)?;
    let vector = VectorIndex::build(&entries, embedder.model_info());
    let reranker = FusionScoreReranker::new();

    Ok(Fixtures {
        code_graph,
        memory_graph,
        fulltext,
        vector,
        embedder,
        reranker,
        search_corpus,
    })
}

#[cfg(test)]
mod fixture_tests {
    use super::{build_fixtures, BoxError};

    type TestResult = Result<(), BoxError>;

    #[test]
    fn fixtures_build_without_error_and_contain_expected_nodes() -> TestResult {
        let fixtures = build_fixtures()?;
        assert!(!fixtures.code_graph.nodes().is_empty());
        assert_eq!(fixtures.search_corpus.len(), 3);
        let hits = enforcer_memory::recall::recall(&fixtures.memory_graph, "parse boundary lesson");
        let mut recalled_ids: Vec<String> =
            hits.iter().map(|hit| hit.node.id().to_string()).collect();
        recalled_ids.sort_unstable();
        assert_eq!(
            recalled_ids,
            vec![
                "mem-x06-9-fixture-0001".to_owned(),
                "mem-x06-9-fixture-0002".to_owned()
            ]
        );
        Ok(())
    }
}
