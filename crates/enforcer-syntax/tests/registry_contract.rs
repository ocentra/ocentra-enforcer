//! Drift and denominator tests for the reviewed UL06 identity projection.
//!
//! Source owner: `crates/enforcer-syntax/src/parsers/mod.rs`.
//! Reviewed manifest: `crates/enforcer-syntax/registry/languages.json`.
//! This test validates the checked-in projection and is not generated source.

use enforcer_domain::language_types::{
    LanguageId, StructuralLanguageSupport, PARSER_IDENTITY_COUNT,
};
use enforcer_syntax::parsers::Language;
use enforcer_syntax::registry::{language_registry, record_for_parser};

use sha2::{Digest, Sha256};

const MANIFEST: &str = include_str!("../registry/languages.json");
const MANIFEST_SHA256: &str = "5235f9d44b4f256abeb82b846a39b98cc7a344cb7314e503e0e979449c13895a";

#[test]
fn registry_preserves_all_parser_identities_and_order() {
    let records = language_registry();
    assert_eq!(records.len(), usize::from(PARSER_IDENTITY_COUNT));
    for (index, record) in records.iter().enumerate() {
        assert_eq!(record.id, LanguageId::from_registry_index(index as u16 + 1));
        if index > 0 {
            assert!(records[index - 1].id < record.id);
        }
    }
    assert_eq!(
        records
            .iter()
            .filter(|record| record.structural == StructuralLanguageSupport::NoParseFile)
            .count(),
        4
    );
}

#[test]
fn registry_has_exact_non_structural_dispositions() -> Result<(), Box<dyn std::error::Error>> {
    let non_structural = [
        Language::ConfigToml,
        Language::ConfigJson,
        Language::ConfigYaml,
        Language::TextOnly,
    ];
    for language in non_structural {
        let record = record_for_parser(language)
            .ok_or_else(|| std::io::Error::other("every parser variant must be registered"))?;
        assert_eq!(record.structural, StructuralLanguageSupport::NoParseFile);
    }
    assert_eq!(
        language_registry()
            .iter()
            .filter(|record| record.structural == StructuralLanguageSupport::NoParseFile)
            .map(|record| record.parser)
            .collect::<Vec<_>>(),
        non_structural
    );
    Ok(())
}

#[test]
fn manifest_is_valid_and_matches_projection() -> Result<(), Box<dyn std::error::Error>> {
    let value: serde_json::Value = serde_json::from_str(MANIFEST)?;
    let identities = value
        .get("identities")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| std::io::Error::other("registry manifest must contain identities"))?;
    assert_eq!(identities.len(), language_registry().len());
    for (entry, record) in identities.iter().zip(language_registry()) {
        assert_eq!(
            entry.get("id").and_then(serde_json::Value::as_u64),
            Some(u64::from(record.id.as_u16()))
        );
        let parser_variant = format!("{:?}", record.parser);
        assert_eq!(
            entry
                .get("parserVariant")
                .and_then(serde_json::Value::as_str),
            Some(parser_variant.as_str())
        );
    }
    Ok(())
}

#[test]
fn manifest_hash_is_pinned_to_the_reviewed_source() {
    let actual = format!("{:x}", Sha256::digest(MANIFEST.as_bytes()));
    assert_eq!(actual, MANIFEST_SHA256);
}

#[test]
fn lookup_is_total_for_registered_parser_variants() -> Result<(), Box<dyn std::error::Error>> {
    for record in language_registry() {
        let found = record_for_parser(record.parser)
            .ok_or_else(|| std::io::Error::other("registered parser lookup must be total"))?;
        assert_eq!(found, record);
    }
    let rust = record_for_parser(Language::Rust)
        .ok_or_else(|| std::io::Error::other("Rust must remain registered"))?;
    assert_eq!(rust.parser, Language::Rust);
    Ok(())
}
