// FAIL fixture for RESIL-ATOMIC-WRITE.1: an in-place truncate-then-write
// with no temp-file+rename (or equivalent) guard anywhere in the file.

fn save_config(path: &str, data: &[u8]) -> std::io::Result<()> {
    fs::write(path, data)
}
