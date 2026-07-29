// PASS fixture for RESIL-ATOMIC-WRITE.1: the write goes through a
// temp-file + atomic rename, so the truncate-then-write marker is guarded.

fn save_config(path: &str, data: &[u8]) -> std::io::Result<()> {
    let mut tmp = tempfile::NamedTempFile::new()?;
    tmp.write_all(data)?;
    tmp.persist(path)?;
    Ok(())
}
