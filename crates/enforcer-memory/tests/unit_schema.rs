use enforcer_memory::schema::{GraphEventKind, IndexManifest, ObservationLogEntry, SCHEMA_VERSION};

#[test]
fn observation_entry_round_trips() -> Result<(), serde_json::Error> {
    let entry = ObservationLogEntry {
        schema_version: SCHEMA_VERSION,
        seq: 0,
        id: "obs-scan-0000".to_owned(),
        lesson_id: "L1".to_owned(),
        rule_id: None,
        fault_class: None,
        repo_context: "crates/enforcer-memory".to_owned(),
        clean: true,
        source_surface: "scan".to_owned(),
        ts: "2026-07-04T00:00:00Z".to_owned(),
        supersedes_seq: None,
        payload_kind: None,
        payload: None,
    };
    let json = serde_json::to_string(&entry)?;
    let back: ObservationLogEntry = serde_json::from_str(&json)?;
    assert_eq!(entry, back);
    Ok(())
}

#[test]
fn graph_event_kind_tags_on_wire() -> Result<(), serde_json::Error> {
    let node = GraphEventKind::NodeAdded {
        node_id: "n1".to_owned(),
        node_kind: "file".to_owned(),
    };
    let json = serde_json::to_string(&node)?;
    let value: serde_json::Value = serde_json::from_str(&json)?;
    assert_eq!(value["kind"].as_str(), Some("nodeAdded"));
    Ok(())
}

#[test]
fn index_manifest_round_trips() -> Result<(), serde_json::Error> {
    let manifest = IndexManifest {
        schema_version: SCHEMA_VERSION,
        source_log: "observation".to_owned(),
        source_high_watermark: 42,
        built_at: "2026-07-04T00:00:00Z".to_owned(),
    };
    let json = serde_json::to_string(&manifest)?;
    let back: IndexManifest = serde_json::from_str(&json)?;
    assert_eq!(manifest, back);
    Ok(())
}
