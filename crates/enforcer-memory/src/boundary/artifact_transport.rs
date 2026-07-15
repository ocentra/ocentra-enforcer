//! Filesystem, JSON, and zstd adapter for graph-artifact persistence.

use std::io::Write as _;
use std::path::Path;

use crate::artifacts::{
    ArtifactMetadata, GraphArtifactError, GraphSnapshot, GRAPH_ARTIFACT_COMPRESSION_LEVEL,
};

pub(crate) fn encode(snapshot: &GraphSnapshot) -> Result<Vec<u8>, GraphArtifactError> {
    let json = serde_json::to_vec(snapshot)?;
    let mut encoder = zstd::Encoder::new(Vec::new(), GRAPH_ARTIFACT_COMPRESSION_LEVEL)
        .map_err(GraphArtifactError::Compression)?;
    encoder
        .write_all(&json)
        .map_err(GraphArtifactError::Compression)?;
    encoder.finish().map_err(GraphArtifactError::Compression)
}

pub(crate) fn decode(bytes: &[u8]) -> Result<GraphSnapshot, GraphArtifactError> {
    let json = zstd::decode_all(bytes).map_err(GraphArtifactError::Decompression)?;
    serde_json::from_slice(&json).map_err(GraphArtifactError::Json)
}

pub(crate) fn read_metadata(path: &Path) -> Result<ArtifactMetadata, GraphArtifactError> {
    let raw = std::fs::read_to_string(path).map_err(|source| GraphArtifactError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(GraphArtifactError::Json)
}
