use std::fs;
use std::io;
use std::path::Path;

pub(crate) fn is_probably_binary(path: &Path) -> io::Result<bool> {
    let bytes = fs::read(path)?;
    Ok(bytes.iter().take(4096).any(|byte| *byte == 0))
}
