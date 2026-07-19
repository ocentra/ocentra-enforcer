//! X06.P2: parity `ingest_traces` -- merge runtime call-trace records
//! into the code graph's call-edge model (scout digest Â§1, row 14:
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

use crate::boundary::log_schema::{ObservationLogEntryDto, TraceRecordDto, SCHEMA_VERSION};
use crate::code_graph::CodeGraph;
use crate::error::Result;
use crate::owned_boundary::Retained;
use crate::store::Store;
use enforcer_domain::memory_types::{
    EdgeProvenance, IngestSourceSurface, IngestTimestamp, ProceduralLessonReference, TraceNodeId,
    TraceObservationCount, TraceStoreRecordCount, TraceUnresolvedCallee, TraceUnresolvedCaller,
};
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
    pub caller: TraceNodeId,
    pub callee: TraceNodeId,
    pub count: TraceObservationCount,
}

/// Where a [`TracedEdge`] came from: parsed from source
/// ([`crate::code_graph`]'s static extraction), inferred (reserved for a
/// future best-effort resolution pass -- not produced by this module),
/// or observed at runtime via [`TraceStore::ingest`]. Mirrors the
/// parity digest's QA-080 "provenance split... matters" requirement
/// directly in the type.
/// One caller->callee edge with its provenance and observed runtime
/// count (0 if never runtime-observed -- a parsed-only edge with no
/// matching trace record).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracedEdge {
    pub caller: TraceNodeId,
    pub callee: TraceNodeId,
    pub provenance: EdgeProvenance,
    pub observed_count: TraceObservationCount,
}

/// A trace record whose `caller` or `callee` (or both) could not be
/// resolved to a known graph node id at ingestion time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedTrace {
    pub record: TraceRecord,
    pub unresolved_caller: TraceUnresolvedCaller,
    pub unresolved_callee: TraceUnresolvedCallee,
}

#[derive(Debug)]
pub struct TraceRecordStoreBatch<'a> {
    pub records: &'a [TraceRecord],
    pub source_surface: IngestSourceSurface,
    pub ts: IngestTimestamp,
}

impl<'a> TraceRecordStoreBatch<'a> {
    pub fn new(
        records: &'a [TraceRecord],
        source_surface: impl Into<IngestSourceSurface>,
        ts: impl Into<IngestTimestamp>,
    ) -> Self {
        Self {
            records,
            source_surface: source_surface.into(),
            ts: ts.into(),
        }
    }
}

pub fn ingest_trace_records_into_store(
    store: &mut Store,
    trace_store: &mut TraceStore,
    graph: &CodeGraph,
    batch: &TraceRecordStoreBatch<'_>,
) -> Result<TraceStoreRecordCount> {
    for record in batch.records {
        let payload = serde_json::to_value(TraceRecordDto::from(record))?;
        store.append_observation_entry(|seq| ObservationLogEntryDto {
            schema_version: SCHEMA_VERSION,
            seq: seq.into(),
            id: format!("trace-{seq:04}"),
            lesson_id: ProceduralLessonReference::default().into(),
            rule_id: None,
            fault_class: Some("runtime-trace".retained()),
            repo_context: format!("{} -> {}", record.caller, record.callee),
            clean: true,
            // CLONE-JUSTIFICATION: emitted observation outlives the borrowed batch.
            source_surface: batch.source_surface.as_str().retained(),
            ts: batch.ts.as_str().retained(),
            supersedes_seq: None,
            payload_kind: Some("runtime-trace".retained()),
            payload: Some(payload),
        })?;
    }
    trace_store.ingest(graph, batch.records);
    Ok(batch.records.len().into())
}

pub fn replay_trace_records_from_store(
    store: &Store,
    trace_store: &mut TraceStore,
    graph: &CodeGraph,
) -> Result<TraceStoreRecordCount> {
    let outcome = store.read_observation_entries()?;
    let mut records = Vec::new();
    for entry in outcome.entries {
        if entry.payload_kind.as_deref() != Some("runtime-trace") {
            continue;
        }
        if let Some(payload) = entry.payload {
            records.push(serde_json::from_value::<TraceRecordDto>(payload)?.into());
        }
    }
    trace_store.ingest(graph, &records);
    Ok(records.len().into())
}

/// The additive runtime-trace overlay described in the module docs.
/// Starts empty; [`TraceStore::ingest`] merges batches of
/// [`TraceRecord`]s against a [`CodeGraph`] snapshot.
#[derive(Debug, Clone, Default)]
pub struct TraceStore {
    /// Keyed by (caller, callee) so re-ingestion naturally finds the
    /// existing entry to sum into (see module docs, "idempotent
    /// re-ingestion").
    // BRAND-INVARIANT: keys are exact caller/callee identities copied only from validated TraceRecord values; counts are accumulated TraceObservationCount values.
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
    /// - `caller` must resolve to a known graph node id (a runtime
    ///   caller is always a file or symbol that was actually indexed);
    /// - `callee` resolves if it is EITHER a known graph node id (a
    ///   trace exporter that reports resolved symbol ids, per
    ///   [`Self::edges`]'s "runtime-only edge" case) OR matches the raw
    ///   callee text of at least one parsed [`crate::code_graph::CallEdge`]
    ///   (a trace exporter that reports call targets the same
    ///   unresolved way the parser recorded them, per [`Self::edges`]'s
    ///   "annotate an existing parsed edge" case) -- both are legitimate
    ///   per this module's docs, so both count as resolved;
    /// - both sides resolve -> the runtime count for that pair is
    ///   summed into `runtime_counts` (see module docs on idempotent
    ///   re-ingestion);
    /// - either side fails to resolve -> the record is appended to
    ///   [`Self::unresolved`], never silently dropped, and its count is
    ///   NOT added to `runtime_counts` (an edge naming an unknown node
    ///   would be a dangling/fabricated edge if merged in).
    pub fn ingest(&mut self, graph: &CodeGraph, records: &[TraceRecord]) {
        let known_ids = known_node_ids(graph);
        let known_raw_callees = known_raw_callee_texts(graph);
        for record in records {
            let caller_id: TraceNodeId = record.caller.as_str().into();
            let callee_id: TraceNodeId = record.callee.as_str().into();
            let caller_known = known_ids.contains(&caller_id);
            let callee_known =
                known_ids.contains(&callee_id) || known_raw_callees.contains(&callee_id);
            if caller_known && callee_known {
                // CLONE-JUSTIFICATION: runtime-count map owns the key after borrowed record inspection.
                let key = (
                    record.caller.as_str().retained(),
                    record.callee.as_str().retained(),
                );
                *self.runtime_counts.entry(key).or_insert(0) += record.count.get();
            } else {
                // CLONE-JUSTIFICATION: unresolved evidence is retained independently of the input record.
                self.unresolved.push(UnresolvedTrace {
                    record: record.retained(),
                    unresolved_caller: (!caller_known).into(),
                    unresolved_callee: (!callee_known).into(),
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

    /// The merged edge view: every parsed [`crate::code_graph::CallEdge`]
    /// annotated with its runtime-observed count (0 if none), plus every
    /// runtime-only pair that has no matching parsed edge (provenance
    /// [`EdgeProvenance::Runtime`]).
    ///
    /// Matching is an EXACT `(caller, callee)` tuple match against
    /// [`crate::code_graph::CallEdge::from_file_id`]/`callee` as
    /// written by the parser -- deliberately not the fuzzy
    /// exact-name-or-trailing-segment resolution
    /// [`super::analysis::CodeAdjacency`] applies when building its
    /// traversal graph (that resolution lives in a private helper this
    /// module does not depend on, per this lane's file claims). A
    /// runtime `caller`/`callee` pair therefore only annotates an
    /// existing parsed edge when it names the callee exactly as the
    /// parser recorded it (typically the raw, unqualified call-site
    /// text); a resolved symbol id (`sym:...`) as `callee` will never
    /// match a parsed edge and always becomes its own
    /// [`EdgeProvenance::Runtime`] entry instead -- see
    /// [`tests::ingest_creates_a_runtime_only_edge_when_no_parsed_edge_exists`]
    /// for exactly this case. Callers that want resolved-id matching
    /// must resolve `callee` to the parser's raw text themselves before
    /// calling [`Self::ingest`].
    ///
    /// Deterministic ordering: sorted by `(caller, callee)`.
    pub fn edges(&self, graph: &CodeGraph) -> Vec<TracedEdge> {
        let mut merged: BTreeMap<(String, String), TracedEdge> = BTreeMap::new();

        for call in graph.calls() {
            // CLONE-JUSTIFICATION: merged edge map owns keys while graph calls stay borrowed.
            let key = (call.from_file_id.retained(), call.callee.retained());
            merged.entry(key.retained()).or_insert(TracedEdge {
                caller: call.from_file_id.retained().into(),
                callee: call.callee.as_str().retained().into(),
                provenance: EdgeProvenance::Parsed,
                observed_count: 0.into(),
            });
        }

        for ((caller, callee), count) in &self.runtime_counts {
            // CLONE-JUSTIFICATION: merged keys and runtime edge payloads have independent ownership.
            let key = (caller.retained(), callee.retained());
            match merged.get_mut(&key) {
                Some(edge) => edge.observed_count = (*count).into(),
                None => {
                    merged.insert(
                        key,
                        TracedEdge {
                            // CLONE-JUSTIFICATION: stored runtime edge owns caller/callee beyond borrowed count-map iteration.
                            caller: caller.retained().into(),
                            callee: callee.retained().into(),
                            provenance: EdgeProvenance::Runtime,
                            observed_count: (*count).into(),
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
fn known_node_ids(graph: &CodeGraph) -> std::collections::HashSet<TraceNodeId> {
    graph.nodes().iter().map(|node| node.id().into()).collect()
}

/// Every raw callee string appearing in `graph.calls()` (the parser's
/// as-written call-site text, e.g. `"helper"` -- never a resolved
/// `sym:...` id). A [`TraceRecord::callee`] that names one of these
/// values is resolvable even though it is not itself a graph node id,
/// because [`TraceStore::edges`] merges runtime counts into parsed
/// edges by exactly this text, not by node id (see its doc comment).
fn known_raw_callee_texts(graph: &CodeGraph) -> std::collections::HashSet<TraceNodeId> {
    graph
        .calls()
        .iter()
        .map(|call| call.callee.as_str().into())
        .collect()
}
