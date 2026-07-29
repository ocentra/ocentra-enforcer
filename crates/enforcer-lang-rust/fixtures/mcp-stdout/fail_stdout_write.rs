// FAIL fixture for RUST-MCP-1.1: rmcp-lane crate writing to stdout, which
// is the MCP protocol channel.
use std::io::Write;

pub fn emit(msg: &str) {
    let _ = std::io::stdout().write_all(msg.as_bytes());
}
