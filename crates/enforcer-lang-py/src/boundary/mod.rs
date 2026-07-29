//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Python source and fixture conversion boundaries.

pub(crate) mod finding;
#[cfg(test)]
pub(crate) mod fixture;
pub(crate) mod line_marker;
pub(crate) mod source;
