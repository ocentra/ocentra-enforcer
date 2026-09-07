//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Wire contracts owned by the proof crate.

pub mod lifecycle;
pub mod proof_query;
pub mod read_model;
pub mod read_model_claim;
pub mod read_model_journal;
pub mod read_model_run;
