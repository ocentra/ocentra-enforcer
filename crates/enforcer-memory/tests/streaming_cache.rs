use std::io::Read;

use enforcer_domain::memory_types::{
    StreamingArtifactKey, StreamingByteCount, StreamingRelativePath,
};
use enforcer_memory::streaming_cache::{
    assemble_chunks_to_file, read_streaming_cache_manifest, should_chunk_file,
    stream_file_into_chunks_with_size, streaming_chunk_reader, STREAMING_CHUNK_SIZE,
};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn artifact_key(value: &str) -> Result<StreamingArtifactKey, Box<dyn std::error::Error>> {
    StreamingArtifactKey::try_new(value.to_owned()).ok_or_else(|| "invalid artifact key".into())
}

fn relative_path(value: &str) -> Result<StreamingRelativePath, Box<dyn std::error::Error>> {
    StreamingRelativePath::try_new(value.to_owned()).ok_or_else(|| "invalid relative path".into())
}

fn byte_count(value: u64) -> Result<StreamingByteCount, Box<dyn std::error::Error>> {
    let value = std::num::NonZeroU64::new(value).ok_or("byte count must be non-zero")?;
    Ok(StreamingByteCount::try_new(value))
}

#[test]
fn large_artifact_threshold_matches_tabagent_chunking_policy() {
    assert!(!should_chunk_file(STREAMING_CHUNK_SIZE.into()).is_required());
    assert!(should_chunk_file((STREAMING_CHUNK_SIZE + 1).into()).is_required());
}

#[test]
fn stream_cache_chunks_and_reassembles_exact_bytes() -> TestResult {
    let temp = TempDir::new()?;
    let source = temp.path().join("model.onnx");
    let bytes = b"abcdefghijklmnopqrstuvwxyz0123456789";
    std::fs::write(&source, bytes)?;

    let report = stream_file_into_chunks_with_size(
        &source,
        temp.path(),
        &artifact_key("Qwen/Qwen3")?,
        &relative_path("onnx/model.onnx")?,
        byte_count(7)?,
    )?;
    assert_eq!(u64::from(report.manifest.total_chunks), 6);
    assert_eq!(u64::from(report.manifest.total_size), bytes.len() as u64);

    let loaded = read_streaming_cache_manifest(&report.manifest_path)?;
    let output = temp.path().join("assembled.onnx");
    assemble_chunks_to_file(&report.manifest_path, &loaded, &output)?;

    assert_eq!(std::fs::read(output)?, bytes);
    Ok(())
}

#[test]
fn malformed_streaming_cache_manifest_is_rejected() -> TestResult {
    let temp = TempDir::new()?;
    let manifest_path = temp.path().join("manifest.json");
    std::fs::write(
        &manifest_path,
        r#"{"schemaVersion":0,"artifactId":"","relativePath":"","totalChunks":0,"chunkSize":0,"totalSize":0,"chunksDir":"","status":"ready"}"#,
    )?;

    let outcome = read_streaming_cache_manifest(&manifest_path);
    assert!(matches!(
        outcome,
        Err(enforcer_memory::error::MemoryError::ModelRuntime { .. })
    ));
    Ok(())
}

#[test]
fn stream_reader_crosses_chunk_boundaries_without_full_assembly() -> TestResult {
    let temp = TempDir::new()?;
    let source = temp.path().join("model.gguf");
    let bytes = b"hello-ornith-qwen-streaming-cache";
    std::fs::write(&source, bytes)?;

    let report = stream_file_into_chunks_with_size(
        &source,
        temp.path(),
        &artifact_key("Ornith/GGUF")?,
        &relative_path("model.gguf")?,
        byte_count(5)?,
    )?;
    let mut reader = streaming_chunk_reader(&report.manifest_path, &report.manifest)?;
    let mut actual = Vec::new();
    reader.read_to_end(&mut actual)?;

    assert_eq!(actual, bytes);
    Ok(())
}

#[test]
fn stream_reader_fails_closed_when_chunk_is_missing() -> TestResult {
    let temp = TempDir::new()?;
    let source = temp.path().join("model.gguf");
    std::fs::write(&source, b"missing-chunk-proof")?;

    let report = stream_file_into_chunks_with_size(
        &source,
        temp.path(),
        &artifact_key("Ornith/GGUF")?,
        &relative_path("model.gguf")?,
        byte_count(4)?,
    )?;
    let manifest_parent = report.manifest_path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "manifest has no parent")
    })?;
    let chunk_path = manifest_parent.join("chunks").join("00000001.chunk");
    std::fs::remove_file(chunk_path)?;

    let mut reader = streaming_chunk_reader(&report.manifest_path, &report.manifest)?;
    let mut actual = Vec::new();
    let result = reader.read_to_end(&mut actual);

    assert!(matches!(
        result,
        Err(ref error) if error.kind() == std::io::ErrorKind::NotFound
    ));
    Ok(())
}
