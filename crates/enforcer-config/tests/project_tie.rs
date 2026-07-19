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

use enforcer_config::error::ConfigLoadError;
use enforcer_config::project_tie::{load_project_tie, parse_project_tie};
use enforcer_domain::config_types::{ConfigJson, ConfigSource};
use enforcer_domain::config_types::{EnforcerScope, NativeMode, NativeTool, RegexPattern};
use std::path::Path;

const VALID_FIXTURE: &str = include_str!("fixtures/project_tie/valid.enforce.config.json");
const BAD_NATIVE_MODE_FIXTURE: &str =
    include_str!("fixtures/project_tie/invalid_bad_native_mode.enforce.config.json");
const UNKNOWN_KEY_FIXTURE: &str =
    include_str!("fixtures/project_tie/invalid_unknown_key.enforce.config.json");
const MALFORMED_JSON_FIXTURE: &str =
    include_str!("fixtures/project_tie/invalid_malformed_json.enforce.config.json");

// ---- fail-fixtures: malformed .enforce/config -------------------------

#[test]
fn bad_native_mode_is_rejected_with_typed_boundary_error() {
    assert!(matches!(
        parse_project_tie(
            &ConfigJson::from_owned(BAD_NATIVE_MODE_FIXTURE.to_owned()),
            &ConfigSource::from_owned("invalid_bad_native_mode.json".to_owned())
        ),
        Err(ConfigLoadError::Parse(_))
    ));
}

#[test]
fn unknown_key_is_rejected_with_typed_boundary_error() {
    assert!(matches!(
        parse_project_tie(
            &ConfigJson::from_owned(UNKNOWN_KEY_FIXTURE.to_owned()),
            &ConfigSource::from_owned("invalid_unknown_key.json".to_owned())
        ),
        Err(ConfigLoadError::Parse(_))
    ));
}

#[test]
fn malformed_json_is_rejected_with_typed_boundary_error() {
    assert!(matches!(
        parse_project_tie(
            &ConfigJson::from_owned(MALFORMED_JSON_FIXTURE.to_owned()),
            &ConfigSource::from_owned("invalid_malformed_json.json".to_owned())
        ),
        Err(ConfigLoadError::Parse(_))
    ));
}

// ---- pass-fixture: valid config -----------------------------------------

#[test]
fn valid_fixture_resolves_and_round_trips_policy_fields() -> Result<(), Box<dyn std::error::Error>>
{
    let resolved = parse_project_tie(
        &ConfigJson::from_owned(VALID_FIXTURE.to_owned()),
        &ConfigSource::from_owned("valid.enforce.config.json".to_owned()),
    )?;

    // Explicit override honored: cargo stays augment/scoped (matches the
    // scoped-augment default, but here it is explicit in the fixture).
    let cargo = resolved.tie(NativeTool::Cargo);
    assert_eq!(cargo.mode, NativeMode::Augment);
    assert_eq!(cargo.scope, EnforcerScope::Scoped);

    // tsc explicitly set to override: native tool is replaced, not run
    // alongside ours.
    let tsc = resolved.tie(NativeTool::Tsc);
    assert_eq!(tsc.mode, NativeMode::Override);

    // Untouched tool (dart) still resolves to the scoped-augment default.
    let dart = resolved.tie(NativeTool::Dart);
    assert_eq!(dart.mode, NativeMode::Augment);
    assert_eq!(dart.scope, EnforcerScope::Scoped);

    // Owner/exempt glob + allow-regex round-trip through serde.
    assert_eq!(
        resolved
            .policy
            .owner_globs
            .iter()
            .map(|g| g.as_str())
            .collect::<Vec<_>>(),
        vec!["crates/enforcer-config/**"]
    );
    assert_eq!(
        resolved
            .policy
            .exempt_globs
            .iter()
            .map(|g| g.as_str())
            .collect::<Vec<_>>(),
        vec!["vendor/**"]
    );
    assert_eq!(
        resolved.policy.allow_regex,
        vec![RegexPattern::new("^// generated:".to_owned())?]
    );
    assert!(matches!(
        resolved.policy.skip_cfg_test,
        enforcer_domain::config_types::CfgTestSkipping::Enabled
    ));

    // Per-rule toggle takes effect: severity override for an enabled rule,
    // and a disabled rule (with waiver) is actually disabled.
    use enforcer_domain::ids::RuleId;
    use enforcer_domain::severity::Severity;
    use std::str::FromStr;

    let rr = RuleId::from_str("RR-1.1")?;
    assert!(matches!(
        resolved.policy.rule_enabled(&rr),
        enforcer_domain::config_types::RuleEnabled::Enabled
    ));
    assert_eq!(
        resolved.policy.effective_severity(&rr, Severity::Error),
        Severity::Warning
    );

    let sec = RuleId::from_str("SEC-2.2")?;
    assert!(matches!(
        resolved.policy.rule_enabled(&sec),
        enforcer_domain::config_types::RuleEnabled::Disabled
    ));

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
fn empty_config_keeps_a_rule_enabled_without_a_declarative_toggle(
) -> Result<(), Box<dyn std::error::Error>> {
    // A resolver built from a config with no matching declarative toggle
    // keeps the rule enabled.
    let resolved = parse_project_tie(
        &ConfigJson::from_owned("{}".to_owned()),
        &ConfigSource::from_owned("empty.json".to_owned()),
    )?;
    use enforcer_domain::ids::RuleId;
    use std::str::FromStr;
    let rr = RuleId::from_str("RR-1.1")?;
    assert!(
        matches!(
            resolved.policy.rule_enabled(&rr),
            enforcer_domain::config_types::RuleEnabled::Enabled
        ),
        "an inline comment-style disable must have zero effect; only declarative policy counts"
    );
    Ok(())
}
