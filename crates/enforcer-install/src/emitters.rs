//! Harness-neutral install-time emitters: writers `init`/`buildInitWrites`
//! (the legacy engine) produced that no single harness [`crate::core::HarnessAdapter`]
//! owns, because the artifact they write is consumer-repo-side (a
//! `.github/workflows/*.yml` set, a pre-commit hook flavor) rather than a
//! harness's own home-directory config.
//!
//! # Ownership (mount-point deviation — see module docs, `src/lib.rs`)
//!
//! Same deviation note as [`crate::adapters`]: this barrel file is not in
//! any workpack's `owns:` line; c07 creates it as a minimal `pub mod`
//! declaration list (not a `pub use` re-export barrel) so its two owned
//! emitter modules have somewhere to live.
pub mod consumer_ci;
pub mod git_hooks;
