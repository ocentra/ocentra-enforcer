//! a08 acceptance proof: path-scoped, branded-rule waivers load only through
//! a strict boundary and never become a project-wide rule toggle.

use enforcer_rules::loader::{load_registry_from_records, parse_catalog};
use enforcer_rules::waiver::{ExpiryPolicy, WaiverDate, WaiverRegistry};

const RULES: &str = r#"[
  {
    "ruleId": "SRC-1.1",
    "version": 1,
    "title": "Source shape",
    "tier": "T1",
    "validator": { "crateName": "enforcer-lang-common", "path": "source_shape::Validator" },
    "fixtures": { "fail": "fixtures/fail", "pass": "fixtures/pass" },
    "docAnchor": "docs/rules/SRC-1.md#SRC-1.1"
  }
]"#;

fn rules() -> Result<enforcer_rules::registry::RuleRegistry, Box<dyn std::error::Error>> {
    load_registry_from_records(parse_catalog(RULES, "<inline rules>")?).map_err(Into::into)
}

fn today() -> Result<WaiverDate, enforcer_rules::waiver::WaiverLoadError> {
    WaiverDate::new(2026, 7, 10)
}

#[test]
fn packaged_empty_registry_loads_at_the_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let registry = WaiverRegistry::parse(
        include_str!("../waivers.json"),
        "waivers.json",
        &rules()?,
        today()?,
        ExpiryPolicy::RejectExpired,
    )?;
    assert!(registry.waivers.is_empty());
    Ok(())
}

#[test]
fn known_rule_and_exact_path_match_only_that_finding() -> Result<(), Box<dyn std::error::Error>> {
    let registry = WaiverRegistry::parse(
        r#"{
          "waivers": [{
            "path": "crates/enforcer-cli/src/legacy.rs",
            "ruleId": "SRC-1.1",
            "owner": "platform-team",
            "reason": "tracked migration budget",
            "expires": "2026-07-10"
          }]
        }"#,
        "<inline>",
        &rules()?,
        today()?,
        ExpiryPolicy::RejectExpired,
    )?;
    let rule_id = "SRC-1.1".parse()?;
    assert!(registry
        .matching("crates\\enforcer-cli\\src\\legacy.rs", &rule_id, today()?)
        .is_some());
    assert!(registry
        .matching("crates/enforcer-cli/src/other.rs", &rule_id, today()?)
        .is_none());
    assert!(registry
        .matching(
            "crates/enforcer-cli/src/legacy.rs",
            &"SRC-1.2".parse()?,
            today()?
        )
        .is_none());
    Ok(())
}

#[test]
fn malformed_rule_empty_reason_and_numeric_override_fail_closed(
) -> Result<(), Box<dyn std::error::Error>> {
    for invalid in [
        r#"{"waivers":[{"path":"src/file.rs","ruleId":"bad","owner":"team","reason":"reason"}]}"#,
        r#"{"waivers":[{"path":"src/file.rs","ruleId":"SRC-1.1","owner":" ","reason":"reason"}]}"#,
        r#"{"waivers":[{"path":"src/file.rs","ruleId":"SRC-1.1","owner":"team","reason":"  "}]}"#,
        r#"{"waivers":[{"path":"src/file.rs","ruleId":"SRC-1.1","owner":"team","reason":"reason","expires":"2026-02-29"}]}"#,
        r#"{"waivers":[{"path":"src/file.rs","ruleId":"SRC-1.1","owner":"team","reason":"reason","maxBranches":122}]}"#,
    ] {
        assert!(WaiverRegistry::parse(
            invalid,
            "<inline>",
            &rules()?,
            today()?,
            ExpiryPolicy::RejectExpired,
        )
        .is_err());
    }
    Ok(())
}

#[test]
fn broad_or_escaping_paths_and_unknown_rules_fail_closed() -> Result<(), Box<dyn std::error::Error>>
{
    for invalid in [
        r#"{"waivers":[{"path":"src/**","ruleId":"SRC-1.1","owner":"team","reason":"reason"}]}"#,
        r#"{"waivers":[{"path":"../src/file.rs","ruleId":"SRC-1.1","owner":"team","reason":"reason"}]}"#,
        r#"{"waivers":[{"path":"src/file.rs","ruleId":"SRC-9.9","owner":"team","reason":"reason"}]}"#,
    ] {
        assert!(WaiverRegistry::parse(
            invalid,
            "<inline>",
            &rules()?,
            today()?,
            ExpiryPolicy::RejectExpired,
        )
        .is_err());
    }
    Ok(())
}

#[test]
fn expiry_can_reject_at_load_or_remain_auditable_without_matching(
) -> Result<(), Box<dyn std::error::Error>> {
    let expired = r#"{
      "waivers": [{
        "path": "src/file.rs",
        "ruleId": "SRC-1.1",
        "owner": "team",
        "reason": "expired migration exception",
        "expires": "2026-07-09"
      }]
    }"#;
    assert!(WaiverRegistry::parse(
        expired,
        "<inline>",
        &rules()?,
        today()?,
        ExpiryPolicy::RejectExpired,
    )
    .is_err());

    let retained = WaiverRegistry::parse(
        expired,
        "<inline>",
        &rules()?,
        today()?,
        ExpiryPolicy::RetainExpiredForAudit,
    )?;
    assert!(retained
        .matching("src/file.rs", &"SRC-1.1".parse()?, today()?)
        .is_none());
    Ok(())
}
