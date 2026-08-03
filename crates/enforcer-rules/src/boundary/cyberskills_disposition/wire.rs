//! Serialized CP00 DTO module container.
//!
//! BOUNDARY-INVARIANT: each child owns one cohesive serialized DTO family.
//! NEGATIVE-TEST: the CP00 fixture matrix validates malformed wire records.
//! ROUNDTRIP-TEST: crates/enforcer-rules/tests/cyberskills_disposition/manifest.rs

#[path = "wire/cp08.rs"]
pub mod cp08;
#[path = "wire/implementation.rs"]
pub mod implementation;
#[path = "wire/manifest.rs"]
pub mod manifest;
