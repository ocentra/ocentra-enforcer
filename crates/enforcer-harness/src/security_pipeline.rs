//! h07 — security-tooling stages wired into the arc-18 run-adapter
//! machinery: coverage floors, API-fuzz/property, concurrency, static
//! analysis, and observability checks, each graceful-skipping HONESTLY
//! (a09-style: `skipped != passed != failed`) when its tool is absent,
//! per `docs/plans/enforcer-selfhost-plan/workpacks/
//! h07-security-tooling-ci-observability.md`.
//!
//! # Domain/boundary split
//!
//! Following the workspace's parse-at-boundary doctrine (see
//! `enforcer-core`'s `run_context`/`run_context::boundary` split for the
//! reference pattern):
//!
//! - [`adapters`] is the BOUNDARY: it alone reads raw recorded engine
//!   output (JSON text captured from an external tool run — no live
//!   engine is required in CI), rejects malformed or dishonest shapes
//!   with typed decode failures, and mints the branded values every
//!   other module consumes.
//! - `enforcer-domain::harness_types` owns the shared branded vocabulary.
//! - The stage modules own the branded outcome types and the pass/fail
//!   GATES (each an `enforcer_validator::Validator`), one gate per
//!   module:
//!   - [`coverage`] — T1 line/branch floors (>=90%/>=80%) + drop
//!     detection ([`coverage::CoverageFloorGate`]).
//!   - [`observability`] — T2 scored money-path logging gate
//!     ([`observability::MoneyPathLoggingGate`]) + the shared event
//!     vocabulary.
//!   - [`observability_sampling`] — T1 security-event sampling gate
//!     ([`observability_sampling::SamplingDropGate`]).
//!   - [`fuzz`] — T1 missing-persisted-seed gate
//!     ([`fuzz::FuzzSeedGate`]).
//!   - [`static_analysis`] — signal-only unless threat-mapped to an
//!     exploitable `ThreatId` ([`static_analysis::StaticThreatGate`]).
//!   - [`concurrency`] — T2 severity-threshold gate
//!     ([`concurrency::ConcurrencySeverityGate`]).
//!   - [`crypto_localnet`] — the OPTIONAL, disjoint opt-in seam
//!     (off by default; consumed only by e-pack-crypto-blockchain; its
//!     absence narrows coverage, never blocks the other stages).
//!
//! Because every stage runs over RECORDED reports through the same
//! functions, the pipeline runs identically as a local `enforcer` check
//! and as a CI job (self-referential parity, matching
//! [`crate::ci_parity`]'s "local == CI" posture). Acceptance proof lives
//! in `tests/security_pipeline.rs` over
//! `tests/fixtures/security_pipeline/**`.

pub mod adapters;
pub mod concurrency;
pub mod coverage;
pub mod crypto_localnet;
pub mod fuzz;
pub mod observability;
pub mod observability_sampling;
pub mod static_analysis;
