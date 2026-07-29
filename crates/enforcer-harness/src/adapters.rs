//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Run-adapter module root. Currently one family: [`cyberskills`] (h12).

pub mod cyberskills;
