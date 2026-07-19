//! Typed configuration consumers use canonical values from `enforcer-domain`.

#[cfg(test)]
mod tests {
    use crate::serde::{decode_json, WirePlatform, WireRustScanScope};
    use enforcer_domain::config_types::{
        ConfigJson, ConfigSource, InlineTestPolicy, Platform, RustScanScope,
    };

    #[test]
    fn platform_all_returns_three_platforms_in_stable_order() {
        assert_eq!(
            Platform::all(),
            vec![Platform::Windows, Platform::Macos, Platform::Linux]
        );
    }

    #[test]
    fn platform_wire_form_is_lowercase() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            serde_json::to_string(&WirePlatform::Windows)?,
            "\"windows\""
        );
        let parsed: Platform = decode_json::<WirePlatform>(
            &ConfigJson::from_owned("\"linux\"".to_owned()),
            &ConfigSource::from_owned("platform fixture".to_owned()),
            "platform fixture must decode",
        )?
        .into();
        assert_eq!(parsed, Platform::Linux);
        Ok(())
    }

    #[test]
    fn inline_test_policy_defaults_to_forbid_and_accepts_each_mode(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let default_scope: RustScanScope = decode_json::<WireRustScanScope>(
            &ConfigJson::from_owned("{}".to_owned()),
            &ConfigSource::from_owned("rust scope fixture".to_owned()),
            "rust scope fixture must decode",
        )?
        .try_into()?;
        assert_eq!(default_scope.inline_test_policy, InlineTestPolicy::Forbid);
        for (wire, expected) in [
            ("\"forbid\"", InlineTestPolicy::Forbid),
            ("\"warn\"", InlineTestPolicy::Warn),
            ("\"allow\"", InlineTestPolicy::Allow),
        ] {
            let scope: RustScanScope = decode_json::<WireRustScanScope>(
                &ConfigJson::from_owned(format!("{{\"inlineTestPolicy\":{wire}}}")),
                &ConfigSource::from_owned("rust scope fixture".to_owned()),
                "rust scope fixture must decode",
            )?
            .try_into()?;
            assert_eq!(scope.inline_test_policy, expected);
        }
        Ok(())
    }
}
