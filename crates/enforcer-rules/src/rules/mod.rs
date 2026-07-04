//! Typed Rust rule-record constructors, one module per rule family that
//! ships its records as Rust code rather than (or in addition to) a
//! `rules/*.json` catalog file. The baseline T1 catalogs
//! (`deny-wall`/`no-reexports`/`ocentra-parent-posture`) stay JSON-only
//! under `rules/**`; a family whose records need to reference a sibling
//! crate's typed constants (as `plan` does — RuleId literals shared with
//! `enforcer-plan`'s validator module) owns a `src/rules/<family>.rs` file
//! instead.
//!
//! - [`plan`] — the `PLAN-*` structure-validator records (b02).

pub mod plan;
