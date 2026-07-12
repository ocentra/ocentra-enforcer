//! `enforcer-harness` — native-tool run-adapters + run-storage (arc-18).
//!
//! # Charter
//!
//! Two halves, both owned by this crate (per the arc-18 workpack), plus
//! feature modules other workpacks plug in ([`ci_parity`] is d11's):
//!
//! - **Half A — parsing** ([`parsers`]): shells out to native tools
//!   (cargo/tsc/ruff/pytest/eslint/pyright/bandit/SARIF-emitting tools...),
//!   parses their stdout/stderr into `enforcer-domain`-flavored diagnostics,
//!   and is the graceful-skip seam where an external engine is
//!   irreplaceable (report skip, never hard-fail).
//! - **Half B — storage** ([`storage`], [`retention`], [`query`],
//!   [`legacy`], [`duckdb_seam`]): persists each run under
//!   `.enforce/runs/<runId>/`, maintains `.enforce/db/ingest-manifest.json`
//!   and `.enforce/db/duckdb-status.json`, runs the retention/prune engine,
//!   and exposes the read/query surface backing the run-store MCP tools
//!   (`run_status`/`diagnostics`/`last_failure`/`artifact`/`prune_runs`/
//!   `reset_runs`) and the `runs` CLI subcommand.
//!
//! Ported from `src/harness.mjs` + `src/harness-parsers*.mjs`. Consumes
//! `enforcer_core::redaction` (does not re-inline the secret pattern list)
//! and `enforcer_domain` branded types. No `pub use` barrels (workspace
//! doctrine): consumers path through the modules directly, e.g.
//! `enforcer_harness::storage::run_harness`.
//!
//! ## DuckDB seam posture (stated per workpack requirement)
//!
//! **DEFERRED.** NDJSON is authoritative. This crate stamps
//! `.enforce/db/duckdb-status.json` with `mode: "optional"`,
//! `available: false` on every run write (matching the legacy `.mjs`
//! contract) but does NOT implement a DuckDB ingestion path. All read/query
//! APIs (`listRuns`/`runSummary`/`runDiagnostics`/`lastFailure`/
//! `readArtifact`) operate purely over the NDJSON + JSON summary files. A
//! later pass MAY port an optional DuckDB ingestion path behind the same
//! `duckdb-status.json` contract without breaking this crate's public API.

pub mod adapters;
pub mod ci_parity;
pub mod config;
pub mod duckdb_seam;
pub mod legacy;
pub mod parsers;
pub mod query;
pub mod retention;
pub mod security_pipeline;
pub mod storage;
