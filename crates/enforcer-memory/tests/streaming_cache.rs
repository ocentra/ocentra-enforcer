use std::io::Read;

use enforcer_memory::streaming_cache::{
    assemble_chunks_to_file, read_streaming_cache_manifest, should_chunk_file,
    stream_file_into_chunks_with_size, streaming_chunk_reader, STREAMING_CHUNK_SIZE,
};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn large_artifact_threshold_matches_tabagent_chunking_policy() {
    assert!(!should_chunk_file(STREAMING_CHUNK_SIZE));
    assert!(should_chunk_file(STREAMING_CHUNK_SIZE + 1));
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
        "Qwen/Qwen3",
        "onnx/model.onnx",
        7,
    )?;
    assert_eq!(report.manifest.total_chunks, 6);
    assert_eq!(report.manifest.total_size, bytes.len() as u64);

    let loaded = read_streaming_cache_manifest(&report.manifest_path)?;
    let output = temp.path().join("assembled.onnx");
    assemble_chunks_to_file(&report.manifest_path, &loaded, &output)?;

    assert_eq!(std::fs::read(output)?, bytes);
    Ok(())
}

#[test]
fn stream_reader_crosses_chunk_boundaries_without_full_assembly() -> TestResult {
    let temp = TempDir::new()?;
    let source = temp.path().join("model.gguf");
    let bytes = b"hello-ornith-qwen-streaming-cache";
    std::fs::write(&source, bytes)?;

    let report =
        stream_file_into_chunks_with_size(&source, temp.path(), "Ornith/GGUF", "model.gguf", 5)?;
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

    let report =
        stream_file_into_chunks_with_size(&source, temp.path(), "Ornith/GGUF", "model.gguf", 4)?;
    let manifest_parent = report.manifest_path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "manifest has no parent")
    })?;
    let chunk_path = manifest_parent.join("chunks").join("00000001.chunk");
    std::fs::remove_file(chunk_path)?;

    let mut reader = streaming_chunk_reader(&report.manifest_path, &report.manifest)?;
    let mut actual = Vec::new();
    let result = reader.read_to_end(&mut actual);

    assert!(result.is_err());
    Ok(())
}
