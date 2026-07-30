//! Typed coordination primitives for multi-agent claim ownership.
//!
//! The crate persists hash-chained coordination events, projects active
//! claims, evaluates exact-path lock conflicts, performs lane-scoped
//! closeout, compacts append-only streams, and runs bounded fix attempts.
//! Caller, claim, lock, event, and path identities use canonical
//! `enforcer-domain` types; serialized strings are confined to boundary
//! modules. The Rust and JavaScript command surfaces share wire-hash and
//! closeout-scope regression coverage.

pub mod api;
pub mod daemon;
pub mod domain;
pub mod error;
pub mod events;
pub mod fix_loop;
pub mod ledger;
pub mod lock;
pub mod sync;
