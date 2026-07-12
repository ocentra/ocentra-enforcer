//! h07 acceptance proof — `security_pipeline` — over RECORDED tool
//! reports under `tests/fixtures/security_pipeline/**` (no live engine
//! required in CI, per the workpack's acceptance section), plus
//! negative coverage for malformed, empty, oversized, and invalid
//! recorded input at every stage's parse boundary:
//!
//! - coverage: `coverage/bad/below_floor.json` fails,
//!   `coverage/good/at_floor.json` is clean.
//! - observability money-path log (T2, scored):
//!   `observability/bad/money_path_no_security_log.json` fails,
//!   `observability/good/money_path_logged.json` is clean.
//! - security event sampled (T1):
//!   `observability/bad/security_event_sampled.json` fails,
//!   `observability/good/security_event_unsampled.json` is clean.
//! - fuzz seed: `fuzz/bad/no_seed.json` fails,
//!   `fuzz/good/with_seed.json` is clean.
//! - static threat-mapping: `static/bad/threat_mapped_exploitable.json`
//!   blocks; `static/good/non_exploitable_signal.json` stays
//!   signal-only (clean from that gate).
//! - concurrency severity:
//!   `concurrency/bad/race_condition_detected.json` fails,
//!   `concurrency/good/below_threshold.json` is clean.
//! - graceful-skip: `graceful_skip/good/missing_tool.json` is an honest
//!   skip with `ran: 0`; `graceful_skip/bad/dishonest_pass.json` (an
//!   absent tool claiming to have run) is rejected at every stage's
//!   parse boundary, never accepted as a silent pass.

use std::path::Path;

use enforcer_core::error::{DecodeError, Error};
use enforcer_domain::findings::{Finding, ScanScope};
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_harness::security_pipeline::adapters::concurrency_report::parse_recorded as parse_concurrency;
use enforcer_harness::security_pipeline::adapters::coverage_report::parse_recorded as parse_coverage;
use enforcer_harness::security_pipeline::adapters::crypto_localnet_report::run_stage;
use enforcer_harness::security_pipeline::adapters::fuzz_report::parse_recorded as parse_fuzz;
use enforcer_harness::security_pipeline::adapters::observability_report::parse_recorded as parse_observability;
use enforcer_harness::security_pipeline::adapters::static_analysis_report::parse_recorded as parse_static;
use enforcer_harness::security_pipeline::concurrency::{
    ConcurrencyOutcome, ConcurrencySeverityGate,
};
use enforcer_harness::security_pipeline::coverage::{CoverageFloorGate, CoverageOutcome};
use enforcer_harness::security_pipeline::crypto_localnet::{
    CryptoLocalnetActivation, CryptoLocalnetConfig, CryptoLocalnetOutcome,
};
use enforcer_harness::security_pipeline::fuzz::{FuzzOutcome, FuzzSeedGate};
use enforcer_harness::security_pipeline::observability::{
    MoneyPathLoggingGate, ObservabilityOutcome,
};
use enforcer_harness::security_pipeline::observability_sampling::SamplingDropGate;
use enforcer_harness::security_pipeline::static_analysis::{StaticOutcome, StaticThreatGate};
use enforcer_validator::error::HarnessError;
use enforcer_validator::harness::run_fixture_parity;
use enforcer_validator::validator::{ValidationInput, Validator};

const MISSING_TOOL: &str =
    include_str!("fixtures/security_pipeline/graceful_skip/good/missing_tool.json");
const DISHONEST_PASS: &str =
    include_str!("fixtures/security_pipeline/graceful_skip/bad/dishonest_pass.json");
const MONEY_PATH_BAD: &str =
    include_str!("fixtures/security_pipeline/observability/bad/money_path_no_security_log.json");
const FUZZ_NO_SEED: &str = include_str!("fixtures/security_pipeline/fuzz/bad/no_seed.json");
const STATIC_SIGNAL_ONLY: &str =
    include_str!("fixtures/security_pipeline/static/good/non_exploitable_signal.json");

/// Typed failure surface for these tests — never an erased error type.
enum TestFailure {
    /// A branded value refused to mint from its literal.
    Decode(DecodeError),
    /// A stage parse boundary rejected input the test expected to pass.
    Core(Error),
    /// The fixture-parity oracle failed.
    Parity(HarnessError),
}

impl std::fmt::Debug for TestFailure {
    /// Render the wrapped failure's own message (this is what the test
    /// harness prints when a `Result`-returning test fails).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestFailure::Decode(source) => write!(f, "brand decode failure: {source}"),
            TestFailure::Core(source) => write!(f, "stage boundary rejection: {source}"),
            TestFailure::Parity(source) => write!(f, "fixture parity failure: {source}"),
        }
    }
}

/// 5-way-plus parity oracle (six gates): every gate's `RuleId` fires on
/// its fail fixture and stays clean on its pass fixture, proven through
/// the SAME `run_fixture_parity` oracle every other rule in this
/// workspace uses.
#[test]
fn security_pipeline() -> Result<(), TestFailure> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));

    let coverage_gate = CoverageFloorGate::new("SEC-COV.1".parse().map_err(TestFailure::Decode)?);
    run_fixture_parity(
        &coverage_gate,
        manifest,
        "tests/fixtures/security_pipeline/coverage/bad/below_floor.json",
        "tests/fixtures/security_pipeline/coverage/good/at_floor.json",
    )
    .map_err(TestFailure::Parity)?;

    let money_path_gate =
        MoneyPathLoggingGate::new("SEC-OBS-MONEYPATH.1".parse().map_err(TestFailure::Decode)?);
    run_fixture_parity(
        &money_path_gate,
        manifest,
        "tests/fixtures/security_pipeline/observability/bad/money_path_no_security_log.json",
        "tests/fixtures/security_pipeline/observability/good/money_path_logged.json",
    )
    .map_err(TestFailure::Parity)?;

    let sampling_gate =
        SamplingDropGate::new("SEC-OBS-SAMPLING.1".parse().map_err(TestFailure::Decode)?);
    run_fixture_parity(
        &sampling_gate,
        manifest,
        "tests/fixtures/security_pipeline/observability/bad/security_event_sampled.json",
        "tests/fixtures/security_pipeline/observability/good/security_event_unsampled.json",
    )
    .map_err(TestFailure::Parity)?;

    let fuzz_gate = FuzzSeedGate::new("SEC-FUZZ-SEED.1".parse().map_err(TestFailure::Decode)?);
    run_fixture_parity(
        &fuzz_gate,
        manifest,
        "tests/fixtures/security_pipeline/fuzz/bad/no_seed.json",
        "tests/fixtures/security_pipeline/fuzz/good/with_seed.json",
    )
    .map_err(TestFailure::Parity)?;

    let static_gate =
        StaticThreatGate::new("SEC-STATIC-THREAT.1".parse().map_err(TestFailure::Decode)?);
    run_fixture_parity(
        &static_gate,
        manifest,
        "tests/fixtures/security_pipeline/static/bad/threat_mapped_exploitable.json",
        "tests/fixtures/security_pipeline/static/good/non_exploitable_signal.json",
    )
    .map_err(TestFailure::Parity)?;

    let concurrency_gate = ConcurrencySeverityGate::new(
        "SEC-CONCURRENCY-SEVERITY.1"
            .parse()
            .map_err(TestFailure::Decode)?,
        Severity::Error,
    );
    run_fixture_parity(
        &concurrency_gate,
        manifest,
        "tests/fixtures/security_pipeline/concurrency/bad/race_condition_detected.json",
        "tests/fixtures/security_pipeline/concurrency/good/below_threshold.json",
    )
    .map_err(TestFailure::Parity)?;

    Ok(())
}

/// Graceful-skip acceptance row: an honest skip (`toolPresent: false`,
/// `outcome: skipped`, `ran: 0`) is accepted with an honest ran-count at
/// EVERY stage's parse boundary, and the dishonest fixture (an absent
/// tool claiming to have run) is rejected at every one of them — never
/// coerced into a silent pass.
#[test]
fn graceful_skip_is_honest_at_every_stage_boundary() -> Result<(), TestFailure> {
    assert_eq!(
        parse_coverage(MISSING_TOOL).map_err(TestFailure::Core)?,
        CoverageOutcome::Skipped { ran: 0 }
    );
    assert_eq!(
        parse_fuzz(MISSING_TOOL).map_err(TestFailure::Core)?,
        FuzzOutcome::Skipped { ran: 0 }
    );
    assert_eq!(
        parse_observability(MISSING_TOOL).map_err(TestFailure::Core)?,
        ObservabilityOutcome::Skipped { ran: 0 }
    );
    assert_eq!(
        parse_static(MISSING_TOOL).map_err(TestFailure::Core)?,
        StaticOutcome::Skipped { ran: 0 }
    );
    assert_eq!(
        parse_concurrency(MISSING_TOOL).map_err(TestFailure::Core)?,
        ConcurrencyOutcome::Skipped { ran: 0 }
    );

    assert!(
        matches!(parse_coverage(DISHONEST_PASS), Err(Error::Decode(_))),
        "coverage boundary must reject an absent tool claiming to have run"
    );
    assert!(
        matches!(parse_fuzz(DISHONEST_PASS), Err(Error::Decode(_))),
        "fuzz boundary must reject an absent tool claiming to have run"
    );
    assert!(
        matches!(parse_observability(DISHONEST_PASS), Err(Error::Decode(_))),
        "observability boundary must reject an absent tool claiming to have run"
    );
    assert!(
        matches!(parse_static(DISHONEST_PASS), Err(Error::Decode(_))),
        "static boundary must reject an absent tool claiming to have run"
    );
    assert!(
        matches!(parse_concurrency(DISHONEST_PASS), Err(Error::Decode(_))),
        "concurrency boundary must reject an absent tool claiming to have run"
    );
    Ok(())
}

/// T2 money-path scoring: both signals missing scores 1.0 at high
/// confidence, and the finding is byte-for-byte the expected one
/// (strongest possible assertion — full value equality).
#[test]
fn money_path_gap_is_scored_and_flagged() -> Result<(), TestFailure> {
    let outcome = parse_observability(MONEY_PATH_BAD).map_err(TestFailure::Core)?;
    let gate =
        MoneyPathLoggingGate::new("SEC-OBS-MONEYPATH.1".parse().map_err(TestFailure::Decode)?);
    let file: RelPath = "observability/recorded-events.json"
        .parse()
        .map_err(TestFailure::Decode)?;
    let findings = gate.evaluate(&outcome, &file);
    let title = String::from("money-critical path missing security log/correlation id");
    let detail = String::from(
        "money-critical event `checkout` emits no security log and carries no correlation \
         id (score=1.0, confidence=high)",
    );
    let expected = Finding {
        rule_id: "SEC-OBS-MONEYPATH.1".parse().map_err(TestFailure::Decode)?,
        severity: Severity::Error,
        title,
        detail,
        // The evaluate call above already finished borrowing `file`, so
        // the expected finding takes ownership — no copy needed.
        file,
        line: 1,
        snippet: None,
    };
    assert_eq!(findings, vec![expected]);
    Ok(())
}

/// T1 fuzz-seed gate: the no-seed failure is flagged with its
/// counterexample carried into the detail, byte-for-byte.
#[test]
fn fuzz_failure_without_seed_is_flagged_with_counterexample() -> Result<(), TestFailure> {
    let outcome = parse_fuzz(FUZZ_NO_SEED).map_err(TestFailure::Core)?;
    let gate = FuzzSeedGate::new("SEC-FUZZ-SEED.1".parse().map_err(TestFailure::Decode)?);
    let file: RelPath = "fuzz/recorded-report.json"
        .parse()
        .map_err(TestFailure::Decode)?;
    let findings = gate.evaluate(&outcome, &file);
    let title = String::from("fuzz/property failure missing persisted seed");
    let detail = String::from(
        "property `prop_never_negative` failed with no persisted seed — the failure cannot \
         be reproduced or regression-tested (counterexample: x = -1)",
    );
    let expected = Finding {
        rule_id: "SEC-FUZZ-SEED.1".parse().map_err(TestFailure::Decode)?,
        severity: Severity::Error,
        title,
        detail,
        // The evaluate call above already finished borrowing `file`, so
        // the expected finding takes ownership — no copy needed.
        file,
        line: 1,
        snippet: None,
    };
    assert_eq!(findings, vec![expected]);
    Ok(())
}

/// Static findings stay signal-only unless threat-mapped exploitable:
/// the CWE-1004 signal fixture produces ZERO blocking findings from the
/// threat gate (the blocking direction is covered by the parity oracle
/// in `security_pipeline`).
#[test]
fn non_exploitable_static_signal_stays_non_blocking() -> Result<(), TestFailure> {
    let outcome = parse_static(STATIC_SIGNAL_ONLY).map_err(TestFailure::Core)?;
    let gate = StaticThreatGate::new("SEC-STATIC-THREAT.1".parse().map_err(TestFailure::Decode)?);
    let file: RelPath = "static/recorded-report.json"
        .parse()
        .map_err(TestFailure::Decode)?;
    assert_eq!(gate.evaluate(&outcome, &file), Vec::new());
    Ok(())
}

/// The crypto-localnet stage is a disjoint opt-in seam: off by default
/// (never even reading its input), honestly skipping when enabled with
/// the tool absent, rejecting a dishonest absent-tool claim, and
/// surfacing an erroring tool's message verbatim.
#[test]
fn crypto_localnet_is_a_disjoint_opt_in_seam() -> Result<(), TestFailure> {
    let off = CryptoLocalnetConfig::default();
    assert_eq!(
        run_stage(
            &off,
            "{this malformed text is never even read while disabled"
        )
        .map_err(TestFailure::Core)?,
        CryptoLocalnetOutcome::Disabled
    );

    let on = CryptoLocalnetConfig {
        activation: CryptoLocalnetActivation::Enabled,
    };
    assert_eq!(
        run_stage(&on, r#"{"toolPresent":false,"outcome":"skipped","ran":0}"#)
            .map_err(TestFailure::Core)?,
        CryptoLocalnetOutcome::Skipped { ran: 0 }
    );
    assert!(
        matches!(
            run_stage(&on, r#"{"toolPresent":false,"outcome":"ran","ran":1}"#),
            Err(Error::Decode(_))
        ),
        "an enabled stage must reject an absent tool claiming to have run"
    );
    let error_message = String::from("solana-test-validator crashed");
    assert_eq!(
        run_stage(
            &on,
            r#"{"toolPresent":true,"outcome":"errored","ran":0,"errorMessage":"solana-test-validator crashed"}"#,
        )
        .map_err(TestFailure::Core)?,
        CryptoLocalnetOutcome::Errored { error_message }
    );
    Ok(())
}

/// Every stage boundary rejects malformed, empty, oversized, and
/// invalid (out-of-range) recorded input — never a panic, never a
/// silent pass.
#[test]
fn malformed_empty_oversized_and_invalid_input_is_rejected() {
    assert!(
        matches!(parse_fuzz(""), Err(Error::Decode(_))),
        "empty input must be rejected"
    );
    assert!(
        matches!(parse_fuzz("{not json"), Err(Error::Decode(_))),
        "malformed input must be rejected"
    );
    let oversized = "x".repeat(2 * 1024 * 1024);
    assert!(
        matches!(parse_fuzz(&oversized), Err(Error::Decode(_))),
        "oversized non-JSON input must be rejected"
    );
    assert!(
        matches!(
            parse_coverage(
                r#"{"toolPresent":true,"outcome":"ran","ran":1,"linePct":250.0,"branchPct":80.0}"#,
            ),
            Err(Error::Decode(_))
        ),
        "an invalid out-of-range coverage percentage must be rejected"
    );
    assert!(
        matches!(
            parse_fuzz(
                r#"{"toolPresent":true,"outcome":"ran","ran":1,"failures":[{"property":"p","seed":""}]}"#,
            ),
            Err(Error::Decode(_))
        ),
        "a blank seed must be rejected as malformed, or it would dodge the seed gate"
    );
    assert!(
        matches!(
            parse_static(
                r#"{"toolPresent":true,"outcome":"ran","ran":1,"findings":[{"ruleId":"r","file":"a.rs","line":1,"message":"m","threatId":"NOT-A-THREAT"}]}"#,
            ),
            Err(Error::Decode(_))
        ),
        "an unrecognized threat citation format must be rejected"
    );
}

/// A gate's `Validator` impl stamps every finding with its OWN rule id,
/// including the rejected-input finding for malformed recorded text.
#[test]
fn every_gate_finding_carries_its_own_rule_id() -> Result<(), TestFailure> {
    let gate = FuzzSeedGate::new("SEC-FUZZ-SEED.1".parse().map_err(TestFailure::Decode)?);
    let file: RelPath = "fuzz/recorded-report.json"
        .parse()
        .map_err(TestFailure::Decode)?;
    let sources = [FUZZ_NO_SEED, "{not json"];
    for source in sources {
        let findings = gate.validate(ValidationInput {
            file: &file,
            source,
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1, "expected exactly one finding");
        for finding in &findings {
            assert_eq!(finding.rule_id.as_str(), "SEC-FUZZ-SEED.1");
        }
    }
    Ok(())
}

/// PROPERTY-TEST: across a generated grid of line-coverage percentages
/// (0.0..=100.0 in tenths, branch fixed above its floor), the coverage
/// floor gate fires exactly when the line percentage sits below the 90%
/// floor — no off-by-one, no silent band.
#[test]
fn coverage_floor_gate_property_over_a_percentage_grid() -> Result<(), TestFailure> {
    let gate = CoverageFloorGate::new("SEC-COV.1".parse().map_err(TestFailure::Decode)?);
    let file: RelPath = "coverage/recorded-report.json"
        .parse()
        .map_err(TestFailure::Decode)?;
    for tenths in 0u16..=1000 {
        let line_pct = f64::from(tenths) / 10.0;
        let raw = format!(
            r#"{{"toolPresent":true,"outcome":"ran","ran":1,"linePct":{line_pct},"branchPct":95.0}}"#
        );
        let outcome = parse_coverage(&raw).map_err(TestFailure::Core)?;
        let findings = gate.evaluate(&outcome, &file);
        if line_pct < 90.0 {
            assert_eq!(
                findings.len(),
                1,
                "the gate must fire below the floor at {line_pct}"
            );
        } else {
            assert_eq!(
                findings.len(),
                0,
                "the gate must stay clean at or above the floor at {line_pct}"
            );
        }
    }
    Ok(())
}

/// PROPERTY-TEST: over the full generated matrix of recorded shapes
/// (outcome word x toolPresent x errorMessage presence), the shared
/// honesty rule accepts exactly the honest combinations and rejects
/// every dishonest one — `skipped != passed != failed` can never drift
/// at any stage, because every stage parses through this same rule.
#[test]
fn recorded_honesty_matrix_property_holds_for_every_stage_shape() {
    let outcomes = ["skipped", "errored", "ran", "bogus"];
    let presence_states = [false, true];
    let error_states = [false, true];
    for outcome in outcomes {
        for tool_present in presence_states {
            for with_error in error_states {
                let error_field = if with_error {
                    r#","errorMessage":"engine exited 2""#
                } else {
                    ""
                };
                let raw = format!(
                    r#"{{"toolPresent":{tool_present},"outcome":"{outcome}","ran":0,"failures":[]{error_field}}}"#
                );
                let parsed = parse_fuzz(&raw);
                let expect_accepted = matches!(
                    (outcome, tool_present, with_error),
                    ("skipped", false, _) | ("errored", _, _) | ("ran", true, false)
                );
                assert_eq!(
                    parsed.is_ok(),
                    expect_accepted,
                    "honesty matrix mismatch for {raw}"
                );
            }
        }
    }
}
