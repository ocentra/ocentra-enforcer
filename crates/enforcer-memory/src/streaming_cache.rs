//! Chunked model artifact cache for large ONNX/GGUF files.
//!
//! TabAgent's browser runtime learned that loading multi-GB model files as a
//! single blob is fragile. This module is the Rust-side equivalent: artifacts
//! can be copied into fixed-size chunks with a manifest and then read back as a
//! streaming `Read` implementation without assembling the whole file in memory.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{MemoryError, Result};

pub const STREAMING_CACHE_SCHEMA_VERSION: u32 = 1;
pub const STREAMING_CHUNK_SIZE: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StreamingArtifactStatus {
    Ready,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamingCacheManifest {
    pub schema_version: u32,
    pub artifact_id: String,
    pub relative_path: String,
    pub total_chunks: u64,
    pub chunk_size: u64,
    pub total_size: u64,
    pub chunks_dir: String,
    pub status: StreamingArtifactStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamingCacheWriteReport {
    pub manifest: StreamingCacheManifest,
    pub manifest_path: PathBuf,
}

pub fn should_chunk_file(size_bytes: u64) -> bool {
    size_bytes > STREAMING_CHUNK_SIZE
}

pub fn stream_file_into_chunks(
    source_path: &Path,
    cache_root: &Path,
    artifact_id: &str,
    relative_path: &str,
) -> Result<StreamingCacheWriteReport> {
    stream_file_into_chunks_with_size(
        source_path,
        cache_root,
        artifact_id,
        relative_path,
        STREAMING_CHUNK_SIZE,
    )
}

pub fn read_streaming_cache_manifest(path: &Path) -> Result<StreamingCacheManifest> {
    let text = std::fs::read_to_string(path).map_err(|source| MemoryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let manifest: StreamingCacheManifest = serde_json::from_str(&text)?;
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
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let mut output = File::create(output_path).map_err(|source| MemoryError::Io {
        path: output_path.to_path_buf(),
        source,
    })?;
    std::io::copy(&mut reader, &mut output).map_err(|source| MemoryError::Io {
        path: output_path.to_path_buf(),
        source,
    })?;
    Ok(())
}

#[derive(Debug)]
pub struct StreamingChunkReader {
    chunks_dir: PathBuf,
    total_chunks: u64,
    next_chunk: u64,
    active: Option<File>,
}

impl StreamingChunkReader {
    fn new(chunks_dir: PathBuf, total_chunks: u64) -> Result<Self> {
        if total_chunks == 0 {
            return Err(model_error(
                "streaming-chunk-reader",
                "streaming manifest has zero chunks",
            ));
        }
        Ok(Self {
            chunks_dir,
            total_chunks,
            next_chunk: 0,
            active: None,
        })
    }

    fn open_next_chunk(&mut self) -> std::io::Result<bool> {
        if self.next_chunk >= self.total_chunks {
            return Ok(false);
        }
        let path = chunk_path(&self.chunks_dir, self.next_chunk);
        self.active = Some(File::open(path)?);
        self.next_chunk += 1;
        Ok(true)
    }
}

impl Read for StreamingChunkReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            if self.active.is_none() && !self.open_next_chunk()? {
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
    artifact_id: &str,
    relative_path: &str,
    chunk_size: u64,
) -> Result<StreamingCacheWriteReport> {
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
            path: source_path.to_path_buf(),
            source,
        })?
        .len();
    if total_size == 0 {
        return Err(model_error(
            "stream-file-into-chunks",
            "cannot chunk an empty artifact",
        ));
    }

    let artifact_segment = safe_segment(artifact_id);
    let path_segment = safe_segment(relative_path);
    let artifact_root = cache_root.join(&artifact_segment).join(&path_segment);
    let chunks_dir = artifact_root.join("chunks");
    std::fs::create_dir_all(&chunks_dir).map_err(|source| MemoryError::Io {
        path: chunks_dir.clone(),
        source,
    })?;

    let mut input = File::open(source_path).map_err(|source| MemoryError::Io {
        path: source_path.to_path_buf(),
        source,
    })?;
    let mut buffer = vec![0_u8; chunk_size.min(1024 * 1024) as usize];
    let mut chunk_index = 0_u64;
    let mut written_in_chunk = 0_u64;
    let mut current_chunk = create_chunk_file(&chunks_dir, chunk_index)?;

    loop {
        let read = input.read(&mut buffer).map_err(|source| MemoryError::Io {
            path: source_path.to_path_buf(),
            source,
        })?;
        if read == 0 {
            break;
        }
        let mut offset = 0;
        while offset < read {
            if written_in_chunk == chunk_size {
                current_chunk.flush().map_err(|source| MemoryError::Io {
                    path: chunk_path(&chunks_dir, chunk_index),
                    source,
                })?;
                chunk_index += 1;
                written_in_chunk = 0;
                current_chunk = create_chunk_file(&chunks_dir, chunk_index)?;
            }
            let available = (chunk_size - written_in_chunk) as usize;
            let take = available.min(read - offset);
            current_chunk
                .write_all(&buffer[offset..offset + take])
                .map_err(|source| MemoryError::Io {
                    path: chunk_path(&chunks_dir, chunk_index),
                    source,
                })?;
            written_in_chunk += take as u64;
            offset += take;
        }
    }
    current_chunk.flush().map_err(|source| MemoryError::Io {
        path: chunk_path(&chunks_dir, chunk_index),
        source,
    })?;

    let total_chunks = chunk_index + 1;
    let manifest = StreamingCacheManifest {
        schema_version: STREAMING_CACHE_SCHEMA_VERSION,
        artifact_id: artifact_id.to_owned(),
        relative_path: relative_path.to_owned(),
        total_chunks,
        chunk_size,
        total_size,
        chunks_dir: "chunks".to_owned(),
        status: StreamingArtifactStatus::Ready,
    };
    let manifest_path = artifact_root.join("streaming-manifest.json");
    let text = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(&manifest_path, text).map_err(|source| MemoryError::Io {
        path: manifest_path.clone(),
        source,
    })?;
    Ok(StreamingCacheWriteReport {
        manifest,
        manifest_path,
    })
}

fn create_chunk_file(chunks_dir: &Path, chunk_index: u64) -> Result<File> {
    let path = chunk_path(chunks_dir, chunk_index);
    File::create(&path).map_err(|source| MemoryError::Io { path, source })
}

fn chunk_path(chunks_dir: &Path, chunk_index: u64) -> PathBuf {
    chunks_dir.join(format!("{chunk_index:08}.chunk"))
}

fn validate_manifest_shape(manifest: &StreamingCacheManifest) -> Result<()> {
    if manifest.schema_version != STREAMING_CACHE_SCHEMA_VERSION {
        return Err(model_error(
            "validate-streaming-cache-manifest",
            format!(
                "unsupported streaming cache schema version {}",
                manifest.schema_version
            ),
        ));
    }
    if manifest.total_chunks == 0 || manifest.chunk_size == 0 || manifest.total_size == 0 {
        return Err(model_error(
            "validate-streaming-cache-manifest",
            "chunk count, chunk size, and total size must be non-zero",
        ));
    }
    validate_relative_path(&manifest.relative_path)?;
    validate_relative_path(&manifest.chunks_dir)?;
    Ok(())
}

fn validate_relative_path(path: &str) -> Result<()> {
    let candidate = Path::new(path);
    let valid = !path.trim().is_empty()
        && !candidate.is_absolute()
        && candidate
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
        && !path.contains('\0');
    if valid {
        Ok(())
    } else {
        Err(model_error(
            "validate-streaming-cache-path",
            format!("unsafe streaming cache relative path: {path:?}"),
        ))
    }
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf> {
    validate_relative_path(relative)?;
    Ok(root.join(relative))
}

fn safe_segment(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('-');
        }
    }
    if out.is_empty() {
        "artifact".to_owned()
    } else {
        out
    }
}

fn model_error(operation: &'static str, reason: impl Into<String>) -> MemoryError {
    MemoryError::ModelRuntime {
        operation,
        reason: reason.into(),
    }
}
