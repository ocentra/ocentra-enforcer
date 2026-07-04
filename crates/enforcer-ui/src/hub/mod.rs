//! g06 mount point — live lane/claim/lease/mail hub panel (read-only view
//! over arc-16 `enforcer-coordination` materialized state).
//!
//! **Ownership**: g06 owns `crates/enforcer-ui/src/hub/` per
//! `WORKPACK_INDEX.md` (a directory, not this single file; deps
//! `arc-16`). This workpack (arc-24) only creates this `mod.rs` seam so
//! g06 has somewhere disjoint to land its own sibling files under
//! `hub/`; it adds no behavior here. g06 must
//! never issue a mutating call against the coordination API from this
//! panel (read-only by contract).
//!
//! Proof row: `proof/ui/g06-hub.json` (`hub-dashboard-mount`) per
//! `TEST_PROOF_EXPECTATIONS.md`. Not proved by this workpack.
