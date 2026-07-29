//! `enforcer-proof` — the Rust proof harness (arc-17).
//!
//! # Charter
//!
//! Where the legacy `.mjs` proof system lived as ad hoc JS
//! (`src/proof.mjs` / `src/proof-storage.mjs` / `src/proof-legacy.mjs` /
//! `src/proof-cli*.mjs` / `scripts/profile-proof-runner.mjs`), this crate is
//! the Rust replacement: it routes proof requests against a registry
//! ([`registry`]), runs them and captures artifacts + freshness
//! ([`harness`]), keeps the rich proof **envelope** — git-state / in-toto /
//! retention — ([`envelope`]), gates PR-ready **claims** against that
//! envelope ([`claim`]), imports/compares legacy artifact evidence
//! ([`legacy_import`]), and adds tamper-evidence the legacy system never
//! had: an append-only SHA-256 hash-chained NDJSON **journal**
//! ([`journal`]) that is verified on open AND on replay.
//!
//! # What this crate CONSUMES, not reimplements
//!
//! Per the OcentraParent "Logging = structured data (NO new crate)" borrow
//! (see `RUST_ARCHITECTURE.md`), the mechanism primitives live in
//! `enforcer-core` (arc-01) and this crate only builds the proof-specific
//! envelope on top of them:
//! - the generic append-only `enforcer_core::ndjson_writer::NdjsonWriter<T>`
//!   sink — [`journal`] appends journal records through it, never opening
//!   its own file handle with different semantics;
//! - the pure `enforcer_core::hash_chain` primitive — [`journal`] folds each
//!   record's digest over the previous record's digest using
//!   `link_digest`/`verify_chain` directly, no local reimplementation;
//! - the two-layer `enforcer_core::redaction::Redactor` — [`envelope`] and
//!   [`harness`] redact secrets through it (key-name layer + value-pattern
//!   layer, both always run), never declaring a local secret-pattern list.
//!
//! No `pub use` barrels (workspace doctrine): consumers path through the
//! modules directly, e.g. `enforcer_proof::claim::claim_proof`.

pub mod boundary;
#[path = "boundary/claim.rs"]
pub mod claim;
#[path = "boundary/envelope.rs"]
pub mod envelope;
pub mod harness;
#[path = "boundary/journal.rs"]
pub mod journal;
#[path = "boundary/legacy_import.rs"]
pub mod legacy_import;
#[path = "boundary/project_read_model.rs"]
pub mod read_model;
