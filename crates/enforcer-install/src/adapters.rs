//! Per-harness [`crate::core::HarnessAdapter`] implementations.
//!
//! # Ownership (mount-point deviation — see module docs, `src/lib.rs`)
//!
//! This module file itself is not listed in any single Track C workpack's
//! `owns:` line (the crate skeleton/arc-23 documents the mount point but
//! does not create it). c07 creates this barrel as a minimal, additive
//! `pub mod` declaration list (not a `pub use` re-export barrel, so it
//! stays inside `[workspace.lints]`) so its own
//! [`generic`] adapter has somewhere to live; sibling packs (c03 `claude`,
//! c06 `codex`, c08 `gemini`/`cursor`/`zed`, c09 the remaining six) add
//! their own `pub mod <harness>;` line here — disjoint statements, no
//! file-content collision expected between lanes.
pub mod generic;
