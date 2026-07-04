//! Stream persistence + retention. Ported from
//! `src/coordination/vendor/{stream,retention}.js`.
//!
//! Peer registry / http+local sync transports / read-index (JSON + opt-in
//! SQLite) / serve-daemon are DEFERRED from this pass (see the crate-level
//! `README` deviation note in `lib.rs`) — the append-only stream + archive
//! model that they build on is fully ported and tested here first, since it
//! is the load-bearing invariant everything else depends on.

pub mod retention;
pub mod stream;
