//! Filesystem, JSON, and zstd adapter for graph-artifact persistence.
//!
//! BOUNDARY-INVARIANT: serialized artifact fields are decoded into canonical
//! memory-domain values before a graph is reconstructed; malformed bytes fail closed.

use std::io::Write as _;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::artifacts::{GraphArtifactError, GRAPH_ARTIFACT_COMPRESSION_LEVEL};
use enforcer_domain::memory_types::{
    GraphArtifactByteCount, GraphArtifactSchemaVersion, GraphChangeCount, GraphCompressionLevel,
    GraphEdgeCount, GraphNodeCount, GraphShingleSize, GraphSnapshotBodyGram, GraphSnapshotCallee,
    GraphSnapshotChunkId, GraphSnapshotCommit, GraphSnapshotFingerprint, GraphSnapshotModulePath,
    GraphSnapshotNodeId, GraphSnapshotRelativePath, GraphSnapshotRouteMethod,
    GraphSnapshotRoutePath, GraphSnapshotSymbolName, GraphSourceLine, GraphSymbolKindSnapshot,
    GraphTextOnly, SourceHash,
};

// ROUNDTRIP-TEST: unit_artifacts::export_then_import_reconstructs_identical_node_and_edge_counts
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphSnapshotDto {
    pub files: Vec<GraphFileSnapshotDto>,
    pub symbols: Vec<GraphSymbolSnapshotDto>,
    pub tombstones: Vec<GraphTombstoneSnapshotDto>,
    pub imports: Vec<ImportEdgeSnapshotDto>,
    pub calls: Vec<CallEdgeSnapshotDto>,
    pub routes: Vec<RouteEdgeSnapshotDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphFileSnapshotDto {
    pub id: GraphSnapshotNodeId,
    pub rel_path: GraphSnapshotRelativePath,
    pub text_only: GraphTextOnly,
    pub content_hash: SourceHash,
    pub last_commit: Option<GraphSnapshotCommit>,
    pub change_count: GraphChangeCount,
    pub chunk_ids: Vec<GraphSnapshotChunkId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphSymbolSnapshotDto {
    pub id: GraphSnapshotNodeId,
    pub kind: GraphSymbolKindSnapshot,
    pub name: GraphSnapshotSymbolName,
    pub file_id: GraphSnapshotNodeId,
    pub line: GraphSourceLine,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_body_fingerprint: Option<GraphSourceBodyFingerprintSnapshotDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphSourceBodyFingerprintSnapshotDto {
    pub source_hash: SourceHash,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fp: Option<GraphSnapshotFingerprint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub k: Option<GraphShingleSize>,
    pub body_grams: Vec<GraphSnapshotBodyGram>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphTombstoneSnapshotDto {
    pub id: GraphSnapshotNodeId,
    pub rel_path: GraphSnapshotRelativePath,
    pub last_commit: Option<GraphSnapshotCommit>,
    pub change_count: GraphChangeCount,
    pub prior_chunk_ids: Vec<GraphSnapshotChunkId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportEdgeSnapshotDto {
    pub from_file_id: GraphSnapshotNodeId,
    pub module_path: GraphSnapshotModulePath,
    pub line: GraphSourceLine,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallEdgeSnapshotDto {
    pub from_file_id: GraphSnapshotNodeId,
    pub callee: GraphSnapshotCallee,
    pub line: GraphSourceLine,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteEdgeSnapshotDto {
    pub from_file_id: GraphSnapshotNodeId,
    pub method: GraphSnapshotRouteMethod,
    pub path: GraphSnapshotRoutePath,
    pub line: GraphSourceLine,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactMetadataDto {
    pub schema_version: GraphArtifactSchemaVersion,
    pub(crate) commit: ArtifactCommitDto,
    pub(crate) indexed_at: ArtifactIndexedAtDto,
    pub(crate) project: ArtifactProjectDto,
    pub nodes: GraphNodeCount,
    pub edges: GraphEdgeCount,
    pub original_size: GraphArtifactByteCount,
    pub compressed_size: GraphArtifactByteCount,
    pub compression_level: GraphCompressionLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct ArtifactCommitDto(pub(crate) Option<String>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct ArtifactIndexedAtDto(pub(crate) String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct ArtifactProjectDto(pub(crate) String);

pub(crate) fn encode(snapshot: &GraphSnapshotDto) -> Result<Vec<u8>, GraphArtifactError> {
    let json = serde_json::to_vec(snapshot)?;
    let mut encoder = zstd::Encoder::new(Vec::new(), i32::from(GRAPH_ARTIFACT_COMPRESSION_LEVEL))
        .map_err(GraphArtifactError::Compression)?;
    encoder
        .write_all(&json)
        .map_err(GraphArtifactError::Compression)?;
    encoder.finish().map_err(GraphArtifactError::Compression)
}

pub(crate) fn decode(bytes: &[u8]) -> Result<GraphSnapshotDto, GraphArtifactError> {
    let json = zstd::decode_all(bytes).map_err(GraphArtifactError::Decompression)?;
    serde_json::from_slice(&json).map_err(GraphArtifactError::Json)
}

pub(crate) fn encoded_snapshot_size(
    snapshot: &GraphSnapshotDto,
) -> Result<enforcer_domain::memory_types::GraphArtifactByteCount, GraphArtifactError> {
    serde_json::to_vec(snapshot)
        .map_err(GraphArtifactError::Json)?
        .len()
        .try_into()
        .map_err(|_size_error| GraphArtifactError::ArtifactTooLarge)
}

pub(crate) fn encode_metadata(
    metadata: &ArtifactMetadataDto,
) -> Result<Vec<u8>, GraphArtifactError> {
    serde_json::to_vec_pretty(metadata).map_err(GraphArtifactError::Json)
}

pub(crate) fn read_metadata(path: &Path) -> Result<ArtifactMetadataDto, GraphArtifactError> {
    let raw = std::fs::read_to_string(path).map_err(|source| GraphArtifactError::Io {
        path: path.to_path_buf().into(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(GraphArtifactError::Json)
}
