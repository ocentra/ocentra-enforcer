//! The two security-family validator sub-modules plus the shared
//! text-scan primitives and data-driven rule-spec plumbing they build on.
//! See `crate` docs for the per-module rule-id breakdown.

pub mod generic_scanner;
pub mod registry;
pub mod secret_scan;
pub mod spec;
pub mod text_scan;
