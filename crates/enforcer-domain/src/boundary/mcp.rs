//! Raw MCP transport decoding into canonical domain decisions.

use crate::mcp_types::{McpExecutionMode, McpWriteIntent};

// BOUNDARY-INVARIANT: optional wire booleans are converted immediately into
// closed domain decisions before routing or coordination logic observes them.
// boundaryOwnerNote: enforcer-domain owns the shared MCP primitive decoder.
// Negative and absent input cases are covered by the boundary unit tests below.

/// Decode the optional MCP `write` field.
#[must_use]
pub const fn write_intent(value: Option<bool>) -> McpWriteIntent {
    match value {
        Some(true) => McpWriteIntent::Write,
        Some(false) => McpWriteIntent::ReadOnly,
        None => McpWriteIntent::Unspecified,
    }
}

/// Decode the optional MCP `dryRun` field.
#[must_use]
pub const fn execution_mode(value: Option<bool>) -> McpExecutionMode {
    match value {
        Some(true) => McpExecutionMode::DryRun,
        Some(false) => McpExecutionMode::Apply,
        None => McpExecutionMode::Unspecified,
    }
}
