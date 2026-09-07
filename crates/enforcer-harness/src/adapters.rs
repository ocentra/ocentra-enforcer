//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Run-adapter module root. Families reuse the shared bounded execution and
//! diagnostic contracts rather than creating another process runner.

pub mod cargo;
pub mod cyberskills;
