//! Test fixture path decoding.
//!
//! BOUNDARY-INVARIANT: raw catalog JSON is decoded and every Python rule id
//! is validated into the canonical `RuleId` before fixture tests consume it.
//! NEGATIVE-TEST: malformed catalog shapes are rejected by the decoder tests.

use std::collections::BTreeSet;
use std::path::PathBuf;

use enforcer_domain::ids::RuleId;

pub(crate) fn python_catalog_rule_ids() -> Result<BTreeSet<RuleId>, Box<dyn std::error::Error>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("rules/rules.json");
    let raw = std::fs::read_to_string(path)?;
    parse_python_catalog_rule_ids(&raw)
}

fn parse_python_catalog_rule_ids(
    raw: &str,
) -> Result<BTreeSet<RuleId>, Box<dyn std::error::Error>> {
    let parsed: serde_json::Value = serde_json::from_str(raw)?;
    let rules = parsed
        .get("rules")
        .and_then(serde_json::Value::as_array)
        .ok_or("rules/rules.json missing top-level `rules` array")?;
    let mut ids = BTreeSet::new();
    for rule in rules {
        if rule.get("language").and_then(serde_json::Value::as_str) != Some("python") {
            continue;
        }
        let id = rule
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or("python rule record missing `id`")?;
        ids.insert(id.parse()?);
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::parse_python_catalog_rule_ids;

    #[test]
    fn malformed_catalog_shape_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let error = match parse_python_catalog_rule_ids(r#"{"notRules":[]}"#) {
            Err(error) => error,
            Ok(_) => return Err("catalog without rules must fail".into()),
        };
        assert_eq!(
            error.to_string(),
            "rules/rules.json missing top-level `rules` array"
        );
        Ok(())
    }

    #[test]
    fn invalid_python_rule_id_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let error = match parse_python_catalog_rule_ids(
            r#"{"rules":[{"language":"python","id":"not a rule id"}]}"#,
        ) {
            Err(error) => error,
            Ok(_) => return Err("invalid rule id must fail".into()),
        };
        let decode = error
            .downcast_ref::<enforcer_domain::boundary::decode_error::DecodeError>()
            .ok_or("canonical RuleId rejection must retain DecodeError")?;
        assert_eq!(
            decode.path, "ruleId",
            "the canonical RuleId parser must own invalid-id rejection"
        );
        Ok(())
    }
}
