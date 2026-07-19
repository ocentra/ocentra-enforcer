//! MCP transport boundaries.
//!
//! BOUNDARY-INVARIANT: only this module tree owns raw MCP JSON and stdio
//! transport values; conversions yield canonical domain values immediately.
//! boundaryOwnerNote: enforcer-mcp owns MCP transport decoding and encoding.
//!
//! Raw JSON and protocol DTOs are isolated here, then decode into the
//! canonical values owned by `enforcer-domain`.
//! NEGATIVE-TEST: each DTO module below rejects malformed or invalid input.

pub mod fingerprint;
pub mod fingerprint_artifact;
pub mod rpc_request;
pub mod rpc_response;
pub mod staleness_report;
pub mod surface_measurement;
pub mod tool_descriptor;
