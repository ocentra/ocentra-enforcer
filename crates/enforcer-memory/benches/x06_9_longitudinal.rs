//! X06.9 longitudinal benchmark SKELETON: the hook point
//! `MEMORY_RETRIEVAL_QA_BENCHMARKS.md` §3 (longitudinal benchmarks)
//! describes -- index + query on a synthetic fixture repo, timed, with
//! zero non-determinism (no `Instant::now()`-seeded randomness, no
//! wall-clock-dependent corpus generation -- the CORPUS is fully
//! deterministic; only the measured DURATION varies run to run, which
//! is the point of a benchmark).
//!
//! This is a plain `harness = false` bench (`[[bench]]` entry in
//! `Cargo.toml`), not a `criterion` benchmark: the crate's existing
//! dependency posture (D-04/D-07a: "fewest heavy deps that meet
//! behavior") does not yet justify pulling in `criterion` as a new
//! dev-dependency for a SKELETON that does not have a real X06.9
//! longitudinal corpus generator to benchmark yet -- see
//! `MEMORY_RETRIEVAL_QA_BENCHMARKS.md` §3 for what a full longitudinal
//! run (1M nodes, 100k commits, replayed history) will eventually need
//! to drive here. A future pass may swap this for `criterion` once
//! there is a real corpus-scale sweep to run through it; today this
//! prints one deterministic timing sample per corpus size so `cargo
//! bench -p enforcer-memory` has something real to execute.
//!
//! Workspace lints (`unwrap_used`/`expect_used`/`panic`/`print_stdout`
//! = deny) apply to bench targets under `cargo clippy --all-targets`
//! too, matching this crate's test convention -- every fallible step
//! returns `Result` and propagates with `?`, and output goes through
//! `std::io::Write` directly (not the `println!`/`print!` macros the
//! `print_stdout` lint targets) so a real measurement is still printed
//! without an `#[allow(...)]`.

use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::embed::{Embedder, HashingEmbedder};
use enforcer_memory::fulltext::FullTextIndex;
use enforcer_memory::rerank::FusionScoreReranker;
use enforcer_memory::search::{DocumentKind, HybridSearcher, SearchDocument};
use enforcer_memory::vector::{embed_documents, VectorIndex};
use std::error::Error;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

type BoxError = Box<dyn Error>;

/// A small, deterministic synthetic repo generated in-memory (no
/// filesystem fixture needed -- every file's content is a pure function
/// of its index, so two runs on two machines produce byte-identical
/// input). `n` files, each a tiny Rust function calling the previous
/// file's function, giving the indexer a real (if small) call graph to
/// resolve.
fn synthetic_repo_files(n: usize) -> Vec<(String, String)> {
    (0..n)
        .map(|i| {
            let name = format!("gen_{i:04}.rs");
            let calls_prev = if i == 0 {
                String::new()
            } else {
                format!("    gen_{:04}();\n", i - 1)
            };
            let content = format!("pub fn gen_{i:04}() {{\n{calls_prev}    let _ = {i};\n}}\n");
            (name, content)
        })
        .collect()
}

fn run_git(dir: &std::path::Path, args: &[&str]) -> Result<(), BoxError> {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()?;
    if !status.success() {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(())
}

/// Write `files` into a real git working tree at `dir` (content hashes
/// and git history are meaningless without a real repo, matching this
/// crate's existing test convention -- see
/// `tests/code_graph_indexer.rs`), and return the written paths.
fn materialize_repo(
    dir: &std::path::Path,
    files: &[(String, String)],
) -> Result<Vec<PathBuf>, BoxError> {
    run_git(dir, &["init", "--quiet"])?;
    run_git(dir, &["config", "user.email", "bench@example.com"])?;
    run_git(dir, &["config", "user.name", "Bench"])?;

    let mut paths = Vec::with_capacity(files.len());
    for (name, content) in files {
        let path = dir.join(name);
        std::fs::write(&path, content)?;
        paths.push(path);
    }
    run_git(dir, &["add", "-A"])?;
    run_git(
        dir,
        &["commit", "--quiet", "-m", "x06.9 bench synthetic repo"],
    )?;
    Ok(paths)
}

/// One measured sample: index a fresh `file_count`-file synthetic repo
/// from scratch, and separately time a single incremental re-index
/// against the resulting manifest with zero files actually changed
/// (the D-02 "unchanged files are skipped entirely" fast path this
/// bench exists to keep honest over time).
struct BenchSample {
    file_count: usize,
    full_index_ms: f64,
    incremental_noop_index_ms: f64,
}

struct RetrievalLatencySample {
    tier: &'static str,
    file_count: usize,
    p50_ms: f64,
    p95_ms: f64,
}

fn run_sample(file_count: usize) -> Result<BenchSample, BoxError> {
    let dir = tempfile::tempdir()?;
    let files = synthetic_repo_files(file_count);
    let paths = materialize_repo(dir.path(), &files)?;

    let mut graph = CodeGraph::new();
    let start = Instant::now();
    let (manifest, report) = graph.index_repository(dir.path(), &paths, &Manifest::default())?;
    let full_index_ms = start.elapsed().as_secs_f64() * 1000.0;
    if report.added.len() != file_count {
        return Err(format!(
            "expected {file_count} added files, got {}",
            report.added.len()
        )
        .into());
    }

    let start = Instant::now();
    let (_manifest2, report2) = graph.index_repository(dir.path(), &paths, &manifest)?;
    let incremental_noop_index_ms = start.elapsed().as_secs_f64() * 1000.0;
    if report2.unchanged.len() != file_count
        || !report2.added.is_empty()
        || !report2.changed.is_empty()
    {
        return Err("incremental no-op re-index unexpectedly reported changes".into());
    }

    Ok(BenchSample {
        file_count,
        full_index_ms,
        incremental_noop_index_ms,
    })
}

fn synthetic_search_corpus(file_count: usize) -> Vec<SearchDocument> {
    (0..file_count)
        .map(|i| {
            let id = format!("sym:gen_{i:04}.rs:1:gen_{i:04}");
            let previous = if i == 0 {
                "root function".to_owned()
            } else {
                format!("calls gen_{:04}", i - 1)
            };
            SearchDocument::new(
                id,
                DocumentKind::Function,
                format!(
                    "pub fn gen_{i:04} handles synthetic retrieval tier file {i} {previous} config widget route policy"
                ),
            )
        })
        .collect()
}

fn percentile(sorted: &[f64], quantile: f64) -> Result<f64, BoxError> {
    if sorted.is_empty() {
        return Err("cannot compute percentile for empty sample".into());
    }
    let last = sorted.len() - 1;
    let index = ((last as f64) * quantile).ceil() as usize;
    sorted
        .get(index.min(last))
        .copied()
        .ok_or_else(|| "percentile index out of bounds".into())
}

fn run_retrieval_latency_sample(
    tier: &'static str,
    file_count: usize,
) -> Result<RetrievalLatencySample, BoxError> {
    let corpus = synthetic_search_corpus(file_count);
    let fulltext = FullTextIndex::build(&corpus)?;
    let embedder = HashingEmbedder::new();
    let doc_texts: Vec<(String, String)> = corpus
        .iter()
        .map(|doc| (doc.id.clone(), doc.text.clone()))
        .collect();
    let entries = embed_documents(&embedder, &doc_texts)?;
    let vector = VectorIndex::build(&entries, embedder.model_info());
    let reranker = FusionScoreReranker::new();
    let searcher = HybridSearcher::new(&fulltext, &vector, &embedder, &reranker);
    let queries = [
        "config widget policy",
        "synthetic retrieval route",
        "gen function calls previous",
        "file tier route policy",
        "widget route config",
    ];
    let mut latencies = Vec::new();
    for query in queries.iter().cycle().take(20) {
        let start = Instant::now();
        let result = searcher.search(query, &corpus, &[])?;
        if result.context.is_empty() {
            return Err(format!("retrieval query {query:?} returned no context").into());
        }
        latencies.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    latencies.sort_by(f64::total_cmp);
    Ok(RetrievalLatencySample {
        tier,
        file_count,
        p50_ms: percentile(&latencies, 0.50)?,
        p95_ms: percentile(&latencies, 0.95)?,
    })
}

fn main() -> Result<(), BoxError> {
    let mut stdout = std::io::stdout();
    // Deterministic corpus sizes -- the longitudinal §3 "index rebuild
    // time vs incremental update time" tiers this bench is the hook
    // point for. Kept small in this skeleton pass (a real 1M-node tier
    // belongs to the longitudinal corpus generator a later pass owns);
    // this proves the measurement function itself is correct and wired.
    for file_count in [10usize, 50, 100] {
        let sample = run_sample(file_count)?;
        let speedup = if sample.incremental_noop_index_ms > 0.0 {
            sample.full_index_ms / sample.incremental_noop_index_ms
        } else {
            0.0
        };
        writeln!(
            stdout,
            "x06.9 longitudinal bench: files={} full_index_ms={:.3} incremental_noop_index_ms={:.3} speedup={:.1}x",
            sample.file_count, sample.full_index_ms, sample.incremental_noop_index_ms, speedup
        )?;
    }
    let baseline = run_retrieval_latency_sample("baseline", 10)?;
    let large = run_retrieval_latency_sample("large-synthetic", 100)?;
    writeln!(
        stdout,
        "x06.9 retrieval latency: tier={} files={} p50_ms={:.3} p95_ms={:.3}",
        baseline.tier, baseline.file_count, baseline.p50_ms, baseline.p95_ms
    )?;
    writeln!(
        stdout,
        "x06.9 retrieval latency: tier={} files={} p50_ms={:.3} p95_ms={:.3}",
        large.tier, large.file_count, large.p50_ms, large.p95_ms
    )?;
    Ok(())
}
