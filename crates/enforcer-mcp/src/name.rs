//! The MCP server-name const.
//!
//! # Ownership (transitional)
//!
//! Per `RUST_ARCHITECTURE.md` ("Global-install scope contract"), the final
//! product/binary/MCP-server name is `enforcer` and every install adapter
//! registers under ONE x01-owned const — `mcpServers["enforcer"]`, tools
//! surfacing as `mcp__enforcer__*`. **x01 owns that const and its final
//! value.** This module is a SKELETON seam laid by arc-21 so the rest of
//! this crate (registry/router/transport `serverInfo`) has exactly one
//! place to read the name from, never a hardcoded literal scattered across
//! modules — when x01 lands, only this file's value (and doc comment)
//! change, nothing else in the crate.
//!
//! The value here is explicitly TRANSITIONAL: it matches the legacy
//! `.mjs` MCP's `package.json` name
//! (`mcp/rust-rules-mcp.mjs`/`rust-rules-mcp-context.mjs`) so the dual-run
//! period (legacy `.mjs` still live per `RUST_ARCHITECTURE.md`'s
//! "Dev-time transition wiring" anti-recursion note) does not collide tool
//! namespaces mid-migration. x01's cutover pass replaces this constant
//! with the canonical `enforcer` name; every canonical tool name in
//! [`crate::registry`] is derived from [`SERVER_NAME`], so that pass is a
//! one-line change here plus a regenerated tool-name table, not a
//! multi-file hunt.

/// The MCP server name this binary registers under.
///
/// TRANSITIONAL value — see module docs. x01 sets the final value.
pub const SERVER_NAME: &str = "ocentra-enforcer";

/// The canonical tool-name prefix derived from [`SERVER_NAME`].
///
/// Canonical tools are named `<CANONICAL_TOOL_PREFIX>_<verb>` (e.g.
/// `ocentra_enforcer_check`), matching the legacy `.mjs` registry's
/// `ocentra_enforcer_*` family so existing worker prompts/docs referencing
/// that family keep resolving during the dual-run period.
pub const CANONICAL_TOOL_PREFIX: &str = "ocentra_enforcer";

/// The legacy compatibility alias prefix (see [`crate::aliases`]).
pub const LEGACY_ALIAS_PREFIX: &str = "rust_rules";

#[cfg(test)]
mod tests {
    use super::{CANONICAL_TOOL_PREFIX, LEGACY_ALIAS_PREFIX, SERVER_NAME};

    #[test]
    fn constants_are_non_empty_and_distinct() {
        assert!(!SERVER_NAME.is_empty());
        assert!(!CANONICAL_TOOL_PREFIX.is_empty());
        assert!(!LEGACY_ALIAS_PREFIX.is_empty());
        assert_ne!(CANONICAL_TOOL_PREFIX, LEGACY_ALIAS_PREFIX);
    }
}
