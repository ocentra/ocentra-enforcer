//! g03 mount point — waiver-honesty actions (a08 waiver shape; no silent
//! suppression).
//!
//! **Ownership**: g03 owns `crates/enforcer-ui/src/actions/` per
//! `WORKPACK_INDEX.md` (a directory, not this single file). This workpack
//! (arc-24) only creates this `mod.rs` seam so g03 has somewhere disjoint
//! to land its own sibling files under `actions/`; it adds no behavior
//! here. g03
//! validates request shape through [`crate::payload::validate_action_
//! request`] (arc-24-owned) before any write, and must write a NAMED
//! `.enforce/` waiver row (owner+reason+ruleId) rather than a hidden
//! mute.
//!
//! Proof row: `proof/ui/g03-actions.json` (`waiver-honesty-actions`) per
//! `TEST_PROOF_EXPECTATIONS.md`. Not proved by this workpack.
