//! Context-budget ratchet behavior over canonical domain values.
//!
//! `enforcer-domain` owns the record/value shapes. This module owns the
//! arithmetic: a fail-closed growth ratchet plus a non-blocking efficiency
//! score.

use enforcer_domain::boundary::core::{
    tool_surface_byte_delta, tool_surface_growth_pct, unit_interval,
};
use enforcer_domain::core_types::{
    BudgetBaseline, BudgetGateDecision, BudgetGateOutcome, EfficiencyScore, MeasuredSurface,
    UnitInterval,
};

/// Target ceiling in bytes per tool for the advisory score.
pub const BYTES_PER_TOOL_CEILING: f64 = 2_000.0;

/// Decide whether an evaluated outcome remains within the committed tolerance.
pub fn decision(outcome: &BudgetGateOutcome) -> BudgetGateDecision {
    if f64::from(outcome.growth_pct()) <= f64::from(outcome.baseline().tolerance_pct()) {
        BudgetGateDecision::Pass
    } else {
        BudgetGateDecision::Fail
    }
}

/// Ratchet a fresh measurement against a committed baseline.
pub fn evaluate(measured: MeasuredSurface, baseline: BudgetBaseline) -> BudgetGateOutcome {
    let measured_bytes = usize::from(measured.total_bytes());
    let baseline_bytes = usize::from(baseline.surface().total_bytes());
    // BRAND-INVARIANT: arithmetic is saturated into the exact signed delta
    // range so an unrepresentable count can never wrap into a passing value.
    let byte_delta = if measured_bytes >= baseline_bytes {
        i64::try_from(measured_bytes - baseline_bytes).unwrap_or(i64::MAX)
    } else {
        i64::try_from(baseline_bytes - measured_bytes).map_or(i64::MIN, |difference| -difference)
    };
    // BRAND-INVARIANT: positive growth over a zero baseline is represented as
    // infinity and therefore fails the ratchet; zero over zero remains zero.
    let growth_pct = if baseline_bytes == 0 {
        if byte_delta > 0 {
            f64::INFINITY
        } else {
            0.0
        }
    } else {
        // CAST-JUSTIFICATION: both operands are exact byte counts and the
        // quotient is intentionally an approximate percentage for reporting.
        (byte_delta as f64 / baseline_bytes as f64) * 100.0
    };

    BudgetGateOutcome::new(
        measured,
        baseline,
        tool_surface_byte_delta(byte_delta),
        tool_surface_growth_pct(growth_pct),
    )
}

/// Compute the advisory efficiency score for a measured surface.
pub fn efficiency_score(measured: MeasuredSurface) -> EfficiencyScore {
    let tool_count = usize::from(measured.tool_count());
    if tool_count == 0 {
        return EfficiencyScore::from_intervals(UnitInterval::ZERO, UnitInterval::ZERO);
    }
    // CAST-JUSTIFICATION: byte and tool counts are converted to floating
    // point solely to calculate a bounded advisory ratio, never an exact ID.
    let bytes_per_tool = usize::from(measured.total_bytes()) as f64 / tool_count as f64;
    let raw = 1.0 - (bytes_per_tool / BYTES_PER_TOOL_CEILING);
    EfficiencyScore::from_intervals(unit_interval(raw), UnitInterval::ONE)
}

#[cfg(test)]
mod tests {
    use super::{decision, efficiency_score, evaluate};
    use enforcer_domain::boundary::core::{growth_tolerance, measured_surface};
    use enforcer_domain::core_types::{
        BudgetBaseline, BudgetGateDecision, GrowthTolerancePct, MeasuredSurface,
        BUDGET_BASELINE_VERSION,
    };

    fn baseline(surface: MeasuredSurface, tolerance_pct: GrowthTolerancePct) -> BudgetBaseline {
        BudgetBaseline::new(BUDGET_BASELINE_VERSION, surface, tolerance_pct)
    }

    #[test]
    fn identical_surface_passes_with_zero_growth(
    ) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        let outcome = evaluate(
            measured_surface(10, 1_000),
            baseline(measured_surface(10, 1_000), growth_tolerance(5.0)?),
        );
        assert_eq!(decision(&outcome), BudgetGateDecision::Pass);
        assert_eq!(i64::from(outcome.byte_delta()), 0);
        assert_eq!(f64::from(outcome.growth_pct()), 0.0);
        Ok(())
    }

    #[test]
    fn shrink_always_passes() -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        let outcome = evaluate(
            measured_surface(10, 800),
            baseline(measured_surface(10, 1_000), growth_tolerance(0.0)?),
        );
        assert_eq!(decision(&outcome), BudgetGateDecision::Pass);
        assert!(i64::from(outcome.byte_delta()) < 0);
        Ok(())
    }

    #[test]
    fn growth_within_tolerance_passes(
    ) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        let outcome = evaluate(
            measured_surface(10, 1_040),
            baseline(measured_surface(10, 1_000), growth_tolerance(5.0)?),
        );
        assert_eq!(decision(&outcome), BudgetGateDecision::Pass);
        Ok(())
    }

    #[test]
    fn growth_beyond_tolerance_fails(
    ) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        let outcome = evaluate(
            measured_surface(10, 1_200),
            baseline(measured_surface(10, 1_000), growth_tolerance(5.0)?),
        );
        assert_eq!(decision(&outcome), BudgetGateDecision::Fail);
        assert_eq!(i64::from(outcome.byte_delta()), 200);
        Ok(())
    }

    #[test]
    fn zero_byte_baseline_fails_on_growth_and_passes_without_growth(
    ) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        assert_eq!(
            decision(&evaluate(
                measured_surface(1, 10),
                baseline(measured_surface(0, 0), growth_tolerance(5.0)?,)
            )),
            BudgetGateDecision::Fail
        );
        let unchanged = evaluate(
            measured_surface(0, 0),
            baseline(measured_surface(0, 0), growth_tolerance(5.0)?),
        );
        assert_eq!(decision(&unchanged), BudgetGateDecision::Pass);
        assert_eq!(f64::from(unchanged.growth_pct()), 0.0);
        Ok(())
    }

    #[test]
    fn efficiency_score_is_bounded_and_confident_for_nonempty_surfaces() {
        let result = efficiency_score(measured_surface(10, 1_000));
        assert!((0.0..=1.0).contains(&f64::from(result.score())));
        assert_eq!(f64::from(result.confidence()), 1.0);
    }

    #[test]
    fn efficiency_score_has_zero_confidence_for_empty_surface() {
        let result = efficiency_score(measured_surface(0, 0));
        assert_eq!(f64::from(result.score()), 0.0);
        assert_eq!(f64::from(result.confidence()), 0.0);
    }

    #[test]
    fn efficiency_score_floors_for_bloated_surface_and_rewards_lean_surface() {
        let bloated = efficiency_score(measured_surface(1, 1_000_000));
        assert_eq!(f64::from(bloated.score()), 0.0);
        let lean = efficiency_score(measured_surface(50, 5_000));
        assert!(f64::from(lean.score()) > 0.9);
    }
}
