//! Transport-only values accepted or emitted at package boundaries.

// BOUNDARY-INVARIANT: this module contains only external decode contracts;
// validated domain values cross the boundary only after negative checks pass.
// boundaryOwnerNote: enforcer-domain owns the shared transport decode boundary.
// Negative malformed and invalid inputs are covered by the crate's boundary tests.

pub mod core;
pub mod decode_error;
pub mod hash;
pub mod json;
pub mod mcp;
mod memory_traits;
mod records;
pub(crate) mod scan;
mod security;
pub mod validation;
