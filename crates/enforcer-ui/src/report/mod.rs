//! g02 mount point — report view (violation-matrix completeness +
//! grouping keys + silent-mode suppression).
//!
//! **Ownership**: g02 owns `crates/enforcer-ui/src/report/` per
//! `WORKPACK_INDEX.md` (a directory, not this single file). This workpack
//! (arc-24) only creates this `mod.rs` seam so g02 has somewhere disjoint
//! to land its own sibling files under `report/`; it adds no behavior
//! here. g02 renders through [`crate::payload::UiReportPayload`]
//! (arc-24-owned) rather than re-deriving its own report shape.
//!
//! Proof row: `proof/ui/g02-report.json` (`report-view-contract`) per
//! `TEST_PROOF_EXPECTATIONS.md`. Not proved by this workpack.
