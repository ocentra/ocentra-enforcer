//! Source-shape policy values are canonical `enforcer-domain` DTOs; JSON
//! decoding and encoding remains in this crate's wire boundary.

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use crate::serde::{decode_json, WireSourceShapePolicy};
    use enforcer_domain::config_types::{
        ConfigField, ConfigJson, ConfigSource, SourceShapeKind, SourceShapePolicy,
    };
    use enforcer_domain::paths::RelPath;

    #[test]
    fn omitted_limit_deserializes_to_none_not_zero() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{
            "roots": ["Tools", "tools"],
            "extensions": [".rs"],
            "kind": "rust",
            "maxFunctionLines": 80,
            "maxFunctions": 18,
            "maxLines": 1000,
            "maxTypes": 24
        }"#;
        let wire: WireSourceShapePolicy = decode_json(
            &ConfigJson::from_owned(json.to_owned()),
            &ConfigSource::from_owned("shape fixture".to_owned()),
            "source shape fixture must decode",
        )?;
        let policy: SourceShapePolicy = wire.try_into()?;
        assert_eq!(policy.kind, SourceShapeKind::Rust);
        assert_eq!(policy.max_classes, None);
        assert_eq!(policy.max_exports, None);
        assert_eq!(policy.max_types.map(NonZeroUsize::get), Some(24));
        Ok(())
    }

    #[test]
    fn round_trips_through_serialize_deserialize() -> Result<(), Box<dyn std::error::Error>> {
        let policy = SourceShapePolicy {
            roots: vec![RelPath::try_from("src".to_owned())?],
            extensions: vec![ConfigField::from_owned(".ts".to_owned())],
            kind: SourceShapeKind::Typescript,
            max_classes: NonZeroUsize::new(1),
            max_exports: NonZeroUsize::new(35),
            max_functions: NonZeroUsize::new(30),
            max_function_lines: NonZeroUsize::new(80),
            max_lines: NonZeroUsize::new(1000),
            max_types: None,
            max_nesting_depth: NonZeroUsize::new(4),
            max_branches: NonZeroUsize::new(12),
        };
        let wire = serde_json::to_string(&WireSourceShapePolicy::from(policy.clone()))?;
        let back: SourceShapePolicy = decode_json::<WireSourceShapePolicy>(
            &ConfigJson::from_owned(wire),
            &ConfigSource::from_owned("shape round trip".to_owned()),
            "source shape wire must decode",
        )?
        .try_into()?;
        assert_eq!(policy, back);
        Ok(())
    }
}
