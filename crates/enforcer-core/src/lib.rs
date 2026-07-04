//! `enforcer-core` — the shared foundation crate for the Ocentra Enforcer
//! Cargo workspace (arc-01).
//!
//! Exports the shared [`error::Result`]/[`error::Error`] types, `tracing`
//! initialization, process exit codes, and the reusable telemetry
//! infrastructure folded in per the OcentraParent "Logging = structured data
//! (NO new crate)" borrow: two-layer redaction, a generic append-only
//! [`ndjson_writer::NdjsonWriter`], a pure SHA-256 hash-chain primitive, and
//! Windows-first path/time/env helpers.
//!
//! VENDORING ATTRIBUTION (arc-01 / EXECUTION_MODEL §2): the `redaction`,
//! `ndjson_writer`, `hash_chain`, and `platform` modules are specified as
//! VENDORED from OcentraParent `logging-core`. The canonical source
//! (`E:\OcentraParent`) was unreachable from this build machine (no E:
//! drive, not indexed in codebase-memory), so these modules implement the
//! workpack's behavioral contract directly and MUST be diff-reconciled
//! against the canonical OcentraParent `logging-core` modules when that
//! source is reachable. Contract honored: both redaction layers always run;
//! the NDJSON sink is append-only; the hash-chain is side-effect-free.
//!
//! No `pub use` barrels (workspace doctrine): consumers path through the
//! modules directly, e.g. `enforcer_core::error::Result`.

pub mod error;
pub mod exit_codes;
pub mod hash_chain;
pub mod ndjson_writer;
pub mod platform;
pub mod redaction;
pub mod tracing_setup;
