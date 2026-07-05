//! CFML/ColdFusion rule families, one module per checklist group. See each
//! module's doc comment for its `RuleId` set; [`crate::all_validators`]
//! composes every module's `all()` into the single vec this crate
//! registers.
//!
//! The `T3` (advisory, no-mechanization-possible) row carries no
//! `Validator` at all by design -- the d01 parity oracle
//! (`enforcer_mechanization::parity`) checks only that its registry
//! record's `tags` carries the verbatim
//! `advisory, no mechanization possible + <reason>` label, never a
//! fixture/detection pair. That row (`CF-ARCH-6.1`) exists only as a
//! record in `crates/enforcer-rules/rules/cfml-advisory.json`, not as Rust
//! code in this module tree.

pub mod arch;
pub mod cflint_adapter;
pub mod err;
pub mod security;
pub mod style;
mod support;
pub mod toolchain;
