//! Harness-neutral `/`-command emitters: install-time artifacts that hand a
//! harness a first-class slash/command surface dispatching to a real
//! `enforcer` subcommand, rather than a per-harness hand-written script.
//!
//! # Ownership (mount-point deviation — see [`crate::adapters`]'s module
//! doc for the same pattern)
//!
//! This barrel file is not in any single workpack's `owns:` line; b05
//! creates it as a minimal `pub mod` declaration list (not a `pub use`
//! re-export barrel) so its owned `plan` module has somewhere to live. A
//! future command emitter adds its own `pub mod` line here without
//! touching this one.
pub mod plan;
