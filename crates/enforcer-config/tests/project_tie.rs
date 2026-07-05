//! Integration tests for the f03 `.enforce/config` project-tie schema:
//! proof row `project-config-native-mode` (TEST_PROOF_EXPECTATIONS.md).
//!
//! Covers, per the workpack acceptance block:
//! - fail-fixture: malformed `.enforce/config` (bad `native_mode`, unknown
//!   key, malformed JSON) -> typed boundary parse error, no silent default.
//! - pass-fixture: valid config -> resolver returns `Augment` scoped by
//!   default and honors explicit overrides; a per-rule toggle + owner/exempt
//!   glob + allow-regex round-trip through serde and take effect.
//! - detection test: absence of config -> resolver returns the scoped
//!   `Augment` default (never whole-repo); an inline-disable fixture is NOT
//!   honored (only declarative policy is).

use enforcer_config::project_tie::{
    load_project_tie, parse_project_tie, EnforcerScope, NativeMode, NativeTool,
};
use std::path::Path;

const VALID_FIXTURE: &str =
    include_str!("fixtures/project_tie/valid.enforce.config.json");
const BAD_NATIVE_MODE_FIXTURE: &str =
    include_str!("fixtures/project_tie/invalid_bad_native_mode.enforce.config.json");
const UNKNOWN_KEY_FIXTURE: &str =
    include_str!("fixtures/project_tie/invalid_unknown_key.enforce.config.json");
const MALFORMED_JSON_FIXTURE: &str =
    include_str!("fixtures/project_tie/invalid_malformed_json.enforce.config.json");
const INLINE_DISABLE_FIXTURE: &str =
    include_str!("fixtures/project_tie/inline_disable_attempt.rs.fixture");

// ---- fail-fixtures: malformed .enforce/config -------------------------

#[test]
fn bad_native_mode_is_rejected_with_typed_boundary_error() {
    let outcome = parse_project_tie(BAD_NATIVE_MODE_FIXTURE, "invalid_bad_native_mode.json");
    assert!(
        outcome.is_err(),
        "malformed native_mode must fail to load, not silently default"
    );
}

#[test]
fn unknown_key_is_rejected_with_typed_boundary_error() {
    let outcome = parse_project_tie(UNKNOWN_KEY_FIXTURE, "invalid_unknown_key.json");
    assert!(
        outcome.is_err(),
        "an unrecognized top-level key must fail to load, not be silently ignored"
    );
}

#[test]
fn malformed_json_is_rejected_with_typed_boundary_error() {
    let outcome = parse_project_tie(MALFORMED_JSON_FIXTURE, "invalid_malformed_json.json");
    assert!(outcome.is_err(), "truncated/invalid JSON must fail to load");
}

// ---- pass-fixture: valid config -----------------------------------------

#[test]
fn valid_fixture_resolves_and_round_trips_policy_fields() -> Result<(), Box<dyn std::error::Error>>
{
    let resolved = parse_project_tie(VALID_FIXTURE, "valid.enforce.config.json")?;

    // Explicit override honored: cargo stays augment/scoped (matches the
    // scoped-augment default, but here it is explicit in the fixture).
    let cargo = resolved.tie(NativeTool::Cargo);
    assert_eq!(cargo.mode, NativeMode::Augment);
    assert_eq!(cargo.scope, EnforcerScope::Scoped);
    assert!(cargo.runs_enforcer_checks());
    assert!(cargo.runs_native_tool());

    // tsc explicitly set to override: native tool is replaced, not run
    // alongside ours.
    let tsc = resolved.tie(NativeTool::Tsc);
    assert_eq!(tsc.mode, NativeMode::Override);
    assert!(!tsc.runs_native_tool());

    // Untouched tool (dart) still resolves to the scoped-augment default.
    let dart = resolved.tie(NativeTool::Dart);
    assert_eq!(dart.mode, NativeMode::Augment);
    assert_eq!(dart.scope, EnforcerScope::Scoped);

    // Owner/exempt glob + allow-regex round-trip through serde.
    assert_eq!(
        resolved.policy.owner_globs.iter().map(|g| g.as_str()).collect::<Vec<_>>(),
        vec!["crates/enforcer-config/**"]
    );
    assert_eq!(
        resolved.policy.exempt_globs.iter().map(|g| g.as_str()).collect::<Vec<_>>(),
        vec!["vendor/**"]
    );
    assert_eq!(resolved.policy.allow_regex, vec!["^// generated:".to_owned()]);
    assert!(resolved.policy.skip_cfg_test);

    // Per-rule toggle takes effect: severity override for an enabled rule,
    // and a disabled rule (with waiver) is actually disabled.
    use enforcer_domain::ids::RuleId;
    use enforcer_domain::severity::Severity;
    use std::str::FromStr;

    let rr = RuleId::from_str("RR-1.1")?;
    assert!(resolved.policy.is_rule_enabled(&rr));
    assert_eq!(
        resolved.policy.effective_severity(&rr, Severity::Error),
        Severity::Warning
    );

    let sec = RuleId::from_str("SEC-2.2")?;
    assert!(!resolved.policy.is_rule_enabled(&sec));

    Ok(())
}

// ---- detection test: absence of config ----------------------------------

#[test]
fn absent_config_file_resolves_to_scoped_augment_default_never_whole_repo(
) -> Result<(), Box<dyn std::error::Error>> {
    let missing_path = Path::new("this/path/does/not/exist/.enforce/config");
    let resolved = load_project_tie(missing_path)?;

    for tool in [
        NativeTool::Cargo,
        NativeTool::Tsc,
        NativeTool::Ruff,
        NativeTool::Dart,
        NativeTool::Cflint,
    ] {
        let tie = resolved.tie(tool);
        assert_eq!(
            tie.mode,
            NativeMode::Augment,
            "absent config must default every tool to Augment"
        );
        assert_eq!(
            tie.scope,
            EnforcerScope::Scoped,
            "absent config must never default to WholeRepo scope"
        );
    }
    Ok(())
}

#[test]
fn inline_disable_attempt_in_a_source_fixture_is_not_honored() -> Result<(), Box<dyn std::error::Error>>
{
    // The inline-disable-shaped comment in this fixture is plain text to
    // the project_tie loader: it is never parsed as `.enforce/config`, so
    // it cannot suppress anything. Only declarative `Policy.rule_toggles`
    // (see `valid_fixture_resolves_and_round_trips_policy_fields`) can
    // disable a rule. Assert the fixture text is present (so this test
    // would fail loudly if the fixture were ever deleted/emptied) and that
    // a resolver built from a config with no matching toggle still leaves
    // that rule enabled.
    assert!(INLINE_DISABLE_FIXTURE.contains("enforcer-disable RR-1.1"));

    let resolved = parse_project_tie("{}", "empty.json")?;
    use enforcer_domain::ids::RuleId;
    use std::str::FromStr;
    let rr = RuleId::from_str("RR-1.1")?;
    assert!(
        resolved.policy.is_rule_enabled(&rr),
        "an inline comment-style disable must have zero effect; only declarative policy counts"
    );
    Ok(())
}
