// PASS fixture for RUST-ERR-CONTEXT: `.with_context` attached at the `?`
// boundary.
use anyhow::Context;
use std::fs;

fn load(path: &str) -> anyhow::Result<String> {
    let contents = fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
    Ok(contents)
}
