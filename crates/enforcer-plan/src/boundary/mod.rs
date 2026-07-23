//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Raw plan input decoding and output construction boundaries.

pub(crate) mod finding;
pub(crate) mod forest;
pub(crate) mod lessons;
pub(crate) mod scaffolder;
pub(crate) mod validator;
pub(crate) mod values;
