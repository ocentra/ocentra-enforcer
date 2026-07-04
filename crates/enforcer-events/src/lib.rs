//! `enforcer-events` — the LEAN in-process typed event spine for the
//! Ocentra Enforcer workspace (arc-25).
//!
//! # Charter
//!
//! `DomainEvent` + `EventEnvelope<E>` (stored-decode re-verifies the
//! contract), correlation/causation ids (branded via `enforcer-domain`), and
//! panic-isolated Sequential/Concurrent dispatch. Consumed ONLY by the
//! long-lived/observable subsystems — scan lifecycle (arc-15), coordination
//! lane/claim/lease (arc-16), and proof (arc-17). Pure-compute crates
//! (domain/config/validator/lang-*) use plain calls, never this spine.
//!
//! Deliberately EXCLUDED (RUST_ARCHITECTURE.md "Eventing (lean, bounded)"):
//! contract-registry RPC catalog, aggregate-ordering gates, TTL/overflow
//! queue, request/response brokering, external transport. Event dispatch is
//! SYNC-first (locked decision); no `tokio` here unless a `serve` daemon
//! appears.
//!
//! # VENDORING ATTRIBUTION (arc-25 / EXECUTION_MODEL §2, lesson L12)
//!
//! This crate's workpack (`docs/plans/enforcer-selfhost-plan/workpacks/
//! arc-25-enforcer-events.md`) specifies VENDORING `enforcer-events` AS-IS
//! from OcentraParent's `ocentra-eventing` crate (copy source, rename
//! package, repoint internal deps). The canonical source
//! (`E:\OcentraParent`) was UNREACHABLE from this build machine (no `E:`
//! drive, not indexed in codebase-memory, no copy found under `vendor/` or
//! elsewhere in this repo). Per protocol this crate instead implements the
//! workpack's BEHAVIORAL CONTRACT directly:
//!
//! - a typed `DomainEvent` marker trait event payloads implement,
//! - `EventEnvelope<E>` carrying correlation/causation ids, a SHA-256
//!   payload digest, and a schema version, where DECODING RE-VERIFIES the
//!   digest against the payload (a version/digest-drifted envelope is
//!   REJECTED on decode, not silently accepted),
//! - panic-isolated `Sequential`/`Concurrent` dispatch to a list of
//!   fallible handlers, where one handler panicking or erroring does not
//!   stop the others from running.
//!
//! This module MUST be diff-reconciled against the canonical OcentraParent
//! `ocentra-eventing` crate when that source becomes reachable, to confirm
//! parity of shape and to pull in any dormant machinery (contract-registry,
//! aggregate-ordering, TTL queue, request/response, external transport)
//! that a later optional pass may choose to enable. Deviation recorded in
//! the arc-25 done-mail to `primary`.
//!
//! No `pub use` barrels (workspace doctrine): consumers path through the
//! modules directly, e.g. `enforcer_events::envelope::EventEnvelope`.

pub mod dispatch;
pub mod envelope;
pub mod event;
