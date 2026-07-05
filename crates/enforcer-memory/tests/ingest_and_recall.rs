//! Integration test: ingest a fixture NDJSON corpus + a fixture lesson
//! ledger into a [`enforcer_memory::MemoryGraph`], and prove recall
//! returns the expected record(s) for a query, and the anti-vacuous
//! case (a query with no match returns empty, not everything).

use enforcer_memory::{ingest_ndjson_into, parse_ledger, recall, MemoryGraph};

fn load_fixture_graph() -> MemoryGraph {
    let ndjson = include_str!("fixtures/memory/sample.ndjson");
    let ledger = include_str!("fixtures/memory/sample-ledger.md");

    let mut graph = MemoryGraph::new();
    let ingested = ingest_ndjson_into(&mut graph, ndjson).expect("fixture ndjson must parse");
    assert_eq!(ingested, 3, "fixture corpus has exactly 3 records");

    for row in parse_ledger(ledger) {
        graph.ingest_lesson_row(row);
    }

    graph
}

#[test]
fn ingests_full_fixture_corpus() {
    let graph = load_fixture_graph();
    // 3 ndjson records + 2 ledger rows.
    assert_eq!(graph.len(), 5);
}

#[test]
fn recall_returns_expected_record_for_targeted_query() {
    let graph = load_fixture_graph();

    let hits = recall(&graph, "idempotent");
    let ids: Vec<&str> = hits.iter().map(|hit| hit.node.id()).collect();
    // Both the ndjson lesson record and the ledger row L1 discuss
    // idempotent init -- both must come back.
    assert!(ids.contains(&"mem-fixture-0002"), "expected the idempotent-init memory record, got {ids:?}");
    assert!(ids.contains(&"L1"), "expected the idempotent-init ledger row, got {ids:?}");
}

#[test]
fn recall_targeted_query_excludes_unrelated_records() {
    let graph = load_fixture_graph();
    let hits = recall(&graph, "idempotent");
    let ids: Vec<&str> = hits.iter().map(|hit| hit.node.id()).collect();
    assert!(!ids.contains(&"mem-fixture-0001"), "recommend-and-proceed record must not match 'idempotent'");
}

#[test]
fn recall_with_no_match_returns_empty_not_everything() {
    let graph = load_fixture_graph();
    let hits = recall(&graph, "xyzzy-nonexistent-token-12345");
    assert!(hits.is_empty(), "a query matching nothing must return empty, not the whole corpus");
}

#[test]
fn rejects_malformed_ndjson_line_instead_of_dropping_it() {
    let mut graph = MemoryGraph::new();
    let bad = "{ this is not valid json }\n";
    let result = ingest_ndjson_into(&mut graph, bad);
    assert!(result.is_err(), "a corrupt line in an append-only log must be a hard error");
    assert!(graph.is_empty(), "no partial ingest on error");
}
