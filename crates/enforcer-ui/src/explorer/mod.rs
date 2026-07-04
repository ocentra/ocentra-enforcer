//! g08 mount point — rules-&-skills explorer, where the human-canonical
//! `.md` is browsed by a human while the AI still reads the structured
//! rule.
//!
//! **Ownership**: g08 owns `crates/enforcer-ui/src/explorer/` per
//! `WORKPACK_INDEX.md` (a directory, not this single file; deps `g01`).
//! This workpack (arc-24) only creates this `mod.rs` seam so g08 has
//! somewhere disjoint to land its own sibling files under `explorer/`;
//! it adds no behavior here.
//!
//! Proof row: `proof/ui/g08-explorer.json`
//! (`rules-skills-explorer-contract`) per `TEST_PROOF_EXPECTATIONS.md`.
//! Not proved by this workpack.
