//! a08 acceptance proof: path-scoped, branded-rule waivers load only through
//! a strict boundary and never become a project-wide rule toggle.

use enforcer_domain::paths::RelPath;
use enforcer_domain::rules_types::{
    RuleCatalogJson, RuleCatalogSource, WaiverDocumentJson, WaiverDocumentSource, WaiverExpiryDate,
};
use enforcer_rules::loader::{load_registry_from_records, parse_catalog};
use enforcer_rules::waiver::{ExpiryPolicy, WaiverRegistry};

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
    let raw = RuleCatalogJson::try_from(RULES.to_owned())?;
    let source = RuleCatalogSource::try_from("<inline rules>".to_owned())?;
    load_registry_from_records(parse_catalog(&raw, &source)?).map_err(Into::into)
}

fn today() -> Result<WaiverExpiryDate, enforcer_rules::waiver::WaiverLoadError> {
    "2026-07-10".parse().map_err(
        |error| enforcer_rules::waiver::WaiverLoadError::InvalidExpiry {
            value: enforcer_rules::boundary_reason(error),
        },
    )
}

fn parse_waivers(
    raw: &str,
    source: &str,
    rules: &enforcer_rules::registry::RuleRegistry,
    today: &WaiverExpiryDate,
    expiry_policy: ExpiryPolicy,
) -> enforcer_rules::waiver::WaiverResult<WaiverRegistry> {
    let raw = WaiverDocumentJson::try_from(raw.to_owned()).map_err(|error| {
        enforcer_rules::waiver::WaiverLoadError::InvalidPath {
            detail: enforcer_rules::boundary_reason(error),
        }
    })?;
    let source = WaiverDocumentSource::try_from(source.to_owned()).map_err(|error| {
        enforcer_rules::waiver::WaiverLoadError::InvalidPath {
            detail: enforcer_rules::boundary_reason(error),
        }
    })?;
    WaiverRegistry::parse(&raw, &source, rules, today, expiry_policy)
}

#[test]
fn packaged_empty_registry_loads_at_the_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let registry = parse_waivers(
        include_str!("../waivers.json"),
        "waivers.json",
        &rules()?,
        &today()?,
        ExpiryPolicy::RejectExpired,
    )?;
    assert!(registry.iter().next().is_none());
    Ok(())
}

#[test]
fn known_rule_and_exact_path_match_only_that_finding() -> Result<(), Box<dyn std::error::Error>> {
    let registry = parse_waivers(
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
        &today()?,
        ExpiryPolicy::RejectExpired,
    )?;
    let rule_id = "SRC-1.1".parse()?;
    assert!(registry
        .matching(
            &RelPath::try_from("crates\\enforcer-cli\\src\\legacy.rs".to_owned())?,
            &rule_id,
            &today()?
        )
        .is_some());
    assert!(registry
        .matching(
            &RelPath::try_from("crates/enforcer-cli/src/other.rs".to_owned())?,
            &rule_id,
            &today()?
        )
        .is_none());
    assert!(registry
        .matching(
            &RelPath::try_from("crates/enforcer-cli/src/legacy.rs".to_owned())?,
            &"SRC-1.2".parse()?,
            &today()?
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
        assert!(parse_waivers(
            invalid,
            "<inline>",
            &rules()?,
            &today()?,
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
        assert!(parse_waivers(
            invalid,
            "<inline>",
            &rules()?,
            &today()?,
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
    assert!(parse_waivers(
        expired,
        "<inline>",
        &rules()?,
        &today()?,
        ExpiryPolicy::RejectExpired,
    )
    .is_err());

    let retained = parse_waivers(
        expired,
        "<inline>",
        &rules()?,
        &today()?,
        ExpiryPolicy::RetainExpiredForAudit,
    )?;
    assert!(retained
        .matching(
            &RelPath::try_from("src/file.rs".to_owned())?,
            &"SRC-1.1".parse()?,
            &today()?
        )
        .is_none());
    Ok(())
}
