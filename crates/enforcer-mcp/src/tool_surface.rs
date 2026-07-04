//! d05 context-budget-brake tool-surface measure: enumerates this server's
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

use enforcer_core::context_budget::{
    efficiency_score, evaluate, BudgetBaseline, BudgetGateOutcome, EfficiencyScore, MeasuredSurface,
};
use enforcer_core::error::Result as CoreResult;
use enforcer_core::ndjson_writer::NdjsonWriter;

use crate::registry::{build_tool_descriptors, tool_surface_bytes};

/// Measure the live tool registry's current surface: tool count + total
/// serialized descriptor bytes (+ derived token estimate). Deterministic —
/// see `registry`'s own `tool_surface_enumeration_is_deterministic` test.
pub fn measure_current_surface() -> MeasuredSurface {
    let descriptors = build_tool_descriptors();
    let total_bytes = tool_surface_bytes(&descriptors);
    MeasuredSurface::from_bytes(descriptors.len(), total_bytes)
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
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceMeasurement {
    /// The measured surface at record time.
    pub surface: MeasuredSurface,
    /// Whether the T1 ratchet passed against the committed baseline at
    /// record time (`None` when no baseline was available to ratchet
    /// against — e.g. a first-ever run before a baseline is committed).
    pub ratchet_passed: Option<bool>,
    /// The T2 advisory efficiency score at record time.
    pub efficiency_score: f64,
    /// The T2 advisory score's confidence at record time.
    pub efficiency_confidence: f64,
}

/// Build the record for `measured`, folding in an optional ratchet outcome.
pub fn build_measurement(
    measured: MeasuredSurface,
    ratchet_outcome: Option<&BudgetGateOutcome>,
) -> SurfaceMeasurement {
    let score = efficiency_score(measured);
    SurfaceMeasurement {
        surface: measured,
        ratchet_passed: ratchet_outcome.map(BudgetGateOutcome::passes),
        efficiency_score: score.score,
        efficiency_confidence: score.confidence,
    }
}

/// Append one [`SurfaceMeasurement`] to the NDJSON sink at `sink_path`,
/// reusing [`enforcer_core::ndjson_writer::NdjsonWriter`] (append-only,
/// never truncates) rather than a bespoke writer.
pub fn record_measurement(sink_path: &Path, record: &SurfaceMeasurement) -> CoreResult<()> {
    let mut writer: NdjsonWriter<SurfaceMeasurement> = NdjsonWriter::open(sink_path)?;
    writer.append(record)
}

#[cfg(test)]
mod tests {
    use super::{
        build_measurement, measure_current_surface, run_advisory_score, SurfaceMeasurement,
    };
    use enforcer_core::context_budget::{evaluate, BudgetBaseline, MeasuredSurface};
    use enforcer_core::ndjson_writer::read_all;

    #[test]
    fn measure_current_surface_matches_registry_byte_count() {
        let measured = measure_current_surface();
        let descriptors = crate::registry::build_tool_descriptors();
        assert_eq!(measured.tool_count, descriptors.len());
        assert_eq!(
            measured.total_bytes,
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
        assert!((0.0..=1.0).contains(&result.score));
        assert!((0.0..=1.0).contains(&result.confidence));
    }

    #[test]
    fn simulated_surface_growth_fixture_fails_the_ratchet() {
        // Fixture intent: a baseline pinned far below the live registry's
        // actual surface must fail the ratchet — proves the gate is wired
        // to real numbers, not a stub that always passes.
        let live = measure_current_surface();
        let starved_baseline = BudgetBaseline {
            version: 1,
            surface: MeasuredSurface::from_bytes(live.tool_count, 1),
            tolerance_pct: 0.0,
        };
        let outcome = evaluate(live, starved_baseline);
        assert!(
            !outcome.passes(),
            "a baseline far below the live surface must fail closed"
        );
    }

    #[test]
    fn measurement_round_trips_through_the_shared_ndjson_sink(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let measured = MeasuredSurface::from_bytes(10, 1_000);
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
        let records: Vec<SurfaceMeasurement> = read_all(&path)?;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0], record);
        std::fs::remove_file(&path)?;
        Ok(())
    }

    #[test]
    fn build_measurement_carries_ratchet_outcome_when_present() {
        let measured = MeasuredSurface::from_bytes(10, 1_000);
        let baseline = BudgetBaseline {
            version: 1,
            surface: MeasuredSurface::from_bytes(10, 1_000),
            tolerance_pct: 5.0,
        };
        let outcome = evaluate(measured, baseline);
        let record = build_measurement(measured, Some(&outcome));
        assert_eq!(record.ratchet_passed, Some(true));
    }
}
