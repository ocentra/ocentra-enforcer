//! Serde adapters owned by the installation transport boundary.
//!
//! Canonical installation values live in `enforcer-domain` and deliberately
//! have no transport dependency. This module preserves the established wire
//! spellings only where JSON crosses the installer boundary.

//! BOUNDARY-INVARIANT: serialized install values convert through canonical domain types.
//! Negative invalid inputs are rejected during wire decoding.
//!
use enforcer_domain::install_types::{
    ArtifactKind, Cap, CommandName, DryRun, FindingKind, HookEvent, InstallOutputMode,
    InstallScope, Support,
};
use serde::{Deserialize, Deserializer, Serializer};

macro_rules! string_wire_enum {
    ($module:ident, $type:ident, { $($wire:literal => $variant:path),+ $(,)? }) => {
        pub mod $module {
            use super::{Deserialize, Deserializer, Serializer, $type};

            /// Serialize the canonical value using its stable installer wire spelling.
            pub fn serialize<S>(value: &$type, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(match value { $( $variant => $wire, )+ })
            }

            /// Decode a stable installer wire spelling into its canonical value.
            pub fn deserialize<'de, D>(deserializer: D) -> Result<$type, D::Error>
            where
                D: Deserializer<'de>,
            {
                match String::deserialize(deserializer)?.as_str() {
                    $( $wire => Ok($variant), )+
                    value => Err(serde::de::Error::unknown_variant(value, &[$($wire),+])),
                }
            }
        }
    };
}

string_wire_enum!(artifact_kind, ArtifactKind, {
    "mcpRegistration" => ArtifactKind::McpRegistration,
    "cargoAlias" => ArtifactKind::CargoAlias,
    "precommitHook" => ArtifactKind::PrecommitHook,
    "doctrineReference" => ArtifactKind::DoctrineReference,
    "harnessSpecific" => ArtifactKind::HarnessSpecific,
});

string_wire_enum!(command_name, CommandName, {
    "install" => CommandName::Install,
    "uninstall" => CommandName::Uninstall,
    "update" => CommandName::Update,
    "doctor" => CommandName::Doctor,
});

string_wire_enum!(install_scope, InstallScope, {
    "user" => InstallScope::User,
    "project" => InstallScope::Project,
});

string_wire_enum!(dry_run, DryRun, {
    "disabled" => DryRun::Disabled,
    "enabled" => DryRun::Enabled,
});

string_wire_enum!(install_output_mode, InstallOutputMode, {
    "human" => InstallOutputMode::Human,
    "json" => InstallOutputMode::Json,
});

string_wire_enum!(hook_event, HookEvent, {
    "sessionStart" => HookEvent::SessionStart,
});

string_wire_enum!(finding_kind, FindingKind, {
    "legacyServerRegistration" => FindingKind::LegacyServerRegistration,
    "conflictingServerRegistration" => FindingKind::ConflictingServerRegistration,
    "legacyToolNameLiteral" => FindingKind::LegacyToolNameLiteral,
    "legacySkillDirPresent" => FindingKind::LegacySkillDirPresent,
});

string_wire_enum!(support, Support, {
    "yes" => Support::Yes,
    "no" => Support::No,
    "unknown" => Support::Unknown,
});

pub mod cap {
    use super::{Cap, Deserialize, Deserializer, Serializer};
    use serde::de::Error;
    use serde::ser::SerializeMap;

    /// Serialize a capability using its stable string or bounded-map wire form.
    pub fn serialize<S>(value: &Cap, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Cap::Bounded(limit) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("bounded", limit)?;
                map.end()
            }
            Cap::Unbounded => serializer.serialize_str("unbounded"),
            Cap::Unknown => serializer.serialize_str("unknown"),
        }
    }

    /// Decode the stable capability wire form into its canonical value.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Cap, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::String(value) if value == "unbounded" => Ok(Cap::Unbounded),
            serde_json::Value::String(value) if value == "unknown" => Ok(Cap::Unknown),
            serde_json::Value::Object(mut fields) => match fields.remove("bounded") {
                Some(serde_json::Value::Number(value)) => value
                    .as_u64()
                    .and_then(|value| u32::try_from(value).ok())
                    .map(Cap::Bounded)
                    .ok_or_else(|| D::Error::custom("bounded cap must be a u32")),
                _ => Err(D::Error::custom(
                    "cap must be bounded, unbounded, or unknown",
                )),
            },
            _ => Err(D::Error::custom(
                "cap must be bounded, unbounded, or unknown",
            )),
        }
    }
}
