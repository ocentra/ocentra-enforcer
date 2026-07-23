//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! External transport and persistence DTOs for the scan crate.
//!
//! These modules own raw serde/persistence shapes.  The scan engine consumes
//! canonical `enforcer_domain` values after the boundary conversion; no DTO
//! barrel or re-export shim is provided.

pub mod baseline;
pub mod coverage;
pub mod modes;
pub mod onboard;
pub mod router;
