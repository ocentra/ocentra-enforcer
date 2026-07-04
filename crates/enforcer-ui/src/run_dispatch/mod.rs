//! g04 mount point — run-dispatch (fix-intent schema at the boundary;
//! ledger write via arc-16 `enforcer-coordination`).
//!
//! **Ownership**: g04 owns `crates/enforcer-ui/src/run_dispatch/` per
//! `WORKPACK_INDEX.md` (a directory, not this single file; deps
//! `arc-16`). This workpack (arc-24) only creates this `mod.rs` seam so
//! g04 has somewhere disjoint to land its own sibling files under
//! `run_dispatch/`; it adds no behavior here. g04 validates the Run
//! payload through
//! [`crate::payload::validate_action_request`] (arc-24-owned) before any
//! ledger write, and must dedupe on ruleId+files rather than forking a
//! lane per click.
//!
//! Proof row: `proof/ui/g04-run-dispatch.json` (`run-dispatch-intent`)
//! per `TEST_PROOF_EXPECTATIONS.md`. Not proved by this workpack.
