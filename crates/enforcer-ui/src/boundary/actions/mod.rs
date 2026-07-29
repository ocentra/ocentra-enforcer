//! g03 mount point — waiver-honesty actions (a08 waiver shape; no silent
//! suppression).
//! BOUNDARY-INVARIANT: action requests are validated before the named waiver
//! persistence boundary is reached.
//! NEGATIVE-TEST: malformed and incomplete waiver requests are rejected by
//! `file_rule_waiver` before any write.
//! boundaryOwnerNote: enforcer-ui owns the g03 action boundary mount.
//!
//! **Ownership**: g03 owns `crates/enforcer-ui/src/actions/` per
//! `WORKPACK_INDEX.md` (a directory, not this single file). This workpack
//! (arc-24) only creates this `mod.rs` seam so g03 has somewhere disjoint
//! to land its own sibling files under `actions/`; it adds no behavior
//! here. g03 writes a named `.enforce/` waiver row (owner+reason+ruleId)
//! rather than a hidden mute.
//!
//! Proof row: `proof/ui/g03-actions.json` (`waiver-honesty-actions`) per
//! `TEST_PROOF_EXPECTATIONS.md`. Not proved by this workpack.

/// Project-local, file-and-rule waiver persistence.
pub mod file_rule_waiver;
