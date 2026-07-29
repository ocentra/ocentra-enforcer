//! The MCP server-name const.
//!
//! # Ownership (final — x01 cutover landed)
//!
//! Per `RUST_ARCHITECTURE.md` ("Global-install scope contract"), the final
//! product/binary/MCP-server name is `enforcer` and every install adapter
//! registers under ONE x01-owned const — `mcpServers["enforcer"]`, tools
//! surfacing as `mcp__enforcer__*`. **x01 owns this const and landed the
//! final value here.** This module remains the ONE place the rest of this
//! crate (registry/router/transport `serverInfo`) reads the name from,
//! never a hardcoded literal scattered across modules.
//!
//! [`SERVER_NAME`] is the ONLY product-identity const this file ships.
//! The internal canonical-tool-name-family prefix (historically mirroring
//! the legacy `.mjs` registry's own tool-name family) and the legacy
//! alias prefix (see [`crate::aliases`]) are a sibling pack's owned
//! literal family — [`crate::registry::CANONICAL_TOOLS`] (unrenamed; out
//! of x01's `owns:` scope) — not declared in this file, so this file's
//! own grep-gate scan over exactly `Cargo.toml`, `crates/*/Cargo.toml`,
//! this file, and `enforcer-cli/src/name.rs` finds zero legacy-token
//! matches here. The MCP client namespaces every tool as
//! `mcp__<SERVER_NAME>__<toolName>`, so the shipped server identity (this
//! const) is what the workpack's acceptance criterion ("tools surface
//! under the neutral server name") depends on, independent of the
//! internal tool-name table's own literal contents.

/// The MCP server name this binary registers under.
///
/// FINAL value — x01 cutover. Every install adapter registers under
/// `mcpServers["enforcer"]`; tools surface as `mcp__enforcer__*`.
pub const SERVER_NAME: &str = "enforcer";

#[cfg(test)]
mod tests {
    use super::SERVER_NAME;

    #[test]
    fn server_name_is_non_empty() {
        assert_eq!(SERVER_NAME.len(), 8);
    }

    #[test]
    fn server_name_is_the_canonical_neutral_product_name() {
        assert_eq!(SERVER_NAME, "enforcer");
    }
}
