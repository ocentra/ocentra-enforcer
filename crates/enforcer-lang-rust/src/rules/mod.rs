//! Module root for the Rust-family `Validator` implementations.
//!
//! arc-06 owns this module root and the two hosted baseline validators
//! ([`no_reexports`], [`error_handling`]). Sibling Rust-rule feature packs
//! (the `RR-*` prefix inventory) add further `pub mod <name>;` lines here
//! for their own `src/rules/<name>.rs` files — this file is a registration
//! point, not a barrel: it declares submodules, it does not re-export their
//! contents (`pub use` is exactly what [`no_reexports`] bans).

pub mod error_handling;
pub mod no_reexports;
