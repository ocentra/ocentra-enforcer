//! JSON and filesystem ingress for rule-registry data.
//!
//! BOUNDARY-INVARIANT: wire values exist only while decoding external JSON or
//! filesystem input and convert exactly once into canonical rule values.
//! boundaryOwnerNote: enforcer-rules owns these catalog, manifest, and waiver boundaries.
//! Negative invalid, empty, oversized, and malformed input coverage is exercised by
//! the boundary-specific unit and integration tests.

pub mod registry;
pub mod version_drift;
#[path = "waiver_json.rs"]
pub mod waiver;
