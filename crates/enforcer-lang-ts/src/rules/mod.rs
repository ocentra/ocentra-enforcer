//! The seven TS-family validator sub-modules plus the shared text-scan
//! primitives and data-driven rule-spec plumbing they build on. See
//! `crate` docs for the per-module rule-id breakdown.

pub mod eslint_json;
pub mod frontend_react;
pub mod generic_scanner;
pub mod import_boundaries;
pub mod registry;
pub mod source_scan;
pub mod spec;
pub mod test_scan;
pub mod tests_family;
pub mod text_scan;
pub mod toolchain;
