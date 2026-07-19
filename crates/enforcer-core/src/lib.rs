//! `enforcer-core` — the shared foundation crate for the Ocentra Enforcer
//! Cargo workspace (arc-01).
//!
//! Exports the shared [`error::Result`]/[`error::Error`] types, `tracing`
//! initialization, process exit codes, and the reusable telemetry
//! infrastructure folded in per the OcentraParent "Logging = structured data
//! (NO new crate)" borrow: two-layer redaction, a generic append-only
//! [`ndjson_writer::NdjsonWriter`], a pure SHA-256 hash-chain primitive, and
//! Windows-first path/time/env helpers. Also owns the d05 context-budget
//! ratchet primitive ([`context_budget`]) — a generic measured-surface vs.
//! committed-baseline gate with no knowledge of what surface is measured
//! (`enforcer-mcp::tool_surface` is its one caller today) — and the f04
//! [`run_context`] silent-vs-human `RunContext` resolution point + UI/server
//! gate.
//!
//! VENDORING ATTRIBUTION (arc-01 / EXECUTION_MODEL §2) — RECONCILED
//! 2026-07-05: the `redaction`, `ndjson_writer`, `hash_chain`, and
//! `platform` modules were originally specified as VENDORED from
//! OcentraParent `logging-core`, but its canonical source was unreachable
//! from the build machine at the time (no `E:` drive, not indexed in
//! codebase-memory; lesson L12), so these modules implemented the
//! workpack's behavioral contract directly. That source is now reachable
//! and has been diff-reconciled; the finding is that NONE of the four
//! required a literal port:
//! - `redaction`: real upstream is single-layer/flat-only — this module's
//!   two-layer/nested-JSON design is a deliberate independent extension
//!   (see the module doc for the comparison).
//! - `hash_chain`: no upstream counterpart exists at all — this is
//!   Enforcer-native code, not a vendored module.
//! - `platform` + `ndjson_writer`: upstream's equivalents (raw-string path
//!   helpers, a scope/stream/date-partitioned multi-file writer) are
//!   superseded by Enforcer's own more-rigorous independent designs
//!   (`enforcer_domain::paths::{RepoRoot, RelPath}` branded newtypes for
//!   path handling; callers pick their own explicit NDJSON path rather
//!   than a forced scope/stream/date taxonomy). Nothing to port.
//!
//! Contract still honored: both redaction layers always run; the NDJSON
//! sink is append-only; the hash-chain is side-effect-free.
//!
//! No `pub use` barrels (workspace doctrine): consumers path through the
//! modules directly, e.g. `enforcer_core::error::Result`.

pub mod context_budget;
pub mod error;
pub mod exit_codes;
#[path = "boundary/hash_chain.rs"]
pub mod hash_chain;
#[path = "boundary/ndjson.rs"]
mod ndjson_boundary;
pub mod ndjson_writer;
#[path = "boundary/platform.rs"]
pub mod platform;
#[path = "boundary/redaction.rs"]
pub mod redaction;
pub mod run_context;
#[path = "boundary/telemetry.rs"]
pub mod telemetry;
#[path = "boundary/tracing_setup.rs"]
pub mod tracing_setup;
