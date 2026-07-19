//! MCP boundary d05 context-budget-brake tool-surface measure: enumerates this server's
//! own consolidated tool registry ([`crate::registry::build_tool_descriptors`])
//! and turns it into a [`enforcer_core::context_budget::MeasuredSurface`],
//! ratchets it against the committed baseline, and — best-effort — records
//! the measurement as an NDJSON line.
//!
//! # Ownership seam (workpack "Where We Are")
//! [`crate::registry::tool_surface_bytes`] (arc-21) computes the raw byte
//! count from a descriptor list; THIS module (d05) owns turning that count
//! into a [`enforcer_core::context_budget::MeasuredSurface`], loading/
//! ratcheting the committed baseline, and recording the outcome. Neither
//! side re-implements the other's arithmetic.
//!
//! # d04 telemetry seam (honest scope note)
//! The workpack asks this measure to "record the measured surface into the
//! d04 telemetry `RunRecord`". As of this pass, `d04-run-telemetry-ndjson`
//! has not landed on `rust-build` — no `RunRecord` type exists yet in this
//! workspace. Rather than fabricate a dependency on a type that does not
//! exist, [`record_measurement`] appends a d05-local [`SurfaceMeasurement`]
//! record via the SAME reusable sink [`enforcer_core::ndjson_writer::NdjsonWriter`]
//! that `RunRecord` telemetry is specified to use (see that module's docs:
//! "d04 run-telemetry records and any pack emitting structured records ride
//! this sink"). When d04 lands, folding this record's fields into
//! `RunRecord` (or having `RunRecord` embed a `context_budget:
//! Option<SurfaceMeasurement>` field) is a d04-owned follow-up; this module
//! does not need to change shape for that fold-in, only its call site.

use std::path::Path;

use enforcer_core::context_budget::{decision, efficiency_score, evaluate};
use enforcer_core::error::Result as CoreResult;
use enforcer_core::ndjson_writer::NdjsonWriter;
use enforcer_domain::boundary::core::measured_surface;
use enforcer_domain::core_types::{
    BudgetBaseline, BudgetGateDecision, BudgetGateOutcome, EfficiencyScore, MeasuredSurface,
};

use crate::boundary::surface_measurement::SurfaceMeasurementDto;
use crate::registry::{build_tool_descriptors, tool_surface_bytes};

/// Measure the live tool registry's current surface: tool count + total
/// serialized descriptor bytes (+ derived token estimate). Deterministic —
/// see `registry`'s own `tool_surface_enumeration_is_deterministic` test.
pub fn measure_current_surface() -> MeasuredSurface {
    let descriptors = build_tool_descriptors();
    let total_bytes = tool_surface_bytes(&descriptors);
    measured_surface(descriptors.len(), total_bytes)
}

/// Load a committed [`BudgetBaseline`] from `path`. Fails closed: a
/// missing/corrupt baseline file is an error, never silently treated as "no
/// baseline" (an absent baseline would make the T1 gate vacuously pass,
/// which defeats its purpose).
pub fn load_baseline(path: &Path) -> CoreResult<BudgetBaseline> {
    let payload = std::fs::read(path)?;
    let baseline: BudgetBaseline = serde_json::from_slice(&payload)?;
    Ok(baseline)
}

/// Run the full T1 ratchet: measure the live registry, load the committed
/// baseline from `baseline_path`, and evaluate. This is what the CI
/// `context-budget-scan` job (workpack Acceptance section) and the
/// `cargo test -p enforcer-mcp` proof both call.
pub fn run_gate(baseline_path: &Path) -> CoreResult<BudgetGateOutcome> {
    let measured = measure_current_surface();
    let baseline = load_baseline(baseline_path)?;
    Ok(evaluate(measured, baseline))
}

/// Run the T2 advisory score over the live registry's current surface.
/// Independent of [`run_gate`] — never blocks, never shares a pass/fail
/// verdict with the T1 ratchet.
pub fn run_advisory_score() -> EfficiencyScore {
    efficiency_score(measure_current_surface())
}

/// One recorded tool-surface measurement — the NDJSON line this module
/// appends per run (see the module doc's "d04 telemetry seam" note for why
/// this is a d05-local record shape rather than `RunRecord` itself).
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceMeasurement {
    /// The measured surface at record time.
    surface: MeasuredSurface,
    /// Whether the T1 ratchet passed against the committed baseline at
    /// record time (`None` when no baseline was available to ratchet
    /// against — e.g. a first-ever run before a baseline is committed).
    ratchet_passed: Option<bool>,
    /// The T2 advisory efficiency score at record time.
    efficiency_score: f64,
    /// The T2 advisory score's confidence at record time.
    efficiency_confidence: f64,
}

impl SurfaceMeasurement {
    pub(crate) fn from_boundary(
        surface: MeasuredSurface,
        ratchet_passed: Option<bool>,
        efficiency_score: f64,
        efficiency_confidence: f64,
    ) -> Self {
        Self {
            surface,
            ratchet_passed,
            efficiency_score,
            efficiency_confidence,
        }
    }

    pub(crate) fn surface(&self) -> MeasuredSurface {
        self.surface
    }

    /// Return the recorded ratchet decision, or `None` when no baseline existed.
    pub fn ratchet_passed(&self) -> Option<bool> {
        self.ratchet_passed
    }

    pub(crate) fn efficiency_score(&self) -> f64 {
        self.efficiency_score
    }

    pub(crate) fn efficiency_confidence(&self) -> f64 {
        self.efficiency_confidence
    }
}

/// Build the record for `measured`, folding in an optional ratchet outcome.
pub fn build_measurement(
    measured: MeasuredSurface,
    ratchet_outcome: Option<&BudgetGateOutcome>,
) -> SurfaceMeasurement {
    let score = efficiency_score(measured);
    SurfaceMeasurement {
        surface: measured,
        ratchet_passed: ratchet_outcome
            .map(|outcome| decision(outcome) == BudgetGateDecision::Pass),
        efficiency_score: f64::from(score.score()),
        efficiency_confidence: f64::from(score.confidence()),
    }
}

/// Append one [`SurfaceMeasurement`] to the NDJSON sink at `sink_path`,
/// reusing [`enforcer_core::ndjson_writer::NdjsonWriter`] (append-only,
/// never truncates) rather than a bespoke writer.
pub fn record_measurement(sink_path: &Path, record: &SurfaceMeasurement) -> CoreResult<()> {
    let mut writer: NdjsonWriter<SurfaceMeasurementDto> = NdjsonWriter::open(sink_path)?;
    writer.append(&SurfaceMeasurementDto::from(record))
}

#[cfg(test)]
mod tests {
    use super::{
        build_measurement, measure_current_surface, run_advisory_score, SurfaceMeasurement,
    };
    use crate::boundary::surface_measurement::SurfaceMeasurementDto;
    use enforcer_core::context_budget::{decision, evaluate};
    use enforcer_core::ndjson_writer::read_all;
    use enforcer_domain::boundary::core::{growth_tolerance, measured_surface};
    use enforcer_domain::core_types::{
        BudgetBaseline, BudgetGateDecision, BUDGET_BASELINE_VERSION,
    };

    #[test]
    fn measure_current_surface_matches_registry_byte_count() {
        let measured = measure_current_surface();
        let descriptors = crate::registry::build_tool_descriptors();
        assert_eq!(usize::from(measured.tool_count()), descriptors.len());
        assert_eq!(
            usize::from(measured.total_bytes()),
            crate::registry::tool_surface_bytes(&descriptors)
        );
    }

    #[test]
    fn measure_current_surface_is_deterministic() {
        let first = measure_current_surface();
        let second = measure_current_surface();
        assert_eq!(first, second);
    }

    #[test]
    fn advisory_score_is_in_unit_range_with_confidence() {
        let result = run_advisory_score();
        assert!((0.0..=1.0).contains(&f64::from(result.score())));
        assert!((0.0..=1.0).contains(&f64::from(result.confidence())));
    }

    #[test]
    fn simulated_surface_growth_fixture_fails_the_ratchet() -> Result<(), Box<dyn std::error::Error>>
    {
        // Fixture intent: a baseline pinned far below the live registry's
        // actual surface must fail the ratchet — proves the gate is wired
        // to live measurements rather than a fixed passing value.
        let live = measure_current_surface();
        let starved_baseline = BudgetBaseline::new(
            BUDGET_BASELINE_VERSION,
            measured_surface(usize::from(live.tool_count()), 1),
            growth_tolerance(0.0)?,
        );
        let outcome = evaluate(live, starved_baseline);
        assert!(
            decision(&outcome) == BudgetGateDecision::Fail,
            "a baseline far below the live surface must fail closed"
        );
        Ok(())
    }

    #[test]
    fn measurement_round_trips_through_the_shared_ndjson_sink(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let measured = measured_surface(10, 1_000);
        let record = build_measurement(measured, None);
        let path = std::env::temp_dir().join(format!(
            "enforcer-mcp-tool-surface-{}-{}.ndjson",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        super::record_measurement(&path, &record)?;
        let records: Vec<SurfaceMeasurement> = read_all::<SurfaceMeasurementDto>(&path)?
            .into_iter()
            .map(Into::into)
            .collect();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0], record);
        std::fs::remove_file(&path)?;
        Ok(())
    }

    #[test]
    fn build_measurement_carries_ratchet_outcome_when_present(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let measured = measured_surface(10, 1_000);
        let baseline = BudgetBaseline::new(
            BUDGET_BASELINE_VERSION,
            measured_surface(10, 1_000),
            growth_tolerance(5.0)?,
        );
        let outcome = evaluate(measured, baseline);
        let record = build_measurement(measured, Some(&outcome));
        assert_eq!(record.ratchet_passed(), Some(true));
        Ok(())
    }
}
