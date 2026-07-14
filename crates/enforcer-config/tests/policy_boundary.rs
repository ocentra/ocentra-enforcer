//! Public policy DTO boundary and invariant coverage.

use std::collections::BTreeMap;
use std::str::FromStr;

use enforcer_config::policy::{Policy, RuleToggle, Waiver};
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;

fn rule_id(value: &str) -> Result<RuleId, enforcer_core::error::DecodeError> {
    RuleId::from_str(value)
}

fn toggle(enabled: bool, severity: Option<Severity>, waiver: Option<Waiver>) -> RuleToggle {
    RuleToggle {
        enabled,
        severity,
        waiver,
    }
}

#[test]
fn absent_toggle_means_enabled() -> Result<(), Box<dyn std::error::Error>> {
    assert!(Policy::default().is_rule_enabled(&rule_id("RR-1.1")?));
    Ok(())
}

#[test]
fn disabled_rule_without_waiver_fails_validation() -> Result<(), Box<dyn std::error::Error>> {
    let id = rule_id("RR-1.1")?;
    let policy = Policy {
        rule_toggles: BTreeMap::from([(id, toggle(false, None, None))]),
        ..Policy::default()
    };
    let Err(reason) = policy.validate() else {
        return Err("disabled rule without waiver unexpectedly validated".into());
    };
    assert_eq!(
        reason,
        "rule `RR-1.1` is disabled but carries no waiver (owner + reason required; inline/silent disables are banned)"
    );
    Ok(())
}

#[test]
fn disabled_rule_with_matching_waiver_passes_validation() -> Result<(), Box<dyn std::error::Error>>
{
    let id = rule_id("RR-1.1")?;
    let waiver = Waiver {
        rule_id: id.clone(),
        owner: "platform-team".to_owned(),
        reason: "legacy module pending migration".to_owned(),
    };
    let policy = Policy {
        rule_toggles: BTreeMap::from([(id.clone(), toggle(false, None, Some(waiver)))]),
        ..Policy::default()
    };
    if let Err(reason) = policy.validate() {
        return Err(reason.into());
    }
    assert!(!policy.is_rule_enabled(&id));
    Ok(())
}

#[test]
fn supplied_waiver_is_validated_even_while_rule_is_enabled(
) -> Result<(), Box<dyn std::error::Error>> {
    let id = rule_id("RR-1.1")?;
    let policy = Policy {
        rule_toggles: BTreeMap::from([(
            id,
            toggle(
                true,
                None,
                Some(Waiver {
                    rule_id: rule_id("RR-2.2")?,
                    owner: "team".to_owned(),
                    reason: "incorrect binding".to_owned(),
                }),
            ),
        )]),
        ..Policy::default()
    };
    let Err(reason) = policy.validate() else {
        return Err("mismatched waiver unexpectedly validated".into());
    };
    assert_eq!(
        reason,
        "waiver.ruleId `RR-2.2` does not match its map key `RR-1.1`"
    );
    Ok(())
}

#[test]
fn disabled_rule_does_not_expose_a_severity_override() -> Result<(), Box<dyn std::error::Error>> {
    let id = rule_id("RR-1.1")?;
    let policy = Policy {
        rule_toggles: BTreeMap::from([(id.clone(), toggle(false, Some(Severity::Warning), None))]),
        ..Policy::default()
    };
    assert_eq!(
        policy.effective_severity(&id, Severity::Error),
        Severity::Error
    );
    Ok(())
}

#[test]
fn enabled_rule_severity_override_wins_and_round_trips() -> Result<(), Box<dyn std::error::Error>> {
    let id = rule_id("RR-1.1")?;
    let policy = Policy {
        rule_toggles: BTreeMap::from([(id.clone(), toggle(true, Some(Severity::Warning), None))]),
        skip_cfg_test: true,
        ..Policy::default()
    };
    assert_eq!(
        policy.effective_severity(&id, Severity::Error),
        Severity::Warning
    );
    let wire = serde_json::to_string(&policy)?;
    let decoded: Policy = serde_json::from_str(&wire)?;
    assert_eq!(decoded, policy);
    Ok(())
}
