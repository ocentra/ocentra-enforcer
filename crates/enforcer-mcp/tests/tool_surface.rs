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

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use enforcer_core::context_budget::decision;
use enforcer_domain::core_types::{BudgetGateDecision, BUDGET_BASELINE_VERSION};
use enforcer_mcp::boundary::tool_descriptor::ToolDescriptorDto;
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

fn smoke_binary_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(path) = std::env::var_os("CARGO_BIN_EXE_enforcer-mcp-smoke") {
        return Ok(PathBuf::from(path));
    }
    let test_binary = std::env::current_exe()?;
    let debug_dir = test_binary
        .parent()
        .and_then(Path::parent)
        .ok_or("integration test binary is not under target/debug/deps")?;
    Ok(debug_dir.join(format!(
        "enforcer-mcp-smoke{}",
        std::env::consts::EXE_SUFFIX
    )))
}

fn live_tool_descriptors() -> Result<Vec<ToolDescriptorDto>, Box<dyn std::error::Error>> {
    let mut child = Command::new(smoke_binary_path()?)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("smoke binary has no stdin")?;
    writeln!(
        stdin,
        "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}}"
    )?;
    drop(stdin);
    let output = child.wait_with_output()?;
    assert!(
        output.status.success(),
        "tools/list smoke binary failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let reply: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    Ok(serde_json::from_value(reply["result"]["tools"].clone())?)
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
        usize::from(first.tool_count()) > 0,
        "the registry must register at least one tool"
    );
    assert!(usize::from(first.total_bytes()) > 0);
}

#[test]
fn simulated_surface_growth_fixture_fails_the_t1_ratchet() -> Result<(), Box<dyn std::error::Error>>
{
    let starved = load_baseline(&fixture("growth-fails-baseline.json"))?;
    let live = measure_current_surface();
    let outcome = enforcer_core::context_budget::evaluate(live, starved);
    assert!(
        decision(&outcome) == BudgetGateDecision::Fail,
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
        decision(&outcome) == BudgetGateDecision::Pass,
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
        (0.0..=1.0).contains(&f64::from(score.score())),
        "T2 score must be in [0,1], got {}",
        f64::from(score.score())
    );
    assert!(
        (0.0..=1.0).contains(&f64::from(score.confidence())),
        "T2 confidence must be in [0,1], got {}",
        f64::from(score.confidence())
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
    assert_eq!(baseline.version(), BUDGET_BASELINE_VERSION);
    assert!(usize::from(baseline.surface().tool_count()) > 0);
    assert!(usize::from(baseline.surface().total_bytes()) > 0);
    assert!(f64::from(baseline.tolerance_pct()) >= 0.0);
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
    // Pin the reviewed post-expansion shape; the 10% ratchet below remains
    // unchanged and rejects any unreviewed future growth.
    assert_eq!(usize::from(live.tool_count()), 102);
    assert_eq!(usize::from(live.total_bytes()), 145_299);
    let outcome = enforcer_core::context_budget::evaluate(live, baseline);
    assert!(
        decision(&outcome) == BudgetGateDecision::Pass,
        "live tool surface ({} tools, {} bytes) exceeds the committed baseline's tolerance \
         ({} bytes, {}% tolerance) by {:.2}% — update context-budget-baseline.json via an \
         explicit reviewed commit if this growth is intentional",
        usize::from(live.tool_count()),
        usize::from(live.total_bytes()),
        usize::from(outcome.baseline().surface().total_bytes()),
        f64::from(outcome.baseline().tolerance_pct()),
        f64::from(outcome.growth_pct())
    );
    Ok(())
}

#[test]
fn reviewed_route_projection_delta_accounts_for_the_exact_surface_growth(
) -> Result<(), Box<dyn std::error::Error>> {
    const PREVIOUS_SURFACE_BYTES: usize = 145_179;
    let live = live_tool_descriptors()?;
    let mut old_like = live.clone();
    let mut route_names = Vec::new();
    let mut route_deltas = Vec::new();

    for (current, old) in live.iter().zip(old_like.iter_mut()) {
        if !matches!(
            current.name.as_str(),
            "ocentra_enforcer_route" | "rust_rules_route"
        ) {
            continue;
        }
        route_names.push(current.name.clone());
        assert!(
            current.input_schema["properties"]
                .as_object()
                .is_some_and(|properties| !properties.contains_key("consumerCapabilities")),
            "consumer capability values are output-only and must not expand the route input schema"
        );
        let current_bytes = serde_json::to_vec(current)?.len();
        old.input_schema["properties"]
            .as_object_mut()
            .ok_or("route schema properties must be an object")?
            .remove("identityProjection");
        let old_bytes = serde_json::to_vec(old)?.len();
        route_deltas.push(current_bytes - old_bytes);
    }

    assert_eq!(
        route_names,
        vec!["ocentra_enforcer_route", "rust_rules_route"]
    );
    assert_eq!(route_deltas, vec![60, 60]);
    let live_bytes = serde_json::to_vec(&live)?.len();
    let old_like_bytes = serde_json::to_vec(&old_like)?.len();
    assert_eq!(old_like_bytes, PREVIOUS_SURFACE_BYTES);
    assert_eq!(live_bytes - old_like_bytes, 120);
    assert_eq!(
        live_bytes - PREVIOUS_SURFACE_BYTES,
        route_deltas.iter().sum::<usize>()
    );
    assert_eq!(
        usize::from(measure_current_surface().total_bytes()),
        live_bytes
    );
    Ok(())
}

#[test]
fn reviewed_check_enum_growth_accounts_for_the_140_byte_baseline_delta(
) -> Result<(), Box<dyn std::error::Error>> {
    let live = live_tool_descriptors()?;
    let added_values = [
        "mutation-risk",
        "docs-completeness",
        "config-lockdown",
        "waiver-policy",
    ];
    let check_names = ["ocentra_enforcer_check", "rust_rules_check"];
    let mut observed_added_occurrences = 0_usize;
    let mut old_like = live.clone();
    let mut descriptor_deltas = Vec::new();

    for (current, old) in live.iter().zip(old_like.iter_mut()) {
        if !check_names.contains(&current.name.as_str()) {
            continue;
        }
        let current_bytes = serde_json::to_vec(current)?.len();
        let values = old.input_schema["properties"]["check"]["enum"]
            .as_array_mut()
            .ok_or("check enum must be an array")?;
        observed_added_occurrences += values
            .iter()
            .filter(|value| {
                value
                    .as_str()
                    .is_some_and(|entry| added_values.contains(&entry))
            })
            .count();
        values.retain(|value| {
            value
                .as_str()
                .is_none_or(|entry| !added_values.contains(&entry))
        });
        let old_bytes = serde_json::to_vec(old)?.len();
        descriptor_deltas.push(current_bytes - old_bytes);
    }

    let live_bytes = serde_json::to_vec(&live)?.len();
    let old_like_bytes = serde_json::to_vec(&old_like)?.len();
    assert_eq!(observed_added_occurrences, 8);
    assert_eq!(descriptor_deltas, vec![70, 70]);
    assert_eq!(live_bytes, 145_299);
    assert_eq!(old_like_bytes, 145_159);
    assert_eq!(live_bytes - old_like_bytes, 140);
    assert_eq!(
        usize::from(measure_current_surface().total_bytes()),
        live_bytes
    );
    Ok(())
}
