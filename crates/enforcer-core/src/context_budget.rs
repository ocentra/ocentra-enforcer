//! d05 context-budget ratchet: a fail-closed T1 gate over a measured MCP
//! tool-surface size, plus an advisory T2 efficiency score.
//!
//! This module is deliberately generic over WHAT is measured — it knows
//! nothing about `enforcer-mcp`'s tool registry (that would invert the
//! dependency graph: `enforcer-mcp` depends on `enforcer-core`, never the
//! reverse). The caller (`enforcer_mcp::tool_surface`) enumerates its own
//! Rust tool registry and hands this module a plain [`MeasuredSurface`];
//! this module owns only the ratchet-against-baseline arithmetic and the
//! committed-baseline wire format.
//!
//! # Contract
//! - [`MeasuredSurface`]: tool count + total description bytes + a token
//!   estimate (bytes / 4, the common rough heuristic — no tokenizer
//!   dependency belongs in `enforcer-core`).
//! - [`BudgetBaseline`]: the committed, versioned wire form a reviewer
//!   edits deliberately (never rewritten silently by a running check).
//! - [`evaluate`]: T1 hard ratchet — growth beyond `tolerance_pct` over the
//!   committed baseline fails closed; growth within tolerance, or any
//!   shrink, passes. A [`BudgetBaseline`] is otherwise immutable input:
//!   this module never returns an "updated baseline to write", by design
//!   (the workpack requires baseline updates to be an explicit, reviewed
//!   commit, never a silent side effect of running the gate).
//! - [`efficiency_score`]: T2 advisory score in `[0, 1]` with a confidence,
//!   non-blocking, independent of the T1 outcome.

use serde::{Deserialize, Serialize};

/// One measured tool-surface snapshot: how many tools are registered, how
/// many bytes their serialized descriptors occupy, and a rough token-count
/// estimate for that same payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeasuredSurface {
    /// Count of registered tool descriptors (canonical + any live aliases).
    pub tool_count: usize,
    /// Total UTF-8 byte length of the serialized tool descriptor list.
    pub total_bytes: usize,
    /// Rough token estimate for `total_bytes` (bytes / 4; no tokenizer
    /// dependency belongs in this foundation crate — callers needing an
    /// exact count own their own tokenizer).
    pub estimated_tokens: usize,
}

impl MeasuredSurface {
    /// Build a measured surface from a tool count and byte length,
    /// deriving `estimated_tokens` via the bytes/4 heuristic.
    pub fn from_bytes(tool_count: usize, total_bytes: usize) -> Self {
        Self {
            tool_count,
            total_bytes,
            estimated_tokens: total_bytes / 4,
        }
    }
}

/// Schema version of [`BudgetBaseline`]'s on-disk wire form. Bump on any
/// breaking change to the record shape so an old baseline file fails to
/// load loudly instead of silently misinterpreting.
pub const BUDGET_BASELINE_VERSION: u32 = 1;

/// The committed, reviewed baseline a fresh [`MeasuredSurface`] is ratcheted
/// against. This is intentionally a plain, hand-editable JSON document (no
/// integrity hash) — the workpack requires baseline updates to be an
/// "explicit, reviewed commit", i.e. a visible diff a reviewer signs off on,
/// not a tamper-evident but opaque blob.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetBaseline {
    /// Schema version; see [`BUDGET_BASELINE_VERSION`].
    pub version: u32,
    /// The committed reference surface.
    pub surface: MeasuredSurface,
    /// Growth tolerance, as a percentage of `surface.total_bytes` (e.g.
    /// `10.0` permits up to 10% growth before the T1 gate fails closed).
    pub tolerance_pct: f64,
}

/// The outcome of ratcheting a fresh [`MeasuredSurface`] against a
/// committed [`BudgetBaseline`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BudgetGateOutcome {
    /// The freshly measured surface that was evaluated.
    pub measured: MeasuredSurface,
    /// The committed baseline it was evaluated against.
    pub baseline: BudgetBaseline,
    /// Byte growth over the baseline (`measured.total_bytes as i64 -
    /// baseline.surface.total_bytes as i64`); negative on a shrink.
    pub byte_delta: i64,
    /// `byte_delta` expressed as a percentage of the baseline's
    /// `total_bytes` (0.0 when the baseline was itself zero bytes).
    pub growth_pct: f64,
}

impl BudgetGateOutcome {
    /// True when `growth_pct` is within `baseline.tolerance_pct` — i.e. the
    /// gate passes. A shrink (negative `growth_pct`) always passes.
    pub fn passes(&self) -> bool {
        self.growth_pct <= self.baseline.tolerance_pct
    }
}

/// Ratchet `measured` against `baseline`. Fails closed: growth beyond
/// `baseline.tolerance_pct` reports [`BudgetGateOutcome::passes`] as
/// `false`. This function performs NO I/O and returns NO updated baseline
/// to persist — baseline updates are an explicit, separately reviewed
/// commit, never a side effect of evaluating the gate.
pub fn evaluate(measured: MeasuredSurface, baseline: BudgetBaseline) -> BudgetGateOutcome {
    let byte_delta = measured.total_bytes as i64 - baseline.surface.total_bytes as i64;
    let growth_pct = if baseline.surface.total_bytes == 0 {
        if byte_delta > 0 {
            f64::INFINITY
        } else {
            0.0
        }
    } else {
        (byte_delta as f64 / baseline.surface.total_bytes as f64) * 100.0
    };
    BudgetGateOutcome {
        measured,
        baseline,
        byte_delta,
        growth_pct,
    }
}

/// T2 advisory efficiency score in `[0, 1]`: bytes-per-tool relative to a
/// target ceiling, higher is better (a lean, well-scoped tool surface
/// scores close to `1.0`; a bloated one approaches `0.0`). Non-blocking —
/// this score never participates in [`BudgetGateOutcome::passes`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EfficiencyScore {
    /// The score itself, always in `[0.0, 1.0]`.
    pub score: f64,
    /// Confidence in the score, always in `[0.0, 1.0]`. Confidence is
    /// pinned to `1.0` when at least one tool is registered (the
    /// arithmetic is exact, not a heuristic estimate over a sample) and to
    /// `0.0` for an empty surface (score is meaningless with zero tools).
    pub confidence: f64,
}

/// Target ceiling (bytes/tool) at or above which [`efficiency_score`]
/// floors out at `0.0`. Chosen as a generous, documented advisory
/// threshold — this is a T2 signal, not a hard limit; the T1 gate is
/// [`evaluate`], not this function.
pub const BYTES_PER_TOOL_CEILING: f64 = 2_000.0;

/// Compute the T2 advisory efficiency score for `measured`.
pub fn efficiency_score(measured: MeasuredSurface) -> EfficiencyScore {
    if measured.tool_count == 0 {
        return EfficiencyScore {
            score: 0.0,
            confidence: 0.0,
        };
    }
    let bytes_per_tool = measured.total_bytes as f64 / measured.tool_count as f64;
    let raw = 1.0 - (bytes_per_tool / BYTES_PER_TOOL_CEILING);
    let score = raw.clamp(0.0, 1.0);
    EfficiencyScore {
        score,
        confidence: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        efficiency_score, evaluate, BudgetBaseline, MeasuredSurface, BUDGET_BASELINE_VERSION,
    };

    fn baseline(total_bytes: usize, tolerance_pct: f64) -> BudgetBaseline {
        BudgetBaseline {
            version: BUDGET_BASELINE_VERSION,
            surface: MeasuredSurface::from_bytes(10, total_bytes),
            tolerance_pct,
        }
    }

    #[test]
    fn identical_surface_passes_with_zero_growth() {
        let measured = MeasuredSurface::from_bytes(10, 1_000);
        let outcome = evaluate(measured, baseline(1_000, 5.0));
        assert!(outcome.passes());
        assert_eq!(outcome.byte_delta, 0);
        assert_eq!(outcome.growth_pct, 0.0);
    }

    #[test]
    fn shrink_always_passes() {
        let measured = MeasuredSurface::from_bytes(10, 800);
        let outcome = evaluate(measured, baseline(1_000, 0.0));
        assert!(outcome.passes());
        assert!(outcome.byte_delta < 0);
    }

    #[test]
    fn growth_within_tolerance_passes() {
        let measured = MeasuredSurface::from_bytes(10, 1_040);
        let outcome = evaluate(measured, baseline(1_000, 5.0));
        assert!(outcome.passes(), "4% growth must pass a 5% tolerance");
    }

    #[test]
    fn simulated_surface_growth_beyond_tolerance_fails_the_ratchet() {
        let measured = MeasuredSurface::from_bytes(10, 1_200);
        let outcome = evaluate(measured, baseline(1_000, 5.0));
        assert!(
            !outcome.passes(),
            "20% growth must fail closed against a 5% tolerance"
        );
        assert_eq!(outcome.byte_delta, 200);
    }

    #[test]
    fn zero_byte_baseline_with_any_growth_fails_closed() {
        let measured = MeasuredSurface::from_bytes(1, 10);
        let outcome = evaluate(measured, baseline(0, 5.0));
        assert!(
            !outcome.passes(),
            "any growth over a zero-byte baseline must fail closed, not divide-by-zero pass"
        );
    }

    #[test]
    fn zero_byte_baseline_with_no_growth_passes() {
        let measured = MeasuredSurface::from_bytes(0, 0);
        let outcome = evaluate(measured, baseline(0, 5.0));
        assert!(outcome.passes());
        assert_eq!(outcome.growth_pct, 0.0);
    }

    #[test]
    fn efficiency_score_is_in_unit_range_with_confidence() {
        let measured = MeasuredSurface::from_bytes(10, 1_000);
        let result = efficiency_score(measured);
        assert!((0.0..=1.0).contains(&result.score));
        assert!((0.0..=1.0).contains(&result.confidence));
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn efficiency_score_is_zero_confidence_for_empty_surface() {
        let measured = MeasuredSurface::from_bytes(0, 0);
        let result = efficiency_score(measured);
        assert_eq!(result.score, 0.0);
        assert_eq!(result.confidence, 0.0);
    }

    #[test]
    fn efficiency_score_floors_at_zero_for_bloated_surface() {
        let measured = MeasuredSurface::from_bytes(1, 1_000_000);
        let result = efficiency_score(measured);
        assert_eq!(result.score, 0.0);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn efficiency_score_is_high_for_lean_surface() {
        let measured = MeasuredSurface::from_bytes(50, 5_000);
        let result = efficiency_score(measured);
        assert!(result.score > 0.9, "100 bytes/tool should score near 1.0");
    }
}
