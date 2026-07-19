//! NDJSON wire shape for MCP tool-surface telemetry.
//!
//! BOUNDARY-INVARIANT: serde belongs to this DTO; the measurement used by
//! the context-budget core remains a serde-free value.

use enforcer_domain::core_types::MeasuredSurface;

use crate::tool_surface::SurfaceMeasurement;

// ROUNDTRIP-TEST: crates/enforcer-mcp/tests/wire_roundtrip.rs::surface_measurement_dto_round_trip_preserves_scores

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
/// Serialized tool-surface measurement written to the NDJSON telemetry seam.
pub struct SurfaceMeasurementDto {
    pub surface: MeasuredSurface,
    pub ratchet_passed: Option<bool>,
    pub efficiency_score: f64,
    pub efficiency_confidence: f64,
}

impl From<&SurfaceMeasurement> for SurfaceMeasurementDto {
    // NEGATIVE-TEST: crates/enforcer-mcp/tests/wire_roundtrip.rs::
    // malformed_surface_measurement_is_rejected_before_domain_conversion
    fn from(value: &SurfaceMeasurement) -> Self {
        Self {
            surface: value.surface(),
            ratchet_passed: value.ratchet_passed(),
            efficiency_score: value.efficiency_score(),
            efficiency_confidence: value.efficiency_confidence(),
        }
    }
}

impl From<SurfaceMeasurementDto> for SurfaceMeasurement {
    fn from(value: SurfaceMeasurementDto) -> Self {
        SurfaceMeasurement::from_boundary(
            value.surface,
            value.ratchet_passed,
            value.efficiency_score,
            value.efficiency_confidence,
        )
    }
}
