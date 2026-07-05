//! Dart/Flutter rule families, one module per checklist group. See each
//! module's doc comment for its `RuleId` set; [`all_validators`] in the
//! crate root composes every module's `all()` into the single vec this
//! crate registers.
//!
//! `T3` (advisory, no-mechanization-possible) rows carry no `Validator`
//! at all by design — the d01 parity oracle
//! (`enforcer_mechanization::parity`) checks only that their registry
//! record's `tags` carries the verbatim
//! `advisory, no mechanization possible + <reason>` label, never a
//! fixture/detection pair. Those three rows
//! (`DART-NAME-3.1`/`DART-IMP-2.1`/`DART-STATE-2.1`) exist only as
//! records in `crates/enforcer-rules/rules/dart-advisory.json`, not as
//! Rust code in this module tree.

pub mod arch;
pub mod err;
pub mod naming;
pub mod security;
pub mod state;
mod support;
pub mod toolchain;
pub mod types;
pub mod widget;
