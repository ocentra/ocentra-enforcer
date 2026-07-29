// FAIL fixture for RUST-ERR-CONTEXT: bare `?` on a fallible I/O call with
// no `.with_context(...)` at the propagation boundary.
use std::fs;

fn load(path: &str) -> std::io::Result<String> {
    let contents = fs::read_to_string(path)?;
    Ok(contents)
}
