use enforcer_memory::error::Result;
use enforcer_memory::schema::{GraphEventKind, GraphEventLogEntry, SCHEMA_VERSION};
use enforcer_memory::store::sqlite::OperationalGraph;

fn node_entry(seq: u64, id: &str, kind: &str) -> GraphEventLogEntry {
    GraphEventLogEntry {
        schema_version: SCHEMA_VERSION,
        seq,
        id: format!("evt-{seq}"),
        event: GraphEventKind::NodeAdded {
            node_id: id.to_owned(),
            node_kind: kind.to_owned(),
        },
        ts: "2026-07-04T00:00:00Z".to_owned(),
        supersedes_seq: None,
    }
}

fn edge_entry(seq: u64, from: &str, to: &str, label: &str) -> GraphEventLogEntry {
    GraphEventLogEntry {
        schema_version: SCHEMA_VERSION,
        seq,
        id: format!("evt-{seq}"),
        event: GraphEventKind::EdgeAdded {
            from: from.to_owned(),
            to: to.to_owned(),
            label: label.to_owned(),
        },
        ts: "2026-07-04T00:00:00Z".to_owned(),
        supersedes_seq: None,
    }
}

#[test]
fn apply_and_counts() -> Result<()> {
    let mut graph = OperationalGraph::open_in_memory()?;
    graph.apply(&node_entry(0, "a", "file"))?;
    graph.apply(&node_entry(1, "b", "file"))?;
    graph.apply(&edge_entry(2, "a", "b", "imports"))?;
    assert_eq!(graph.node_count()?, 2);
    assert_eq!(graph.edge_count()?, 1);
    Ok(())
}

#[test]
fn rebuild_is_deterministic() -> Result<()> {
    let entries = vec![
        node_entry(0, "a", "file"),
        node_entry(1, "b", "file"),
        edge_entry(2, "a", "b", "imports"),
        node_entry(3, "c", "symbol"),
    ];
    let mut first = OperationalGraph::open_in_memory()?;
    first.rebuild(&entries)?;
    let mut second = OperationalGraph::open_in_memory()?;
    second.rebuild(&entries)?;
    assert_eq!(first.nodes_snapshot()?, second.nodes_snapshot()?);
    assert_eq!(first.node_count()?, second.node_count()?);
    assert_eq!(first.edge_count()?, second.edge_count()?);

    // Rebuilding a THIRD time by replaying twice into the same
    // database (idempotent apply) must not change the counts.
    first.rebuild(&entries)?;
    assert_eq!(first.node_count()?, 3);
    assert_eq!(first.edge_count()?, 1);
    Ok(())
}

#[test]
fn later_seq_supersedes_node_kind_for_the_same_id() -> Result<()> {
    let mut graph = OperationalGraph::open_in_memory()?;
    graph.apply(&node_entry(0, "a", "file"))?;
    graph.apply(&node_entry(1, "a", "symbol"))?;
    let snapshot = graph.nodes_snapshot()?;
    assert_eq!(snapshot, vec![("a".to_owned(), "symbol".to_owned())]);
    Ok(())
}
