//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Raw JSON contracts for canonical core values.

use crate::boundary::decode_error::DecodeError;
use crate::core_types::{
    BudgetBaselineVersion, ChainEntryCount, ChainLinkIndex, EstimatedTokenCount,
    GrowthTolerancePct, MeasuredSurface, ToolDescriptorCount, ToolSurfaceByteCount,
    ToolSurfaceByteDelta, ToolSurfaceGrowthPct, UnitInterval,
};
use crate::memory_types::StoreCacheInstant;

/// Convert a monotonic clock reading at the cache-clock boundary.
pub fn store_cache_instant(value: std::time::Instant) -> StoreCacheInstant {
    StoreCacheInstant(value)
}

/// Mint a measured surface from raw registry/serialization counts.
pub const fn measured_surface(tool_count: usize, total_bytes: usize) -> MeasuredSurface {
    MeasuredSurface::from_canonical_counts(
        ToolDescriptorCount(tool_count),
        ToolSurfaceByteCount(total_bytes),
        EstimatedTokenCount(total_bytes / 4),
    )
}

/// Validate a raw context-budget growth tolerance.
pub fn growth_tolerance(value: f64) -> Result<GrowthTolerancePct, DecodeError> {
    if value.is_finite() && value >= 0.0 {
        Ok(GrowthTolerancePct(value))
    } else {
        Err(DecodeError::new(
            "budgetBaseline.tolerancePct",
            "must be a finite non-negative percentage",
        ))
    }
}

/// Mint an exact signed delta produced by budget arithmetic.
pub const fn tool_surface_byte_delta(value: i64) -> ToolSurfaceByteDelta {
    ToolSurfaceByteDelta(value)
}

/// Mint a growth percentage, normalizing indeterminate arithmetic to failure.
pub fn tool_surface_growth_pct(value: f64) -> ToolSurfaceGrowthPct {
    if value.is_nan() {
        ToolSurfaceGrowthPct(f64::INFINITY)
    } else {
        ToolSurfaceGrowthPct(value)
    }
}

/// Clamp an advisory mechanism value into the canonical unit interval.
pub fn unit_interval(value: f64) -> UnitInterval {
    let finite = if value.is_finite() { value } else { 0.0 };
    UnitInterval(finite.clamp(0.0, 1.0))
}

/// Mint an exact zero-based chain position.
pub const fn chain_link_index(value: usize) -> ChainLinkIndex {
    ChainLinkIndex(value)
}

/// Mint an exact non-negative chain entry count.
pub const fn chain_entry_count(value: usize) -> ChainEntryCount {
    ChainEntryCount(value)
}

/// Transport shape decoded before constructing a measured surface.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MeasuredSurfaceWire {
    pub(crate) tool_count: usize,
    pub(crate) total_bytes: usize,
    pub(crate) estimated_tokens: usize,
}

/// Transport shape decoded before constructing a budget baseline.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BudgetBaselineWire {
    pub(crate) version: BudgetBaselineVersion,
    pub(crate) surface: MeasuredSurface,
    pub(crate) tolerance_pct: GrowthTolerancePct,
}
