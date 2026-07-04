//! `enforcer-coordination` — the Rust port of the multi-agent coordination
//! hub.
//!
//! # Origin
//! Ported from `src/coordination/{vendor/*.js,api.mjs,runner.mjs}` per
//! `docs/plans/enforcer-selfhost-plan/workpacks/arc-16-enforcer-coordination.md`
//! (arc-16). Behavior parity is asserted with fail/pass fixtures inline in
//! each module's `#[cfg(test)]` block, including the golden wire-hash
//! sentinel in [`events`] that any conforming implementation (JS or Rust)
//! must reproduce byte-for-byte.
//!
//! # Live dogfood findings fixed in this crate
//! `docs/plans/enforcer-selfhost-plan/refs/orchestration-lessons.md` L1, L2,
//! L13 were observed live against the `.mjs` coordination hub this very
//! multi-agent build runs on, and are load-bearing REQUIREMENTS here (see
//! [`api::init`] for L1, [`api::CallerContext`]/[`api::claim_all`] for L2,
//! and [`api::normalize_owns_paths`]/[`api::claim_all`] for L13).
//!
//! # Scope of this pass (deviations, honestly labeled)
//! The full arc-16 workpack is explicitly sized for MULTIPLE concurrent
//! workers splitting the crate by disjoint module sub-unit (`domain`,
//! `lock`, `session`, `sync`, `repair`, `events`, `api`, `runner`). This pass
//! was executed by a single lane and prioritizes the load-bearing,
//! independently-provable core over full surface coverage:
//!
//! - **Ported with fixtures:** `domain` (identity/paths/root), `events`
//!   (wire-hash canonicalization + golden sentinel), `lock` (the 6 conflict
//!   classes + protected-singleton escalation), `sync::stream` (append-only
//!   ndjson read/write + hash-chain), `sync::retention` (compact → archive,
//!   round-trip proof), `ledger` (active-claims projection), `api`
//!   (init/claim/release/closeout with L1/L2/L13 fixes).
//! - **Deferred (not in this pass):** `session` (session-lease TTL /
//!   thread-mode / delegate-grant "org chart" engine, `runner.mjs:365-537`),
//!   `repair` (the 3 repair engines + doctor/inspectLedger), `guard` (the
//!   full focused-vs-global-findings truncation-budget engine — this crate
//!   exposes the primitives `blockers_for_request`/`active_claims` guard
//!   would be built on, but not the `guardLedger` orchestration itself),
//!   `notify` (wake-request seen-dedupe), `peers`/`sync` transports (http+
//!   local replication), `read-index` (JSON + opt-in SQLite hot index),
//!   `server`/`daemon` (serve-daemon — explicitly gated to the FULL profile
//!   per the workpack's own decision, tied to arc-22), `runner.rs` (CLI
//!   dispatch + lifecycle-report structured-field validation), and the
//!   `enforcer coordination lane new/park/rm` worktree-spawn primitive.
//!   Each is a disjoint module sub-unit per the workpack's own split and can
//!   be claimed independently without touching the modules landed here.
//! - **Claim-conflict scoping decision (workpack row, EXECUTION_MODEL §2d):**
//!   this port keeps convention (i) — one-hub-per-worktree/lane, conflicts
//!   keyed by bare normalized relative path (not `(worktreeRoot, path)`) —
//!   matching current usage and explicitly NOT implementing (ii). If a
//!   future workpack needs one shared hub across many worktrees with
//!   colliding relative paths, `lock::EnrichedClaim`'s path-key derivation is
//!   the place to add worktree-folding.
//!
//! # Deviation flagged to primary
//! The workpack's own claim/mail tooling used to coordinate THIS build
//! (`ocentra_enforcer_coordination_claim`, the `.mjs` implementation) is the
//! one exhibiting the L13 bug being fixed here in Rust — using it to claim
//! this crate's own files required enumerating exact files and splitting
//! into two ≤10-file claims, exactly the friction this port's
//! `api::claim_all` eliminates for future Rust-hub consumers.

pub mod api;
pub mod domain;
pub mod error;
pub mod events;
pub mod ledger;
pub mod lock;
pub mod sync;

pub use domain::{HubConfig, NodeId, NodeName, WriterId};
pub use error::{CoordinationError, Result};
