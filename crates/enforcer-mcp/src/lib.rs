//! `enforcer-mcp` — the Rust stdio MCP server (arc-21).
//!
//! # Charter
//!
//! Per `RUST_ARCHITECTURE.md` ("One binary IS the engine"), this crate is
//! transport + consolidated tool registry + router — NOT business logic.
//! Where the legacy `.mjs` MCP tree lived as a large Node engine
//! (`mcp/rust-rules-mcp-transport*.mjs`, `-tool-registry*.mjs`,
//! `-dispatch.mjs`, `-fallback*.mjs`, `-fingerprint.mjs`, `-context.mjs`),
//! this crate is the Rust replacement, laid as a SKELETON that:
//!
//! - speaks MCP over stdio with DUAL framing (`Content-Length:` header
//!   blocks OR bare NDJSON lines, auto-detected per message, echoed back
//!   in the same framing) — [`transport`];
//! - exposes the CONSOLIDATED tool surface (scan/check/proof/coordination/
//!   diagnostics), doubled under a defined-window `rust_rules_*` legacy
//!   alias set — [`registry`], [`aliases`];
//! - refuses coordination WRITE tools fail-closed when the running
//!   server's fingerprint disagrees with disk (the a02 seam) — [`gate`];
//! - dispatches every tool call to its owning engine crate
//!   (`enforcer-scan`/`-coordination`/`-proof`/`-harness`), never
//!   reimplementing their logic — [`router`];
//! - confines every stdout/stderr write to ONE sanctioned sink module —
//!   [`sink`].
//!
//! # Server-name seam
//! [`name::SERVER_NAME`] is a TRANSITIONAL constant; x01 owns the final
//! canonical value (`enforcer`) and its cutover. See [`name`]'s module
//! docs.
//!
//! # Orphaned MCP mechanics ported here (AUDIT_FINDINGS WAVE 4)
//! Four mechanics that had no prior Rust owner are ported in THIS crate,
//! each with an explicit fail/pass fixture (per the workpack's own
//! requirement that none silently drop in the port):
//! - the legacy `rust_rules_*` alias surface + deprecation window —
//!   [`aliases`];
//! - the stale-server write-gate + `ocentra_enforcer_run` CLI fallback —
//!   [`gate`];
//! - `coordination_repair`'s write/dry-run gating predicate — also
//!   [`gate::should_block_stale_tool`];
//! - the `check` named-check enum parity seam — [`registry`].
//!
//! # Context-budget brake (d05)
//! [`tool_surface`] measures this crate's own consolidated tool registry
//! (tool count + description bytes/token estimate) and ratchets it against
//! a committed baseline via `enforcer-core`'s generic
//! [`enforcer_core::context_budget`] gate — a T1 hard ratchet plus a T2
//! advisory efficiency score. See [`tool_surface`]'s module docs for the
//! d04 `RunRecord` telemetry seam this measure records into once that
//! sibling pack lands.
//!
//! # Live orchestration lessons this surface must not regress
//! `docs/plans/enforcer-selfhost-plan/refs/orchestration-lessons.md` L1
//! (idempotent init), L2 (caller identity required), L13 (glob claims +
//! transparent batching) are already fixed in `enforcer-coordination`
//! (arc-16) — [`router::dispatch`]'s `coordination_claim` handler decodes
//! args and calls straight through to `enforcer_coordination::api::{init,
//! claim_all}` rather than re-deriving caller identity or re-splitting
//! paths itself, so this MCP layer inherits the fix instead of
//! reintroducing the bug at a new layer. L21 (mail sender identity / claim
//! TTL) is NOT yet fixed upstream in `enforcer-coordination` (no `mail`
//! API exists there yet) — this crate's registry advertises
//! `coordination_mail`/`message` tool NAMES (for surface-parity/d05
//! measurement) but [`router::dispatch`] reports them as registered-but-
//! not-yet-wired rather than fabricating a working handler, so this layer
//! never pretends L21 is fixed when it is not.
//!
//! No `pub use` barrels (workspace doctrine): consumers path through the
//! modules directly, e.g. `enforcer_mcp::sink::run_stdio_server`.

pub mod aliases;
pub mod fingerprint;
pub mod gate;
pub mod name;
pub mod registry;
pub mod router;
pub mod sink;
pub mod tool_surface;
pub mod transport;
