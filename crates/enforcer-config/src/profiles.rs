//! Canonical global profiles, embedded into the binary at compile time so
//! the engine is self-contained ("one binary IS the engine" doctrine) — no
//! external `profiles/` directory is required for baseline operation. A
//! project may still supply additional custom profiles as external files
//! (not modeled here: `crate::resolve` accepts a raw JSON string for those).

use crate::error::{ConfigLoadError, ConfigResult};

/// The known-profile name set (mechanical mirror of the legacy `.mjs`
/// `knownProfiles` set enforced by `CFG-1.11`).
pub const KNOWN_PROFILE_NAMES: [&str; 4] =
    ["strict", "default", "ocentra-enforcer", "ocentra-parent"];

const STRICT_JSON: &str = include_str!("../profiles/strict.json");
const DEFAULT_JSON: &str = include_str!("../profiles/default.json");
const OCENTRA_ENFORCER_JSON: &str = include_str!("../profiles/ocentra-enforcer.json");
const OCENTRA_PARENT_JSON: &str = include_str!("../profiles/ocentra-parent.json");

/// Look up the embedded canonical profile JSON by name. Returns
/// `ConfigLoadError::UnknownProfile` (mechanical mirror of `CFG-1.11`) for
/// any name outside [`KNOWN_PROFILE_NAMES`].
pub fn embedded_profile_json(profile_name: &str) -> ConfigResult<&'static str> {
    match profile_name {
        "strict" => Ok(STRICT_JSON),
        "default" => Ok(DEFAULT_JSON),
        "ocentra-enforcer" => Ok(OCENTRA_ENFORCER_JSON),
        "ocentra-parent" => Ok(OCENTRA_PARENT_JSON),
        other => Err(ConfigLoadError::UnknownProfile {
            path: "<embedded profile lookup>".to_owned(),
            profile_name: other.to_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{embedded_profile_json, KNOWN_PROFILE_NAMES};

    #[test]
    fn every_known_profile_name_resolves_to_embedded_json() -> Result<(), Box<dyn std::error::Error>>
    {
        for name in KNOWN_PROFILE_NAMES {
            let raw = embedded_profile_json(name)?;
            let value: serde_json::Value = serde_json::from_str(raw)?;
            assert_eq!(
                value.get("profileName").and_then(serde_json::Value::as_str),
                Some(name)
            );
        }
        Ok(())
    }

    #[test]
    fn unknown_profile_name_is_rejected() {
        let outcome = embedded_profile_json("nonexistent");
        assert!(outcome.is_err());
    }
}
