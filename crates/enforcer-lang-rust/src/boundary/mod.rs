//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Source-parser boundary adapters.

pub(crate) mod finding;
#[cfg(test)]
pub(crate) mod fixture;
pub(crate) mod syntax;
