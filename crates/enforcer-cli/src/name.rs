//! The `enforcer` binary/product name const.
//!
//! # Ownership (transitional)
//!
//! Per `RUST_ARCHITECTURE.md` ("Global-install scope contract"), the final
//! product/binary/MCP-server name is `enforcer` and every install adapter
//! registers under ONE x01-owned const -- `mcpServers["enforcer"]`, tools
//! surfacing as `mcp__enforcer__*`. **x01 owns that const and its final
//! value.** This module is a SKELETON seam laid by arc-22 so the CLI side
//! of the crate (help text, `Cli::command().name`, any future install-time
//! self-description) has exactly one place to read the binary's own name
//! from, never a hardcoded literal scattered across `cli.rs`/`main.rs` --
//! when x01 lands, only this file's value (and doc comment) change.
//!
//! This mirrors `enforcer_mcp::name::SERVER_NAME`'s identical transitional
//! posture on the MCP side of the same binary; the two consts are
//! deliberately kept as separate seams (one per crate) so x01's cutover
//! pass touches each crate's own single source of truth rather than a
//! cross-crate re-export barrel (workspace doctrine: no `pub use` barrels).
//!
//! The value here is explicitly TRANSITIONAL: it matches the clap
//! `#[command(name = "...")]` literal already in `crate::cli` today. x01's
//! cutover pass replaces this constant (and the matching literal in
//! `crate::cli::Cli`) with the canonical final name in one change.

/// The name this binary presents as: its own clap program name and,
/// eventually, its install-adapter registration key.
///
/// TRANSITIONAL value -- see module docs. x01 sets the final value.
pub const BINARY_NAME: &str = "enforcer";

#[cfg(test)]
mod tests {
    use super::BINARY_NAME;

    #[test]
    fn binary_name_is_non_empty() {
        assert_eq!(BINARY_NAME, "enforcer");
    }

    #[test]
    fn binary_name_matches_the_clap_program_name() {
        // `crate::cli::Cli` hardcodes `name = "enforcer"` today (see
        // `cli.rs` module docs); this test documents that the two must
        // move together until x01's cutover pass derives the clap name
        // from this const directly.
        assert_eq!(BINARY_NAME, "enforcer");
    }
}
