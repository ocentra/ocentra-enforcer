//! UL01 proof: requirement and framework family are separate profile data.

use enforcer_config::doctrine_profile::{decode_profile, encode_profile, load_embedded_profile};
use enforcer_domain::config_types::{ConfigProfileName, ConfigSource};
use enforcer_domain::doctrine_profile_types::{
    DoctrineFrameworkFamily, DoctrineLanguage, DoctrineRequirement, DoctrineVerdict,
};
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;

const EFFECT_DEFAULT: &str = include_str!("../profiles/doctrine/effect-default.json");
const ZOD_PROFILE: &str = include_str!("../profiles/doctrine/zod.json");
const MALFORMED: &str = include_str!("fixtures/doctrine_profile/malformed-unknown-family.json");
const DISABLED: &str = include_str!("fixtures/doctrine_profile/disabled-requirement.json");

fn source() -> ConfigSource {
    ConfigSource::from_owned("doctrine profile test".to_owned())
}

#[test]
fn the_same_shape_flips_only_with_the_selected_profile() -> Result<(), Box<dyn std::error::Error>> {
    let source = source();
    let effect = decode_profile(EFFECT_DEFAULT, &source)?;
    let zod = decode_profile(ZOD_PROFILE, &source)?;

    assert_eq!(
        effect.resolve(
            DoctrineLanguage::Typescript,
            DoctrineRequirement::SchemaRequired,
            DoctrineFrameworkFamily::Zod,
        ),
        DoctrineVerdict::Rejected
    );
    assert_eq!(
        zod.resolve(
            DoctrineLanguage::Typescript,
            DoctrineRequirement::SchemaRequired,
            DoctrineFrameworkFamily::Zod,
        ),
        DoctrineVerdict::Accepted
    );
    assert_eq!(
        effect.resolve(
            DoctrineLanguage::Typescript,
            DoctrineRequirement::SchemaRequired,
            DoctrineFrameworkFamily::Effect,
        ),
        DoctrineVerdict::Accepted
    );
    Ok(())
}

#[test]
fn disabled_requirement_is_visible_and_explained() -> Result<(), Box<dyn std::error::Error>> {
    let source = source();
    let profile = decode_profile(DISABLED, &source)?;
    assert_eq!(
        profile.resolve(
            DoctrineLanguage::Typescript,
            DoctrineRequirement::ParseAtBoundary,
            DoctrineFrameworkFamily::Effect,
        ),
        DoctrineVerdict::RequirementDisabled
    );
    let policy = profile
        .requirements()
        .find(|(requirement, _)| **requirement == DoctrineRequirement::ParseAtBoundary)
        .map(|(_, policy)| policy)
        .ok_or("disabled requirement row missing")?;
    assert_eq!(
        policy.owner().map(|value| value.as_str()),
        Some("platform-team")
    );
    assert_eq!(
        policy.reason().map(|value| value.as_str()),
        Some("The repository is migrating legacy boundaries in a staged release.")
    );
    Ok(())
}

#[test]
fn malformed_family_fails_with_a_typed_field_error() -> Result<(), Box<dyn std::error::Error>> {
    let source = source();
    let error = decode_profile(MALFORMED, &source)
        .err()
        .ok_or("an unknown framework family must not silently default")?;
    let message = error.to_string();
    assert!(
        message.contains("requirements[0].families[2].family"),
        "unexpected malformed-family diagnostic: {message}"
    );
    assert!(
        message.contains("unsupported doctrine framework family"),
        "unexpected malformed-family reason: {message}"
    );
    Ok(())
}

#[test]
fn incompatible_language_family_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let source = source();
    let raw = ZOD_PROFILE.replace("\"typescript\"", "\"python\"");
    let error = decode_profile(&raw, &source)
        .err()
        .ok_or("a TypeScript family must not be accepted by a Python profile")?;
    let message = error.to_string();
    assert!(
        message.contains("not valid for language `python`"),
        "unexpected language-family diagnostic: {message}"
    );
    Ok(())
}

#[test]
fn shipped_profiles_round_trip_without_losing_rule_or_family_toggles(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = source();
    for (raw, name) in [(EFFECT_DEFAULT, "effect-default"), (ZOD_PROFILE, "zod")] {
        let profile = decode_profile(raw, &source)?;
        let encoded = encode_profile(&profile)?;
        let original: serde_json::Value = serde_json::from_str(raw)?;
        let round_trip: serde_json::Value = serde_json::from_str(&encoded)?;
        assert_eq!(
            round_trip, original,
            "profile `{name}` changed on round-trip"
        );
    }
    let effect = load_embedded_profile(&ConfigProfileName::new("effect-default".to_owned())?)?;
    let rule_id = RuleId::try_from("FE-EFFECT-1.1".to_owned())?;
    let toggle = effect
        .rule_policy(&rule_id)
        .ok_or("shipped default rule toggle missing")?;
    assert_eq!(toggle.severity(), Severity::Error);
    Ok(())
}
