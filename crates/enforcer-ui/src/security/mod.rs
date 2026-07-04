//! g07 mount point — UI-security surface (loopback-bind assertion +
//! origin/CSRF + dispatch-authorization guards).
//!
//! **Ownership**: g07 owns `crates/enforcer-ui/src/security/*` and
//! `crates/enforcer-ui/tests/ui_security/**` per `WORKPACK_INDEX.md`
//! (deps `g04`). This workpack (arc-24) only creates this `mod.rs` seam
//! (and the empty `tests/ui_security/` mount dir) so g07 has somewhere
//! disjoint to land; it adds no behavior here.
//!
//! Proof row: `proof/ui/g07-security.json` (`ui-security-contract`) per
//! `TEST_PROOF_EXPECTATIONS.md`. Not proved by this workpack.
