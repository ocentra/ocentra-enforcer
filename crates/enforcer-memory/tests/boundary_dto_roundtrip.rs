use std::error::Error;

use enforcer_domain::memory_types::{
    LocalRuntimeAcceleration, LocalRuntimeArtifactKind, LocalRuntimeKind, ModelTask,
};
use enforcer_memory::boundary::artifact_transport::{
    ArtifactMetadataDto, CallEdgeSnapshotDto, GraphFileSnapshotDto, GraphSnapshotDto,
    GraphSourceBodyFingerprintSnapshotDto, GraphSymbolSnapshotDto, GraphTombstoneSnapshotDto,
    ImportEdgeSnapshotDto, RouteEdgeSnapshotDto,
};
use enforcer_memory::boundary::model_cache::{
    ModelCacheArtifactEntryDto, ModelCacheManifestDto, ModelCacheValidationDto,
};
use enforcer_memory::model_cache::{validate_model_cache_manifest, MODEL_CACHE_SCHEMA_VERSION};
use serde::{de::DeserializeOwned, Serialize};

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
fn artifact_transport_dtos_round_trip_from_external_json() -> Result<(), Box<dyn Error>> {
    let source_hash = "ab".repeat(32);
    let file: GraphFileSnapshotDto = serde_json::from_value(serde_json::json!({
        "id": "file:a.rs", "rel_path": "a.rs", "text_only": false,
        "content_hash": source_hash, "last_commit": null, "change_count": 1,
        "chunk_ids": []
    }))?;
    let symbol: GraphSymbolSnapshotDto = serde_json::from_value(serde_json::json!({
        "id": "symbol:a", "kind": "Function", "name": "a",
        "file_id": "file:a.rs", "line": 1,
        "source_body_fingerprint": null
    }))?;
    let fingerprint: GraphSourceBodyFingerprintSnapshotDto =
        serde_json::from_value(serde_json::json!({
            "source_hash": "ab".repeat(32), "fp": null, "k": null, "body_grams": []
        }))?;
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
    let metadata: ArtifactMetadataDto = serde_json::from_value(serde_json::json!({
        "schema_version": 2, "commit": null, "indexed_at": "2026-07-17T00:00:00Z",
        "project": "demo", "nodes": 2, "edges": 3, "original_size": 100,
        "compressed_size": 50, "compression_level": 3
    }))?;
    let snapshot = GraphSnapshotDto {
        files: vec![file.clone()],
        symbols: vec![symbol.clone()],
        tombstones: vec![tombstone.clone()],
        imports: vec![import.clone()],
        calls: vec![call.clone()],
        routes: vec![route.clone()],
    };

    assert_json_round_trip(&snapshot)?;
    assert_json_round_trip(&file)?;
    assert_json_round_trip(&symbol)?;
    assert_json_round_trip(&fingerprint)?;
    assert_json_round_trip(&tombstone)?;
    assert_json_round_trip(&import)?;
    assert_json_round_trip(&call)?;
    assert_json_round_trip(&route)?;
    assert_json_round_trip(&metadata)?;
    Ok(())
}

#[test]
fn model_cache_dtos_round_trip_from_validated_manifest() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let model_path = temp.path().join("model.gguf");
    std::fs::write(&model_path, b"model-bytes")?;
    let artifact = ModelCacheArtifactEntryDto {
        kind: Some(LocalRuntimeArtifactKind::Model),
        path: "model.gguf".to_owned().into(),
        sha256: enforcer_memory::model_runtime::sha256_file(&model_path)?.into(),
        size_bytes: None,
        streaming_manifest_path: None,
    };
    let manifest = ModelCacheManifestDto {
        schema_version: MODEL_CACHE_SCHEMA_VERSION.into(),
        backend: LocalRuntimeKind::LlamaCpp,
        task: ModelTask::Embedding,
        model_id: "local-model".to_owned().into(),
        revision: "local".to_owned().into(),
        acceleration: LocalRuntimeAcceleration::Cpu,
        artifacts: vec![artifact.clone()],
    };
    let manifest_path = temp.path().join("manifest.json");
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest)?)?;
    let validation: ModelCacheValidationDto = validate_model_cache_manifest(&manifest_path)?;

    assert_json_round_trip(&artifact)?;
    assert_json_round_trip(&manifest)?;
    assert_json_round_trip(&validation)?;
    Ok(())
}
