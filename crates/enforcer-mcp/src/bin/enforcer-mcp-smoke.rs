//! A minimal stdio-server BINARY that exists ONLY to give this crate's
//! P3 live-MCP-tool proof a real OS process to spawn (per this workpack's
//! Acceptance And Proof: "the MCP stdio smoke... is the key row").
//!
//! # Ownership note
//! `enforcer-cli` (arc-22) owns the PRODUCTION `enforcer` binary/entry
//! point (`enforcer scan|check|install|serve|plan|...`), which will wire
//! [`enforcer_mcp::sink::run_stdio_server`] behind its own `serve`
//! subcommand once that crate lands. This binary is a throwaway smoke
//! harness scoped to THIS crate's own proof — it does nothing but call the
//! same public [`enforcer_mcp::sink::run_stdio_server`] entry point arc-22
//! will call, so proving THIS binary's stdio behavior is equivalent to
//! proving the eventual production wiring's stdio behavior (the router/
//! transport/registry code path is identical either way).

fn main() -> std::io::Result<()> {
    let cli_path = std::env::args()
        .nth(1)
        // ALLOC-JUSTIFICATION: default_dispatch_context takes the command path by
        // value, so the fallback must match argv's owned String representation.
        .unwrap_or_else(|| "enforcer".to_owned());
    let ctx = enforcer_mcp::sink::default_dispatch_context(cli_path);
    enforcer_mcp::sink::run_stdio_server(&ctx)
}
