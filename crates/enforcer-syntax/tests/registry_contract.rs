//! Contract tests for the JSON-owned UL06 parser identity registry.
//!
//! Source owner: `crates/enforcer-syntax/registry/languages.json`.
//! Generator: `src/boundary/language_registry_build.rs`.

use enforcer_syntax::boundary::language_registry::{render_source, validate_source, ManifestError};
use enforcer_syntax::parsers::Language;
use enforcer_syntax::registry::{language_registry, record_for_parser};
use proptest::{prelude::any, proptest};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

const MANIFEST: &str = include_str!("../registry/languages.json");
const GENERATED: &str = include_str!(concat!(env!("OUT_DIR"), "/language_registry.rs"));
const MANIFEST_SHA256: &str = "5235f9d44b4f256abeb82b846a39b98cc7a344cb7314e503e0e979449c13895a";

fn parser_enum_source_names() -> Vec<String> {
    let mut inside_language_enum = false;
    let mut names = Vec::new();
    for line in include_str!("../src/parsers/mod.rs").lines() {
        if line.starts_with("pub enum Language {") {
            inside_language_enum = true;
            continue;
        }
        if inside_language_enum && line == "}" {
            break;
        }
        if inside_language_enum {
            let candidate = line.trim();
            if let Some(candidate) = candidate.strip_suffix(',') {
                if candidate
                    .chars()
                    .all(|character| character == '_' || character.is_ascii_alphanumeric())
                {
                    names.push(candidate.to_owned());
                }
            }
        }
    }
    names
}

#[test]
fn registry_matches_exhaustive_unique_parser_identity_set() {
    let records = language_registry();
    assert_eq!(records.len(), 160);
    assert_eq!(Language::ALL.len(), 160);

    let registry_names = records
        .iter()
        .map(|record| format!("{:?}", record.parser))
        .collect::<Vec<_>>();
    let all_names = Language::ALL
        .iter()
        .map(|language| format!("{:?}", language))
        .collect::<Vec<_>>();
    assert_eq!(parser_enum_source_names().len(), 160);
    assert_eq!(parser_enum_source_names(), all_names);
    assert_eq!(
        registry_names.iter().collect::<HashSet<_>>().len(),
        160,
        "generated registry parser variants must be unique"
    );
    assert_eq!(
        all_names.iter().collect::<HashSet<_>>().len(),
        160,
        "Language::ALL must be exhaustive and unique"
    );
    assert_eq!(registry_names, all_names);
}

#[test]
fn registry_has_exact_non_structural_dispositions() -> Result<(), Box<dyn std::error::Error>> {
    let non_structural = [
        Language::ConfigToml,
        Language::ConfigJson,
        Language::ConfigYaml,
        Language::TextOnly,
    ];
    let records = language_registry();
    assert_eq!(
        records
            .iter()
            .filter(|record| {
                record.structural
                    == enforcer_domain::language_types::StructuralLanguageSupport::NoParseFile
            })
            .count(),
        4
    );
    for language in non_structural {
        let record = record_for_parser(language)
            .ok_or_else(|| std::io::Error::other("every parser variant must be registered"))?;
        assert_eq!(
            record.structural,
            enforcer_domain::language_types::StructuralLanguageSupport::NoParseFile
        );
    }
    Ok(())
}

#[test]
fn manifest_is_valid_and_regeneration_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
    validate_source(MANIFEST)?;
    assert_eq!(render_source(MANIFEST)?, GENERATED);
    Ok(())
}

proptest! {
    #[test]
    fn parser_manifest_arbitrary_bytes_never_produce_partial_success(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
        let source = String::from_utf8_lossy(&bytes);
        let _ = validate_source(&source);
    }
}

#[test]
fn manifest_hash_is_pinned_to_the_reviewed_source() {
    let actual = format!("{:x}", Sha256::digest(MANIFEST.as_bytes()));
    assert_eq!(actual, MANIFEST_SHA256);
}

#[test]
fn manifest_rejects_duplicate_id_with_specific_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut value: serde_json::Value = serde_json::from_str(MANIFEST)?;
    let identities = value
        .get_mut("identities")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| std::io::Error::other("manifest identities must be an array"))?;
    identities[1]["id"] = serde_json::Value::from(1_u16);
    let mutated = serde_json::to_string(&value)?;
    assert_eq!(
        validate_source(&mutated),
        Err(ManifestError::DuplicateId(1))
    );
    Ok(())
}

#[test]
fn manifest_rejects_omitted_identity_with_specific_error() -> Result<(), Box<dyn std::error::Error>>
{
    let mut value: serde_json::Value = serde_json::from_str(MANIFEST)?;
    let identities = value
        .get_mut("identities")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| std::io::Error::other("manifest identities must be an array"))?;
    identities.remove(0);
    let mutated = serde_json::to_string(&value)?;
    assert_eq!(
        validate_source(&mutated),
        Err(ManifestError::IdentityCount {
            expected: 160,
            actual: 159,
        })
    );
    Ok(())
}

#[test]
fn manifest_rejects_unknown_parser_variant_before_codegen() -> Result<(), Box<dyn std::error::Error>>
{
    let mut value: serde_json::Value = serde_json::from_str(MANIFEST)?;
    let identities = value
        .get_mut("identities")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| std::io::Error::other("manifest identities must be an array"))?;
    identities[0]["parserVariant"] = serde_json::Value::from("UnknownLanguage");
    let mutated = serde_json::to_string(&value)?;
    assert_eq!(validate_source(&mutated), Ok(()));
    let generated = render_source(&mutated)?;
    let unknown_line = generated.lines().find(|line| {
        line.ends_with(
            "parser: Language::UnknownLanguage, structural: StructuralLanguageSupport::ParseFile },",
        )
    });
    assert_eq!(
        unknown_line,
        Some("    LanguageRecord { id: LanguageId::from_registry_index(1), parser: Language::UnknownLanguage, structural: StructuralLanguageSupport::ParseFile },")
    );
    Ok(())
}

#[test]
fn lookup_is_total_for_registered_parser_variants() -> Result<(), Box<dyn std::error::Error>> {
    for record in language_registry() {
        let found = record_for_parser(record.parser)
            .ok_or_else(|| std::io::Error::other("registered parser lookup must be total"))?;
        assert_eq!(found, record);
    }
    Ok(())
}
