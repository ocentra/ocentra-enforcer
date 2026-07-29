//! Property-style boundary coverage for JSON decoding and project tie parsing.

use enforcer_config::error::ConfigLoadError;
use enforcer_config::project_tie::parse_project_tie;
use enforcer_config::serde::decode_json;
use enforcer_domain::config_types::{ConfigJson, ConfigSource};
use proptest::prelude::any;
use proptest::{prop_assert, prop_assert_eq, prop_assume, proptest};

#[test]
fn property_parse_json_value_accepts_valid_values_and_rejects_malformed_values() {
    for raw in ["null", "[]", "{}", "{\"value\":1}"] {
        let decoded: Result<serde_json::Value, _> = decode_json(
            &ConfigJson::from_owned(raw.to_owned()),
            &ConfigSource::from_owned("property-json".to_owned()),
            "property JSON decode",
        );
        assert_eq!(
            decoded.map(|value| value.is_null() || value.is_array() || value.is_object()),
            Ok(true)
        );
    }
    for raw in ["{", "[", "not-json", "{\"value\":}"] {
        let decoded: Result<serde_json::Value, _> = decode_json(
            &ConfigJson::from_owned(raw.to_owned()),
            &ConfigSource::from_owned("property-json".to_owned()),
            "property JSON decode",
        );
        assert!(matches!(decoded, Err(ConfigLoadError::Parse(_))));
    }
}

#[test]
fn property_project_tie_parser_rejects_unknown_native_tool_keys() {
    for suffix in ["alpha", "beta", "gamma", "delta"] {
        let raw = format!("{{\"native\":{{\"{suffix}\":{{\"mode\":\"augment\"}}}}}}");
        let outcome = parse_project_tie(
            &ConfigJson::from_owned(raw),
            &ConfigSource::from_owned("property-config.json".to_owned()),
        );
        assert!(matches!(outcome, Err(ConfigLoadError::Parse(_))));
    }
}

proptest! {
    #[test]
    fn generated_json_objects_round_trip_through_the_config_boundary(
        key in "[A-Za-z][A-Za-z0-9_]{0,15}",
        value in any::<i64>(),
    ) {
        let raw = serde_json::json!({key: value}).to_string();
        let decoded: Result<serde_json::Value, _> = decode_json(
            &ConfigJson::from_owned(raw),
            &ConfigSource::from_owned("generated-property-json".to_owned()),
            "generated property JSON decode",
        );
        prop_assert_eq!(
            decoded
                .ok()
                .and_then(|value| value.as_object().map(|_| ())),
            Some(())
        );
    }

    #[test]
    fn parse_project_tie_rejects_generated_unknown_native_tools(tool in "[a-z]{1,12}") {
        prop_assume!(!["cargo", "tsc", "ruff", "dart", "cflint"].contains(&tool.as_str()));
        let raw = serde_json::json!({
            "native": {tool: {"mode": "augment"}}
        })
        .to_string();
        let outcome = parse_project_tie(
            &ConfigJson::from_owned(raw),
            &ConfigSource::from_owned("generated-project-config.json".to_owned()),
        );
        prop_assert!(matches!(outcome, Err(ConfigLoadError::Parse(_))));
    }
}
