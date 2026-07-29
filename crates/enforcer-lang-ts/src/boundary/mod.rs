//! TypeScript analyzer boundaries.
//!
//! BOUNDARY-INVARIANT: each child module must parse one owned raw input boundary and
//! converts accepted input before rule policy is evaluated.
//! boundaryOwnerNote: enforcer-lang-ts owns these analyzer boundaries.
//! Negative invalid-input coverage lives beside each concrete decoder.

pub(crate) mod finding;
pub(crate) mod rule_spec;
pub(crate) mod source_analysis;
pub(crate) mod source_text;
#[cfg(test)]
pub(crate) mod test_fixtures;
pub(crate) mod toolchain_policy;
