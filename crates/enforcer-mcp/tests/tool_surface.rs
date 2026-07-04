//! d05 (context-budget brake) proof row: `cargo test -p enforcer-mcp`
//! (`tests/tool_surface.rs` + `tests/fixtures/tool_surface/**`).
//!
//! Per `docs/plans/enforcer-selfhost-plan/workpacks/d05-context-budget-brake.md`'s
//! Acceptance section, this proves:
//! - measurement determinism (mirrors the unit test in `src/tool_surface.rs`,
//!   re-asserted here as a black-box integration check against the crate's
//!   public surface);
//! - a simulated surface-growth fixture fails the T1 ratchet;
//! - the T2 score is in `[0,1]` with confidence;
//! - the COMMITTED baseline (`crates/enforcer-mcp/context-budget-baseline.json`)
//!   is loadable, well-formed, and the live registry currently passes it
//!   (the "declarative committed policy" checklist item: the checked-in
//!   baseline must actually describe the current surface, not a stale one);
//! - a corrupt/malformed baseline fails closed rather than being silently
//!   treated as "no baseline".

use std::path::{Path, PathBuf};

use enforcer_mcp::tool_surface::{load_baseline, measure_current_surface, run_advisory_score};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture(name: &str) -> PathBuf {
    manifest_dir()
        .join("tests")
        .join("fixtures")
        .join("tool_surface")
        .join(name)
}

fn committed_baseline_path() -> PathBuf {
    manifest_dir().join("context-budget-baseline.json")
}

#[test]
fn measurement_is_deterministic_across_calls() {
    let first = measure_current_surface();
    let second = measure_current_surface();
    assert_eq!(
        first, second,
        "the same process must measure the same surface twice"
    );
    assert!(
        first.tool_count > 0,
        "the registry must register at least one tool"
    );
    assert!(first.total_bytes > 0);
}

#[test]
fn simulated_surface_growth_fixture_fails_the_t1_ratchet() -> Result<(), Box<dyn std::error::Error>>
{
    let starved = load_baseline(&fixture("growth-fails-baseline.json"))?;
    let live = measure_current_surface();
    let outcome = enforcer_core::context_budget::evaluate(live, starved);
    assert!(
        !outcome.passes(),
        "a baseline far below the live registry's real surface must fail closed"
    );
    Ok(())
}

#[test]
fn generous_fixture_baseline_passes() -> Result<(), Box<dyn std::error::Error>> {
    let generous = load_baseline(&fixture("generous-baseline.json"))?;
    let live = measure_current_surface();
    let outcome = enforcer_core::context_budget::evaluate(live, generous);
    assert!(
        outcome.passes(),
        "a baseline far above the live registry's real surface must pass"
    );
    Ok(())
}

#[test]
fn corrupt_baseline_fails_closed_on_load() {
    let result = load_baseline(&fixture("corrupt-baseline.json"));
    assert!(
        result.is_err(),
        "a malformed baseline file must error, never silently parse as empty/default"
    );
}

#[test]
fn missing_baseline_fails_closed_on_load() {
    let result = load_baseline(&fixture("does-not-exist.json"));
    assert!(
        result.is_err(),
        "a missing baseline file must error, never silently parse as empty/default"
    );
}

#[test]
fn t2_advisory_score_is_in_unit_range_with_confidence() {
    let score = run_advisory_score();
    assert!(
        (0.0..=1.0).contains(&score.score),
        "T2 score must be in [0,1], got {}",
        score.score
    );
    assert!(
        (0.0..=1.0).contains(&score.confidence),
        "T2 confidence must be in [0,1], got {}",
        score.confidence
    );
}

#[test]
fn committed_baseline_file_is_loadable_and_well_formed() -> Result<(), Box<dyn std::error::Error>> {
    let path = committed_baseline_path();
    assert!(
        Path::new(&path).exists(),
        "the committed baseline must exist at crates/enforcer-mcp/context-budget-baseline.json"
    );
    let baseline = load_baseline(&path)?;
    assert_eq!(baseline.version, 1);
    assert!(baseline.surface.tool_count > 0);
    assert!(baseline.surface.total_bytes > 0);
    assert!(baseline.tolerance_pct >= 0.0);
    Ok(())
}

#[test]
fn live_registry_currently_passes_the_committed_baseline() -> Result<(), Box<dyn std::error::Error>>
{
    // This is the actual CI gate this workpack installs: the committed,
    // reviewed baseline must describe a surface the live registry does not
    // exceed by more than its tolerance. If this test fails, the baseline
    // needs an EXPLICIT reviewed update (never a silent rewrite) — see the
    // workpack's "Baseline is updatable only by an explicit, reviewed
    // commit" requirement.
    let baseline = load_baseline(&committed_baseline_path())?;
    let live = measure_current_surface();
    let outcome = enforcer_core::context_budget::evaluate(live, baseline);
    assert!(
        outcome.passes(),
        "live tool surface ({} tools, {} bytes) exceeds the committed baseline's tolerance \
         ({} bytes, {}% tolerance) by {:.2}% — update context-budget-baseline.json via an \
         explicit reviewed commit if this growth is intentional",
        live.tool_count,
        live.total_bytes,
        outcome.baseline.surface.total_bytes,
        outcome.baseline.tolerance_pct,
        outcome.growth_pct
    );
    Ok(())
}
