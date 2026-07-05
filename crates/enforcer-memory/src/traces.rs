//! X06.P2: parity `ingest_traces` -- merge runtime call-trace records
//! into the code graph's call-edge model (scout digest §1, row 14:
//! "runtime traces {caller, callee, count} enrich CALLS edges").
//!
//! [`crate::code_graph::CodeGraph`] only ever records *parsed* call
//! edges ([`crate::code_graph::CallEdge`], no `count` field, no
//! provenance tag). This module builds a separate, additive overlay
//! ([`TraceStore`]) rather than mutating `CodeGraph` in place -- adding
//! a mutable "annotate this edge" API to `CodeGraph` itself is out of
//! this lane's file claims (`code_graph.rs` internals are consume-only
//! per the mission's DO-NOT-TOUCH list) and would also blur X06.2's
//! "parsed" edges with runtime-observed ones. Instead:
//!
//! - a caller first indexes the static graph ([`crate::code_graph`]);
//! - then calls [`TraceStore::ingest`] with runtime `{caller, callee,
//!   count}` records and, separately, `graph.calls()` so ingestion can
//!   tell which edges already exist (provenance = parsed vs runtime);
//! - [`TraceStore::edges`] returns every edge (parsed-with-runtime-count,
//!   or runtime-only) as a [`TracedEdge`] carrying an explicit
//!   [`EdgeProvenance`] -- never conflating "we saw this in source" with
//!   "we saw this actually execute" (QA-080's parsed-vs-inferred-vs-
//!   runtime split, named directly in this lane's mission).
//!
//! # Idempotent re-ingestion
//!
//! Re-ingesting the exact same trace batch twice **sums** the counts
//! (documented choice, tested by
//! [`tests::reingesting_the_same_batch_sums_counts`]): a runtime trace
//! collector legitimately re-reports the same caller/callee pair every
//! collection interval, and each report represents *additional*
//! observed calls, not a restatement of a cumulative total -- summing
//! matches that collection model. A caller that instead holds a
//! cumulative counter and wants "replace, not add" semantics should
//! call [`TraceStore::reset`] before re-ingesting that snapshot; this
//! module does not guess which one an unlabeled batch is.
//!
//! # Unresolved symbols
//!
//! A trace record whose `caller` or `callee` does not match a known
//! [`crate::code_graph::CodeNode`] id is never silently dropped: it is
//! recorded in [`TraceStore::unresolved`] as an [`UnresolvedTrace`],
//! same "never silent skip" doctrine [`crate::code_graph`] itself
//! follows for unsupported file extensions.

use crate::code_graph::{CallEdge, CodeGraph, CodeNode};
use std::collections::BTreeMap;

/// One runtime-observed call record, as a caller (e.g. an APM/tracing
/// exporter) would report it: `caller`/`callee` are graph node ids
/// (matching [`crate::code_graph::CodeNode::id`] -- typically a
/// `sym:`-prefixed symbol id, though a `file:`-prefixed id is accepted
/// too since not every runtime tracer resolves calls to symbol
/// granularity), and `count` is how many times this edge was observed
/// in the reporting window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceRecord {
    pub caller: String,
    pub callee: String,
    pub count: u64,
}

/// Where a [`TracedEdge`] came from: parsed from source
/// ([`crate::code_graph`]'s static extraction), inferred (reserved for a
/// future best-effort resolution pass -- not produced by this module),
/// or observed at runtime via [`TraceStore::ingest`]. Mirrors the
/// parity digest's QA-080 "provenance split... matters" requirement
/// directly in the type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EdgeProvenance {
    Parsed,
    Inferred,
    Runtime,
}

/// One caller->callee edge with its provenance and observed runtime
/// count (0 if never runtime-observed -- a parsed-only edge with no
/// matching trace record).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracedEdge {
    pub caller: String,
    pub callee: String,
    pub provenance: EdgeProvenance,
    pub observed_count: u64,
}

/// A trace record whose `caller` or `callee` (or both) could not be
/// resolved to a known graph node id at ingestion time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedTrace {
    pub record: TraceRecord,
    pub unresolved_caller: bool,
    pub unresolved_callee: bool,
}

/// The additive runtime-trace overlay described in the module docs.
/// Starts empty; [`TraceStore::ingest`] merges batches of
/// [`TraceRecord`]s against a [`CodeGraph`] snapshot.
#[derive(Debug, Clone, Default)]
pub struct TraceStore {
    /// Keyed by (caller, callee) so re-ingestion naturally finds the
    /// existing entry to sum into (see module docs, "idempotent
    /// re-ingestion").
    runtime_counts: BTreeMap<(String, String), u64>,
    unresolved: Vec<UnresolvedTrace>,
}

impl TraceStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge `records` into this store against `graph`. Every record is
    /// classified:
    ///
    /// - both `caller` and `callee` resolve to known node ids -> the
    ///   runtime count for that pair is summed into `runtime_counts`
    ///   (see module docs on idempotent re-ingestion);
    /// - either side fails to resolve -> the record is appended to
    ///   [`Self::unresolved`], never silently dropped, and its count is
    ///   NOT added to `runtime_counts` (an edge naming an unknown node
    ///   would be a dangling/fabricated edge if merged in).
    pub fn ingest(&mut self, graph: &CodeGraph, records: &[TraceRecord]) {
        let known_ids = known_node_ids(graph);
        for record in records {
            let caller_known = known_ids.contains(record.caller.as_str());
            let callee_known = known_ids.contains(record.callee.as_str());
            if caller_known && callee_known {
                let key = (record.caller.clone(), record.callee.clone());
                *self.runtime_counts.entry(key).or_insert(0) += record.count;
            } else {
                self.unresolved.push(UnresolvedTrace {
                    record: record.clone(),
                    unresolved_caller: !caller_known,
                    unresolved_callee: !callee_known,
                });
            }
        }
    }

    /// Clear every accumulated runtime count (but not [`Self::unresolved`]
    /// history) -- the escape hatch for a caller that wants "replace,
    /// not add" semantics on the next [`Self::ingest`] call (see module
    /// docs).
    pub fn reset(&mut self) {
        self.runtime_counts.clear();
    }

    /// Every unresolved trace record ingested so far, in ingestion
    /// order (stable: `Vec`, never reordered).
    pub fn unresolved(&self) -> &[UnresolvedTrace] {
        &self.unresolved
    }

    /// The merged edge view: every parsed [`CallEdge`] annotated with
    /// its runtime-observed count (0 if none), plus every runtime-only
    /// pair that has no matching parsed edge (provenance
    /// [`EdgeProvenance::Runtime`]). `graph`'s parsed calls are matched
    /// to a runtime pair by resolving [`CallEdge::callee`] the same way
    /// [`super::analysis::CodeAdjacency`] does (exact name or trailing
    /// path/method segment) against the runtime pair's callee id --
    /// simplified here to an exact-id match against `caller` = the
    /// call's `from_file_id` and `callee` = the resolved symbol id,
    /// since callers of this API are expected to have already resolved
    /// symbol ids the same way trace exporters that instrument function
    /// entry/exit naturally do (symbol-id-to-symbol-id, not raw source
    /// text).
    ///
    /// Deterministic ordering: sorted by `(caller, callee)`.
    pub fn edges(&self, graph: &CodeGraph) -> Vec<TracedEdge> {
        let mut merged: BTreeMap<(String, String), TracedEdge> = BTreeMap::new();

        for call in graph.calls() {
            let key = (call.from_file_id.clone(), call.callee.clone());
            merged.entry(key.clone()).or_insert(TracedEdge {
                caller: call.from_file_id.clone(),
                callee: call.callee.clone(),
                provenance: EdgeProvenance::Parsed,
                observed_count: 0,
            });
        }

        for ((caller, callee), count) in &self.runtime_counts {
            let key = (caller.clone(), callee.clone());
            match merged.get_mut(&key) {
                Some(edge) => edge.observed_count = *count,
                None => {
                    merged.insert(
                        key,
                        TracedEdge {
                            caller: caller.clone(),
                            callee: callee.clone(),
                            provenance: EdgeProvenance::Runtime,
                            observed_count: *count,
                        },
                    );
                }
            }
        }

        merged.into_values().collect()
    }
}

/// Every node id present in `graph` (files, symbols, tombstones -- any
/// id a trace record could legitimately reference).
fn known_node_ids(graph: &CodeGraph) -> std::collections::HashSet<&str> {
    graph.nodes().iter().map(CodeNode::id).collect()
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

    fn build_fixture_graph(dir: &Path) -> TestResult<CodeGraph> {
        init_repo(dir)?;
        fs::write(dir.join("a.rs"), "fn caller() { helper(); }\n")?;
        fs::write(dir.join("b.rs"), "fn helper() {}\n")?;
        commit_all(dir, "first")?;

        let mut graph = CodeGraph::new();
        let files = vec![dir.join("a.rs"), dir.join("b.rs")];
        graph.index_repository(dir, &files, &Manifest::default())?;
        Ok(graph)
    }

    #[test]
    fn ingest_annotates_an_existing_parsed_edge_with_runtime_count() -> TestResult {
        let dir = tempfile::tempdir()?;
        let graph = build_fixture_graph(dir.path())?;

        // graph.calls() records file:a.rs -> "helper" (callee as
        // written, per code_graph::CallEdge -- see module docs).
        let mut store = TraceStore::new();
        store.ingest(
            &graph,
            &[TraceRecord {
                caller: "file:a.rs".to_string(),
                callee: "helper".to_string(),
                count: 5,
            }],
        );

        let edges = store.edges(&graph);
        let annotated = edges
            .iter()
            .find(|e| e.caller == "file:a.rs" && e.callee == "helper")
            .ok_or("expected an annotated edge")?;
        assert_eq!(annotated.provenance, EdgeProvenance::Parsed);
        assert_eq!(annotated.observed_count, 5);
        assert!(store.unresolved().is_empty());
        Ok(())
    }

    #[test]
    fn ingest_creates_a_runtime_only_edge_when_no_parsed_edge_exists() -> TestResult {
        let dir = tempfile::tempdir()?;
        let graph = build_fixture_graph(dir.path())?;

        let helper_id = graph
            .symbol_nodes()
            .find(|s| s.name == "helper")
            .map(|s| s.id.clone())
            .ok_or("expected helper symbol")?;
        let caller_id = graph
            .symbol_nodes()
            .find(|s| s.name == "caller")
            .map(|s| s.id.clone())
            .ok_or("expected caller symbol")?;

        let mut store = TraceStore::new();
        // symbol-id -> symbol-id has no matching parsed CallEdge (parsed
        // edges are file_id -> raw callee name), so this must appear as
        // a brand-new Runtime-provenance edge.
        store.ingest(
            &graph,
            &[TraceRecord {
                caller: caller_id.clone(),
                callee: helper_id.clone(),
                count: 3,
            }],
        );

        let edges = store.edges(&graph);
        let runtime_edge = edges
            .iter()
            .find(|e| e.caller == caller_id && e.callee == helper_id)
            .ok_or("expected a runtime-only edge")?;
        assert_eq!(runtime_edge.provenance, EdgeProvenance::Runtime);
        assert_eq!(runtime_edge.observed_count, 3);
        Ok(())
    }

    #[test]
    fn reingesting_the_same_batch_sums_counts() -> TestResult {
        let dir = tempfile::tempdir()?;
        let graph = build_fixture_graph(dir.path())?;

        let batch = vec![TraceRecord {
            caller: "file:a.rs".to_string(),
            callee: "helper".to_string(),
            count: 4,
        }];

        let mut store = TraceStore::new();
        store.ingest(&graph, &batch);
        store.ingest(&graph, &batch);

        let edges = store.edges(&graph);
        let edge = edges
            .iter()
            .find(|e| e.caller == "file:a.rs" && e.callee == "helper")
            .ok_or("expected the edge")?;
        assert_eq!(
            edge.observed_count, 8,
            "re-ingesting the same batch twice must SUM counts (documented idempotency choice)"
        );
        Ok(())
    }

    #[test]
    fn reset_then_reingest_replaces_counts() -> TestResult {
        let dir = tempfile::tempdir()?;
        let graph = build_fixture_graph(dir.path())?;

        let batch = vec![TraceRecord {
            caller: "file:a.rs".to_string(),
            callee: "helper".to_string(),
            count: 10,
        }];

        let mut store = TraceStore::new();
        store.ingest(&graph, &batch);
        store.reset();
        store.ingest(&graph, &batch);

        let edges = store.edges(&graph);
        let edge = edges
            .iter()
            .find(|e| e.caller == "file:a.rs" && e.callee == "helper")
            .ok_or("expected the edge")?;
        assert_eq!(edge.observed_count, 10, "reset() must clear prior counts");
        Ok(())
    }

    #[test]
    fn unknown_caller_or_callee_is_recorded_not_dropped() -> TestResult {
        let dir = tempfile::tempdir()?;
        let graph = build_fixture_graph(dir.path())?;

        let mut store = TraceStore::new();
        store.ingest(
            &graph,
            &[
                TraceRecord {
                    caller: "sym:does-not-exist.rs:1:ghost".to_string(),
                    callee: "helper".to_string(),
                    count: 1,
                },
                TraceRecord {
                    caller: "file:a.rs".to_string(),
                    callee: "sym:does-not-exist.rs:1:ghost".to_string(),
                    count: 1,
                },
            ],
        );

        assert_eq!(store.unresolved().len(), 2);
        assert!(store.unresolved()[0].unresolved_caller);
        assert!(!store.unresolved()[0].unresolved_callee);
        assert!(!store.unresolved()[1].unresolved_caller);
        assert!(store.unresolved()[1].unresolved_callee);

        // Neither malformed record should have been merged into edges().
        let edges = store.edges(&graph);
        assert!(edges
            .iter()
            .all(|e| !e.caller.contains("ghost") && !e.callee.contains("ghost")));
        Ok(())
    }

    #[test]
    fn ingest_is_deterministically_ordered_by_caller_then_callee() -> TestResult {
        let dir = tempfile::tempdir()?;
        let graph = build_fixture_graph(dir.path())?;

        let mut store = TraceStore::new();
        store.ingest(
            &graph,
            &[
                TraceRecord {
                    caller: "file:a.rs".to_string(),
                    callee: "zzz-unknown".to_string(),
                    count: 1,
                },
                TraceRecord {
                    caller: "file:a.rs".to_string(),
                    callee: "helper".to_string(),
                    count: 1,
                },
            ],
        );

        let edges_a = store.edges(&graph);
        let edges_b = store.edges(&graph);
        assert_eq!(edges_a, edges_b, "edges() must be deterministic");

        let callers_callees: Vec<(&str, &str)> = edges_a
            .iter()
            .map(|e| (e.caller.as_str(), e.callee.as_str()))
            .collect();
        let mut sorted = callers_callees.clone();
        sorted.sort();
        assert_eq!(callers_callees, sorted, "edges() must be sorted by (caller, callee)");
        Ok(())
    }
}
