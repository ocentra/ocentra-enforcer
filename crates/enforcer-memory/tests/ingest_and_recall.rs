//! Integration test: ingest a fixture NDJSON corpus + a fixture lesson
//! ledger into a [`enforcer_memory::MemoryGraph`], and prove recall
//! returns the expected record(s) for a query, and the anti-vacuous
//! case (a query with no match returns empty, not everything).

use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::ingest::ingest_ndjson_into;
use enforcer_memory::lesson::parse_ledger;
use enforcer_memory::recall::recall;

fn load_fixture_graph() -> Result<MemoryGraph, Box<dyn std::error::Error>> {
    let ndjson = include_str!("fixtures/memory/sample.ndjson");
    let ledger = include_str!("fixtures/memory/sample-ledger.md");

    let mut graph = MemoryGraph::new();
    let ingested = ingest_ndjson_into(&mut graph, ndjson)?;
    assert_eq!(ingested, 3, "fixture corpus has exactly 3 records");

    for row in parse_ledger(ledger) {
        graph.ingest_lesson_row(row);
    }

    Ok(graph)
}

#[test]
fn ingests_full_fixture_corpus() -> Result<(), Box<dyn std::error::Error>> {
    let graph = load_fixture_graph()?;
    // 3 ndjson records + 2 ledger rows.
    assert_eq!(graph.len(), 5);
    Ok(())
}

#[test]
fn recall_returns_expected_record_for_targeted_query() -> Result<(), Box<dyn std::error::Error>> {
    let graph = load_fixture_graph()?;

    let hits = recall(&graph, "idempotent");
    let ids: Vec<String> = hits.iter().map(|hit| hit.node.id().to_string()).collect();
    // Both the ndjson lesson record and the ledger row L1 discuss
    // idempotent init -- both must come back.
    assert!(
        ids.iter().any(|id| id == "mem-fixture-0002"),
        "expected the idempotent-init memory record, got {ids:?}"
    );
    assert!(
        ids.iter().any(|id| id == "L1"),
        "expected the idempotent-init ledger row, got {ids:?}"
    );
    Ok(())
}

#[test]
fn recall_targeted_query_excludes_unrelated_records() -> Result<(), Box<dyn std::error::Error>> {
    let graph = load_fixture_graph()?;
    let hits = recall(&graph, "idempotent");
    let ids: Vec<String> = hits.iter().map(|hit| hit.node.id().to_string()).collect();
    assert!(
        !ids.iter().any(|id| id == "mem-fixture-0001"),
        "recommend-and-proceed record must not match 'idempotent'"
    );
    Ok(())
}

#[test]
fn recall_with_no_match_returns_empty_not_everything() -> Result<(), Box<dyn std::error::Error>> {
    let graph = load_fixture_graph()?;
    let hits = recall(&graph, "xyzzy-nonexistent-token-12345");
    assert!(
        hits.is_empty(),
        "a query matching nothing must return empty, not the whole corpus"
    );
    Ok(())
}

#[test]
fn rejects_malformed_ndjson_line_instead_of_dropping_it() {
    let mut graph = MemoryGraph::new();
    let bad = "{ this is not valid json }\n";
    let result = ingest_ndjson_into(&mut graph, bad);
    assert!(
        result.is_err(),
        "a corrupt line in an append-only log must be a hard error"
    );
    assert!(graph.is_empty().is_empty(), "no partial ingest on error");
}
