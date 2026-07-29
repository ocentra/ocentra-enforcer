//! Integration coverage for `enforcer_memory::artifacts`, moved out of
//! the source module's inline `#[cfg(test)]` block.

use enforcer_domain::memory_types::ArtifactId;
use enforcer_memory::artifacts::{
    artifact_dir, artifact_exists, export_graph_artifact, get_exact, get_snippet_exact,
    import_graph_artifact, ArtifactLookupError, GraphArtifactError, GRAPH_ARTIFACT_FILENAME,
    GRAPH_ARTIFACT_META_FILENAME,
};
use enforcer_memory::boundary::artifact_transport::{
    ArtifactMetadataDto, CallEdgeSnapshotDto, GraphFileSnapshotDto, GraphSnapshotDto,
    GraphSourceBodyFingerprintSnapshotDto, GraphSymbolSnapshotDto, GraphTombstoneSnapshotDto,
    ImportEdgeSnapshotDto, RouteEdgeSnapshotDto,
};
use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::store::manifest::ArtifactManifest;
use serde::{de::DeserializeOwned, Serialize};
use std::error::Error;
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let unique = format!(
        "enforcer-memory-artifacts-{}-{}-{name}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    );
    std::env::temp_dir().join(unique)
}

fn sample_graph() -> Result<CodeGraph, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(dir.path())
        .status()?;
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir.path())
        .status()?;
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .status()?;
    let file_path = dir.path().join("a.rs");
    std::fs::write(&file_path, "fn a() { let _ = 1; }\nfn b() { let _ = 2; }\n")?;
    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir.path())
        .status()?;
    std::process::Command::new("git")
        .args(["commit", "--quiet", "-m", "first"])
        .current_dir(dir.path())
        .status()?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;
    Ok(graph)
}

fn assert_json_round_trip<T>(value: &T) -> Result<(), serde_json::Error>
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let encoded = serde_json::to_vec(value)?;
    let decoded: T = serde_json::from_slice(&encoded)?;
    assert_eq!(&decoded, value);
    Ok(())
}

#[test]
fn exact_id_returns_exact_content() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("exact-hit");
    let mut manifest = ArtifactManifest::open(&root)?;
    let id = manifest.put(
        b"hello artifact",
        Some("a.txt".into()),
        "2026-07-05T00:00:00Z",
    )?;

    let content = get_exact(&manifest, &id)?;
    assert_eq!(content, b"hello artifact");

    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn wrong_id_is_fail_closed_not_similar() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("wrong-id");
    let mut manifest = ArtifactManifest::open(&root)?;
    manifest.put(
        b"artifact one",
        Some("a.txt".into()),
        "2026-07-05T00:00:00Z",
    )?;
    manifest.put(
        b"artifact two",
        Some("b.txt".into()),
        "2026-07-05T00:00:01Z",
    )?;

    let digest = format!("sha256:{}", "ab".repeat(32)).parse()?;
    let unknown = ArtifactId::from_digest(digest);
    let outcome = get_exact(&manifest, &unknown);
    assert!(
        matches!(outcome, Err(ArtifactLookupError::NotFound { .. })),
        "unknown exact id must fail closed, never substitute a similar artifact"
    );

    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn snippet_exact_shares_the_same_fail_closed_contract() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("snippet");
    let mut manifest = ArtifactManifest::open(&root)?;
    let id = manifest.put(
        b"fn snippet() { let _ = 1; }",
        Some("snip.rs".into()),
        "2026-07-05T00:00:00Z",
    )?;
    let content = get_snippet_exact(&manifest, &id)?;
    assert_eq!(content, b"fn snippet() { let _ = 1; }");

    let digest = format!("sha256:{}", "ef".repeat(32)).parse()?;
    let unknown = ArtifactId::from_digest(digest);
    let outcome = get_snippet_exact(&manifest, &unknown);
    assert!(matches!(outcome, Err(ArtifactLookupError::NotFound { .. })));
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn export_then_import_reconstructs_identical_node_and_edge_counts() -> Result<(), Box<dyn Error>> {
    let graph = sample_graph()?;
    let snapshot = GraphSnapshotDto::from_code_graph(&graph)?;
    let root = temp_dir("export-import");
    std::fs::create_dir_all(&root)?;

    export_graph_artifact(
        &root,
        &snapshot,
        "demo",
        Some("deadbeef".into()),
        "2026-07-05T00:00:00Z",
    )?;
    let (imported, meta) = import_graph_artifact(&root)?;

    assert_eq!(imported.node_count(), snapshot.node_count());
    assert_eq!(imported.edge_count(), snapshot.edge_count());
    assert_eq!(meta.nodes, snapshot.node_count());
    assert_eq!(meta.edges, snapshot.edge_count());

    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn corrupted_compressed_artifact_is_rejected() -> Result<(), Box<dyn Error>> {
    let graph = sample_graph()?;
    let snapshot = GraphSnapshotDto::from_code_graph(&graph)?;
    let root = temp_dir("corrupt-compressed-artifact");
    std::fs::create_dir_all(&root)?;

    export_graph_artifact(&root, &snapshot, "demo", None, "2026-07-17T00:00:00Z")?;
    let artifact_path = artifact_dir(&root).join(GRAPH_ARTIFACT_FILENAME);
    std::fs::write(&artifact_path, b"not-a-zstd-frame")?;

    let outcome = import_graph_artifact(&root);
    assert!(matches!(outcome, Err(GraphArtifactError::Decompression(_))));

    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn artifact_transport_dtos_round_trip_with_their_wire_contracts() -> Result<(), Box<dyn Error>> {
    let snapshot = GraphSnapshotDto::from_code_graph(&sample_graph()?)?;
    let file: GraphFileSnapshotDto = snapshot.files[0].clone();
    let symbol: GraphSymbolSnapshotDto = snapshot.symbols[0].clone();
    let fingerprint = GraphSourceBodyFingerprintSnapshotDto {
        source_hash: file.content_hash.clone(),
        fp: None,
        k: None,
        body_grams: Vec::new(),
    };
    let tombstone: GraphTombstoneSnapshotDto = serde_json::from_value(serde_json::json!({
        "id": "file:removed.rs", "rel_path": "removed.rs", "last_commit": null,
        "change_count": 1, "prior_chunk_ids": []
    }))?;
    let import: ImportEdgeSnapshotDto = serde_json::from_value(serde_json::json!({
        "from_file_id": "file:a.rs", "module_path": "crate::dependency", "line": 1
    }))?;
    let call: CallEdgeSnapshotDto = serde_json::from_value(serde_json::json!({
        "from_file_id": "file:a.rs", "callee": "crate::callee", "line": 2
    }))?;
    let route: RouteEdgeSnapshotDto = serde_json::from_value(serde_json::json!({
        "from_file_id": "file:a.rs", "method": "GET", "path": "/health", "line": 3
    }))?;

    let root = temp_dir("dto-round-trip");
    std::fs::create_dir_all(&root)?;
    export_graph_artifact(&root, &snapshot, "demo", None, "2026-07-05T00:00:00Z")?;
    let (_, metadata): (_, ArtifactMetadataDto) = import_graph_artifact(&root)?;

    assert_json_round_trip(&file)?;
    assert_json_round_trip(&symbol)?;
    assert_json_round_trip(&fingerprint)?;
    assert_json_round_trip(&tombstone)?;
    assert_json_round_trip(&import)?;
    assert_json_round_trip(&call)?;
    assert_json_round_trip(&route)?;
    assert_json_round_trip(&metadata)?;

    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn exporting_over_an_existing_artifact_fails_without_overwriting() -> Result<(), Box<dyn Error>> {
    let graph = sample_graph()?;
    let snapshot = GraphSnapshotDto::from_code_graph(&graph)?;
    let root = temp_dir("export-refuses-overwrite");
    std::fs::create_dir_all(&root)?;

    export_graph_artifact(&root, &snapshot, "original", None, "2026-07-05T00:00:00Z")?;
    let artifact_path = artifact_dir(&root).join(GRAPH_ARTIFACT_FILENAME);
    let metadata_path = artifact_dir(&root).join(GRAPH_ARTIFACT_META_FILENAME);
    let artifact_before = std::fs::read(&artifact_path)?;
    let metadata_before = std::fs::read(&metadata_path)?;

    let outcome = export_graph_artifact(
        &root,
        &snapshot,
        "replacement",
        Some("different-commit".into()),
        "2026-07-06T00:00:00Z",
    );
    assert!(matches!(
        outcome,
        Err(GraphArtifactError::AlreadyExists { path }) if path == artifact_path
    ));
    assert_eq!(std::fs::read(&artifact_path)?, artifact_before);
    assert_eq!(std::fs::read(&metadata_path)?, metadata_before);

    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn artifact_json_has_exactly_the_baseline_field_set() -> Result<(), Box<dyn Error>> {
    let graph = sample_graph()?;
    let snapshot = GraphSnapshotDto::from_code_graph(&graph)?;
    let root = temp_dir("field-parity");
    std::fs::create_dir_all(&root)?;

    export_graph_artifact(&root, &snapshot, "demo", None, "2026-07-05T00:00:00Z")?;
    let meta_path = artifact_dir(&root).join(GRAPH_ARTIFACT_META_FILENAME);
    let raw = std::fs::read_to_string(&meta_path)?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    let obj = value.as_object().ok_or("artifact.json must be an object")?;

    let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
    keys.sort_unstable();
    let mut expected = vec![
        "schema_version",
        "commit",
        "indexed_at",
        "project",
        "nodes",
        "edges",
        "original_size",
        "compressed_size",
        "compression_level",
    ];
    expected.sort_unstable();
    assert_eq!(keys, expected, "artifact.json field set must match exactly");
    assert_eq!(obj["schema_version"], serde_json::json!(2));
    assert_eq!(obj["commit"], serde_json::Value::Null);
    assert_eq!(obj["indexed_at"], serde_json::json!("2026-07-05T00:00:00Z"));
    assert_eq!(obj["project"], serde_json::json!("demo"));

    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn artifact_exists_is_false_until_export_then_true() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("exists-flag");
    std::fs::create_dir_all(&root)?;
    assert!(!bool::from(artifact_exists(&root)));

    let graph = sample_graph()?;
    let snapshot = GraphSnapshotDto::from_code_graph(&graph)?;
    export_graph_artifact(&root, &snapshot, "demo", None, "2026-07-05T00:00:00Z")?;
    assert!(artifact_exists(&root).is_present());

    std::fs::remove_dir_all(&root)?;
    Ok(())
}
