//! Chunked model artifact cache for large ONNX/GGUF files.
//!
//! TabAgent's browser runtime learned that loading multi-GB model files as a
//! single blob is fragile. This module is the Rust-side equivalent: artifacts
//! can be copied into fixed-size chunks with a manifest and then read back as a
//! streaming `Read` implementation without assembling the whole file in memory.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Component, Path};

use crate::error::{MemoryError, Result};
use crate::owned_boundary::Retained;
use enforcer_domain::memory_types::{
    StreamingArtifactKey, StreamingArtifactStatus, StreamingByteCount, StreamingCacheChunkPath,
    StreamingCacheChunksDirectory, StreamingCacheManifestPath, StreamingCachePathSegment,
    StreamingCacheSchemaVersion, StreamingCacheSegmentInput, StreamingChunkAdvance,
    StreamingChunkCount, StreamingChunkDecision, StreamingChunkIndex, StreamingRelativePath,
};

pub const STREAMING_CACHE_SCHEMA_VERSION: u32 = 1;
pub const STREAMING_CHUNK_SIZE: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamingCacheManifest {
    pub schema_version: StreamingCacheSchemaVersion,
    pub artifact_id: StreamingArtifactKey,
    pub relative_path: StreamingRelativePath,
    pub total_chunks: StreamingChunkCount,
    pub chunk_size: StreamingByteCount,
    pub total_size: StreamingByteCount,
    pub chunks_dir: StreamingRelativePath,
    pub status: StreamingArtifactStatus,
}

pub(crate) struct StreamingCacheManifestParts {
    pub schema_version: StreamingCacheSchemaVersion,
    pub artifact_id: StreamingArtifactKey,
    pub relative_path: StreamingRelativePath,
    pub total_chunks: StreamingChunkCount,
    pub chunk_size: StreamingByteCount,
    pub total_size: StreamingByteCount,
    pub chunks_dir: StreamingRelativePath,
    pub status: StreamingArtifactStatus,
}

impl StreamingCacheManifest {
    pub(crate) fn from_wire(parts: StreamingCacheManifestParts) -> Self {
        Self {
            schema_version: parts.schema_version,
            artifact_id: parts.artifact_id,
            relative_path: parts.relative_path,
            total_chunks: parts.total_chunks,
            chunk_size: parts.chunk_size,
            total_size: parts.total_size,
            chunks_dir: parts.chunks_dir,
            status: parts.status,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamingCacheWriteReport {
    pub manifest: StreamingCacheManifest,
    pub manifest_path: StreamingCacheManifestPath,
}

pub fn should_chunk_file(size_bytes: StreamingByteCount) -> StreamingChunkDecision {
    if u64::from(size_bytes) > STREAMING_CHUNK_SIZE {
        StreamingChunkDecision::Chunked
    } else {
        StreamingChunkDecision::Direct
    }
}

pub fn stream_file_into_chunks(
    source_path: &Path,
    cache_root: &Path,
    artifact_id: &StreamingArtifactKey,
    relative_path: &StreamingRelativePath,
) -> Result<StreamingCacheWriteReport> {
    stream_file_into_chunks_with_size(
        source_path,
        cache_root,
        artifact_id,
        relative_path,
        StreamingByteCount::try_new(
            std::num::NonZeroU64::new(STREAMING_CHUNK_SIZE).unwrap_or(std::num::NonZeroU64::MIN),
        ),
    )
}

pub fn read_streaming_cache_manifest(path: &Path) -> Result<StreamingCacheManifest> {
    let text = std::fs::read_to_string(path).map_err(|source| MemoryError::Io {
        path: path.to_path_buf().into(),
        source,
    })?;
    let manifest = crate::boundary::streaming_cache::decode_manifest(&text)?;
    validate_manifest_shape(&manifest)?;
    Ok(manifest)
}

pub fn streaming_chunk_reader(
    manifest_path: &Path,
    manifest: &StreamingCacheManifest,
) -> Result<StreamingChunkReader> {
    validate_manifest_shape(manifest)?;
    let root = manifest_path.parent().ok_or_else(|| {
        model_error(
            "streaming-chunk-reader",
            "streaming manifest path has no parent directory",
        )
    })?;
    let chunks_dir = safe_join(root, &manifest.chunks_dir)?;
    StreamingChunkReader::new(chunks_dir, manifest.total_chunks)
}

pub fn assemble_chunks_to_file(
    manifest_path: &Path,
    manifest: &StreamingCacheManifest,
    output_path: &Path,
) -> Result<()> {
    let mut reader = streaming_chunk_reader(manifest_path, manifest)?;
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| MemoryError::Io {
            path: parent.to_path_buf().into(),
            source,
        })?;
    }
    let mut output = File::create(output_path).map_err(|source| MemoryError::Io {
        path: output_path.to_path_buf().into(),
        source,
    })?;
    std::io::copy(&mut reader, &mut output).map_err(|source| MemoryError::Io {
        path: output_path.to_path_buf().into(),
        source,
    })?;
    Ok(())
}

#[derive(Debug)]
pub struct StreamingChunkReader {
    chunks_dir: StreamingCacheChunksDirectory,
    total_chunks: StreamingChunkCount,
    next_chunk: StreamingChunkIndex,
    active: Option<File>,
}

impl StreamingChunkReader {
    fn new(
        chunks_dir: StreamingCacheChunksDirectory,
        total_chunks: StreamingChunkCount,
    ) -> Result<Self> {
        if u64::from(total_chunks) == 0 {
            return Err(model_error(
                "streaming-chunk-reader",
                "streaming manifest has zero chunks",
            ));
        }
        Ok(Self {
            chunks_dir,
            total_chunks,
            next_chunk: StreamingChunkIndex::ZERO,
            active: None,
        })
    }

    fn open_next_chunk(&mut self) -> std::io::Result<StreamingChunkAdvance> {
        if u64::from(self.next_chunk) >= u64::from(self.total_chunks) {
            return Ok(StreamingChunkAdvance::Exhausted);
        }
        let path = chunk_path(&self.chunks_dir, self.next_chunk);
        self.active = Some(File::open(path.as_path())?);
        self.next_chunk = self.next_chunk.next();
        Ok(StreamingChunkAdvance::Opened)
    }
}

impl Read for StreamingChunkReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            if self.active.is_none()
                && matches!(self.open_next_chunk()?, StreamingChunkAdvance::Exhausted)
            {
                return Ok(0);
            }
            if let Some(active) = self.active.as_mut() {
                let read = active.read(buf)?;
                if read > 0 {
                    return Ok(read);
                }
            }
            self.active = None;
        }
    }
}

pub fn stream_file_into_chunks_with_size(
    source_path: &Path,
    cache_root: &Path,
    artifact_id: &StreamingArtifactKey,
    relative_path: &StreamingRelativePath,
    chunk_size: StreamingByteCount,
) -> Result<StreamingCacheWriteReport> {
    let chunk_size_count = chunk_size;
    let chunk_size = u64::from(chunk_size);
    if chunk_size == 0 {
        return Err(model_error(
            "stream-file-into-chunks",
            "chunk size must be greater than zero",
        ));
    }
    validate_relative_path(relative_path)?;
    let total_size = source_path
        .metadata()
        .map_err(|source| MemoryError::Io {
            path: source_path.to_path_buf().into(),
            source,
        })?
        .len();
    if total_size == 0 {
        return Err(model_error(
            "stream-file-into-chunks",
            "cannot chunk an empty artifact",
        ));
    }
    let total_size_count = StreamingByteCount::try_new(
        std::num::NonZeroU64::new(total_size).unwrap_or(std::num::NonZeroU64::MIN),
    );

    let artifact_segment = safe_segment(artifact_id.as_str().into());
    let path_segment = safe_segment(relative_path.as_str().into());
    let artifact_root = cache_root
        .join(artifact_segment.as_str())
        .join(path_segment.as_str());
    let chunks_dir: StreamingCacheChunksDirectory = artifact_root.join("chunks").into();
    std::fs::create_dir_all(chunks_dir.as_path()).map_err(|source| MemoryError::Io {
        path: chunks_dir.as_path().to_path_buf().into(),
        source,
    })?;

    let mut input = File::open(source_path).map_err(|source| MemoryError::Io {
        path: source_path.to_path_buf().into(),
        source,
    })?;
    let buffer_len = usize::try_from(chunk_size.min(1024 * 1024)).unwrap_or(1024 * 1024);
    let mut buffer = vec![0_u8; buffer_len];
    let mut chunk_index = StreamingChunkIndex::ZERO;
    let mut written_in_chunk = 0_u64;
    let mut current_chunk = create_chunk_file(&chunks_dir, chunk_index)?;

    loop {
        let read = input.read(&mut buffer).map_err(|source| MemoryError::Io {
            path: source_path.to_path_buf().into(),
            source,
        })?;
        if read == 0 {
            break;
        }
        let mut offset = 0;
        while offset < read {
            if written_in_chunk == chunk_size {
                current_chunk.flush().map_err(|source| MemoryError::Io {
                    path: chunk_path(&chunks_dir, chunk_index)
                        .as_path()
                        .to_path_buf()
                        .into(),
                    source,
                })?;
                chunk_index = chunk_index.next();
                written_in_chunk = 0;
                current_chunk = create_chunk_file(&chunks_dir, chunk_index)?;
            }
            let available = usize::try_from(chunk_size - written_in_chunk).unwrap_or(usize::MAX);
            let take = available.min(read - offset);
            let end = offset
                .checked_add(take)
                .ok_or_else(|| MemoryError::InternalInvariant {
                    operation: "streaming cache chunk range".into(),
                    reason: "chunk range overflow".into(),
                })?;
            let bytes = buffer
                .get(offset..end)
                .ok_or_else(|| MemoryError::InternalInvariant {
                    operation: "streaming cache chunk range".into(),
                    reason: "chunk range exceeds read buffer".into(),
                })?;
            current_chunk
                .write_all(bytes)
                .map_err(|source| MemoryError::Io {
                    path: chunk_path(&chunks_dir, chunk_index)
                        .as_path()
                        .to_path_buf()
                        .into(),
                    source,
                })?;
            written_in_chunk += u64::try_from(take).unwrap_or(u64::MAX);
            offset = end;
        }
    }
    current_chunk.flush().map_err(|source| MemoryError::Io {
        path: chunk_path(&chunks_dir, chunk_index)
            .as_path()
            .to_path_buf()
            .into(),
        source,
    })?;

    let manifest = StreamingCacheManifest {
        schema_version: StreamingCacheSchemaVersion::INITIAL,
        artifact_id: artifact_id.retained(),
        relative_path: relative_path.retained(),
        total_chunks: StreamingChunkCount::from_last_index(chunk_index),
        chunk_size: chunk_size_count,
        total_size: total_size_count,
        chunks_dir: StreamingRelativePath::try_new("chunks".retained()).ok_or_else(|| {
            model_error(
                "stream-file-into-chunks",
                "chunks directory must not be empty",
            )
        })?,
        status: StreamingArtifactStatus::Ready,
    };
    let manifest_path = artifact_root.join("streaming-manifest.json");
    let text = crate::boundary::streaming_cache::encode_manifest(&manifest)?;
    std::fs::write(&manifest_path, text).map_err(|source| MemoryError::Io {
        path: manifest_path.retained().into(),
        source,
    })?;
    Ok(StreamingCacheWriteReport {
        manifest,
        manifest_path: manifest_path.into(),
    })
}

fn create_chunk_file(
    chunks_dir: &StreamingCacheChunksDirectory,
    chunk_index: StreamingChunkIndex,
) -> Result<File> {
    let path = chunk_path(chunks_dir, chunk_index);
    File::create(path.as_path()).map_err(|source| MemoryError::Io {
        path: path.as_path().to_path_buf().into(),
        source,
    })
}

fn chunk_path(
    chunks_dir: &StreamingCacheChunksDirectory,
    chunk_index: StreamingChunkIndex,
) -> StreamingCacheChunkPath {
    chunks_dir
        .join(format!("{:08}.chunk", u64::from(chunk_index)))
        .into()
}

fn validate_manifest_shape(manifest: &StreamingCacheManifest) -> Result<()> {
    if u32::from(manifest.schema_version) != STREAMING_CACHE_SCHEMA_VERSION {
        return Err(model_error(
            "validate-streaming-cache-manifest",
            format!(
                "unsupported streaming cache schema version {}",
                u32::from(manifest.schema_version)
            ),
        ));
    }
    if u64::from(manifest.total_chunks) == 0
        || u64::from(manifest.chunk_size) == 0
        || u64::from(manifest.total_size) == 0
    {
        return Err(model_error(
            "validate-streaming-cache-manifest",
            "chunk count, chunk size, and total size must be non-zero",
        ));
    }
    validate_relative_path(&manifest.relative_path)?;
    validate_relative_path(&manifest.chunks_dir)?;
    Ok(())
}

fn validate_relative_path(relative: &StreamingRelativePath) -> Result<()> {
    let path_text = relative.as_str();
    let candidate = Path::new(path_text);
    let valid = !path_text.trim().is_empty()
        && !candidate.is_absolute()
        && candidate
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
        && !path_text.contains('\0');
    if valid {
        Ok(())
    } else {
        Err(model_error(
            "validate-streaming-cache-path",
            format!("unsafe streaming cache relative path: {path_text:?}"),
        ))
    }
}

fn safe_join(
    root: &Path,
    relative: &StreamingRelativePath,
) -> Result<StreamingCacheChunksDirectory> {
    validate_relative_path(relative)?;
    Ok(root.join(relative.as_str()).into())
}

fn safe_segment(value: StreamingCacheSegmentInput<'_>) -> StreamingCachePathSegment {
    let value = value.as_str();
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('-');
        }
    }
    if out.is_empty() {
        "artifact".retained().into()
    } else {
        out.into()
    }
}

fn model_error(operation: &'static str, reason: impl Into<String>) -> MemoryError {
    MemoryError::ModelRuntime {
        operation: operation.into(),
        reason: reason.into().into(),
    }
}
