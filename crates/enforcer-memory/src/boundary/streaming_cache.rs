//! JSON transport for streaming-cache persistence manifests.
//!
//! BOUNDARY-INVARIANT: every raw manifest field is validated and converted to
//! a canonical streaming-cache type; empty and zero-valued invariants fail closed.

use serde::{Deserialize, Serialize};

use enforcer_domain::memory_types::{
    StreamingArtifactKey, StreamingArtifactStatus, StreamingByteCount, StreamingCacheSchemaVersion,
    StreamingChunkCount, StreamingRelativePath,
};

use crate::error::{MemoryError, Result};
use crate::streaming_cache::{StreamingCacheManifest, StreamingCacheManifestParts};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StreamingCacheManifestWire {
    schema_version: u32,
    artifact_id: String,
    relative_path: String,
    total_chunks: u64,
    chunk_size: u64,
    total_size: u64,
    chunks_dir: String,
    status: StreamingArtifactStatusWire,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum StreamingArtifactStatusWire {
    Ready,
}

pub(crate) fn decode_manifest(text: &str) -> Result<StreamingCacheManifest> {
    let wire: StreamingCacheManifestWire = serde_json::from_str(text)?;
    let artifact_id = StreamingArtifactKey::try_new(wire.artifact_id)
        .ok_or_else(|| invalid_manifest("artifact id is empty"))?;
    let relative_path = StreamingRelativePath::try_new(wire.relative_path)
        .ok_or_else(|| invalid_manifest("relative path is empty"))?;
    let chunks_dir = StreamingRelativePath::try_new(wire.chunks_dir)
        .ok_or_else(|| invalid_manifest("chunks directory is empty"))?;
    let schema_version = std::num::NonZeroU32::new(wire.schema_version)
        .map(StreamingCacheSchemaVersion::try_new)
        .ok_or_else(|| invalid_manifest("schema version must be non-zero"))?;
    let total_chunks = std::num::NonZeroU64::new(wire.total_chunks)
        .map(StreamingChunkCount::try_new)
        .ok_or_else(|| invalid_manifest("total chunks must be non-zero"))?;
    let chunk_size = std::num::NonZeroU64::new(wire.chunk_size)
        .map(StreamingByteCount::try_new)
        .ok_or_else(|| invalid_manifest("chunk size must be non-zero"))?;
    let total_size = std::num::NonZeroU64::new(wire.total_size)
        .map(StreamingByteCount::try_new)
        .ok_or_else(|| invalid_manifest("total size must be non-zero"))?;
    Ok(StreamingCacheManifest::from_wire(
        StreamingCacheManifestParts {
            schema_version,
            artifact_id,
            relative_path,
            total_chunks,
            chunk_size,
            total_size,
            chunks_dir,
            status: StreamingArtifactStatus::Ready,
        },
    ))
}

pub(crate) fn encode_manifest(manifest: &StreamingCacheManifest) -> Result<String> {
    let wire = StreamingCacheManifestWire {
        schema_version: u32::from(manifest.schema_version),
        artifact_id: manifest.artifact_id.as_str().to_owned(),
        relative_path: manifest.relative_path.as_str().to_owned(),
        total_chunks: u64::from(manifest.total_chunks),
        chunk_size: u64::from(manifest.chunk_size),
        total_size: u64::from(manifest.total_size),
        chunks_dir: manifest.chunks_dir.as_str().to_owned(),
        status: StreamingArtifactStatusWire::Ready,
    };
    serde_json::to_string_pretty(&wire).map_err(Into::into)
}

fn invalid_manifest(reason: &'static str) -> MemoryError {
    MemoryError::ModelRuntime {
        operation: "decode-streaming-cache-manifest".into(),
        reason: reason.to_owned().into(),
    }
}
