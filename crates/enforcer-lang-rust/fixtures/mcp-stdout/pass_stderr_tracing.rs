// PASS fixture for RUST-MCP-1.1: writes routed to stderr/tracing instead of
// stdout.
pub fn emit(msg: &str) {
    tracing::info!(%msg, "emitting");
}
