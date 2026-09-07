//! Contract proof for the JSON-owned UL06 P1A1 registry.
// Source owner: crates/enforcer-syntax/registry/languages.json
// Generator: crates/enforcer-syntax/src/boundary/language_registry_build.rs
// schemaHash: 97dff487a6e01afbab60d36452243bfd68d766ea1ae119350d44633b25c7d878

use enforcer_domain::language_types::{
    DetectionMatcher, DetectionMatcherKind, DetectionPrecedenceTieBreak, InvalidLanguageId,
    LanguageId, LiteralDisposition, LiteralProjection, LiteralProjectionDisposition,
    StructuralLanguageSupport, NO_LITERAL_PARSER_IDENTITY_COUNT,
};
use enforcer_syntax::boundary::language_registry::{render_source, validate_source};
use enforcer_syntax::parsers::Language;
use enforcer_syntax::registry::{
    collision_resolutions, detection_precedence, language_registry, literal_projections,
    record_for_parser,
};
use serde_json::Value;
use std::collections::HashSet;
use std::num::NonZeroU16;

const MANIFEST: &str = include_str!("../registry/languages.json");
const GENERATED: &str = include_str!(concat!(env!("OUT_DIR"), "/language_registry.rs"));

const EXPECTED_LITERAL_NAMES: &[&str] = &[
    "rust",
    "typescript",
    "javascript",
    "python",
    "c",
    "cpp",
    "csharp",
    "objective-c",
    "zig",
    "go",
    "d",
    "v",
    "nim",
    "java",
    "kotlin",
    "scala",
    "groovy",
    "swift",
    "dart",
    "php",
    "ruby",
    "perl",
    "lua",
    "r",
    "julia",
    "shell",
    "powershell",
    "batch",
    "make",
    "dockerfile",
    "haskell",
    "ocaml",
    "fsharp",
    "elm",
    "purescript",
    "elixir",
    "erlang",
    "clojure",
    "lisp",
    "sql",
    "graphql",
    "terraform",
    "nix",
    "starlark",
    "protobuf",
    "thrift",
    "solidity",
    "move",
    "apex",
    "qml",
    "cuda",
    "shader",
    "raku",
    "reason",
    "rescript",
    "sml",
    "avro",
    "html",
    "css",
    "json",
    "yaml",
    "toml",
    "env",
    "markdown",
    "xml",
    "csv",
    "coldfusion",
    "unknown",
];

type TestResult<T = ()> = Result<T, String>;

fn must_result<T, E: std::fmt::Debug>(result: Result<T, E>, label: &str) -> TestResult<T> {
    result.map_err(|error| format!("{label}: {error:?}"))
}

fn must_option<T>(value: Option<T>, label: &str) -> TestResult<T> {
    value.ok_or_else(|| format!("missing {label}"))
}

fn rejected<E: std::fmt::Debug>(result: Result<(), E>) -> TestResult<E> {
    match result {
        Ok(()) => Err("mutation was accepted".to_owned()),
        Err(error) => Ok(error),
    }
}

fn manifest_value() -> TestResult<Value> {
    must_result(
        serde_json::from_str(MANIFEST),
        "reviewed manifest must parse",
    )
}

fn expect_error(value: &Value, expected: &str) -> TestResult {
    let source = must_result(serde_json::to_string(value), "mutation must serialize")?;
    let error = rejected(validate_source(&source))?;
    assert!(
        format!("{error:?}").contains(expected),
        "expected {expected}, got {error:?}"
    );
    Ok(())
}

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
            let candidate = line.trim().trim_end_matches(',');
            if !candidate.is_empty()
                && candidate
                    .chars()
                    .all(|character| character == '_' || character.is_ascii_alphanumeric())
            {
                names.push(candidate.to_owned());
            }
        }
    }
    names
}

#[test]
fn registry_is_exhaustive_unique_and_structurally_honest() -> TestResult {
    let records = language_registry();
    assert_eq!(records.len(), 160);
    assert_eq!(Language::ALL.len(), 160);

    let registry_names = records
        .iter()
        .map(|record| format!("{:?}", record.parser()))
        .collect::<Vec<_>>();
    let all_names = Language::ALL
        .iter()
        .map(|language| format!("{:?}", language))
        .collect::<Vec<_>>();
    assert_eq!(parser_enum_source_names(), all_names);
    assert_eq!(registry_names, all_names);
    assert_eq!(registry_names.iter().collect::<HashSet<_>>().len(), 160);
    assert_eq!(
        records
            .iter()
            .filter(|record| record.structural() == StructuralLanguageSupport::NoParseFile)
            .count(),
        4
    );
    for language in [
        Language::ConfigToml,
        Language::ConfigJson,
        Language::ConfigYaml,
        Language::TextOnly,
    ] {
        let record = must_option(
            record_for_parser(language),
            "all parser variants must be registered",
        )?;
        assert_eq!(record.structural(), StructuralLanguageSupport::NoParseFile);
    }
    assert_eq!(
        detection_precedence().ordered_kinds(),
        &[
            DetectionMatcherKind::ExactBasename,
            DetectionMatcherKind::CompoundSuffix,
            DetectionMatcherKind::Extension
        ]
    );
    assert_eq!(
        detection_precedence().same_kind_tie_break(),
        DetectionPrecedenceTieBreak::LongestValue
    );
    Ok(())
}

#[test]
fn manifest_proves_exact_crosswalk_and_unmatched_denominators() -> TestResult {
    must_result(
        validate_source(MANIFEST),
        "manifest must satisfy the closed contract",
    )?;
    let value = manifest_value()?;
    let identities = must_option(value["identities"].as_array(), "identities array")?;
    let projection = must_option(
        value["literalProjection"].as_array(),
        "literalProjection array",
    )?;
    let unmatched = must_option(
        value["unmatchedParserIds"].as_array(),
        "unmatchedParserIds array",
    )?;
    assert_eq!(identities.len(), 160);
    assert_eq!(
        identities
            .iter()
            .filter(|row| row["structural"] == true)
            .count(),
        156
    );
    assert_eq!(projection.len(), 68);
    assert_eq!(unmatched.len(), NO_LITERAL_PARSER_IDENTITY_COUNT);
    assert_eq!(
        value["crosswalkCounts"],
        serde_json::json!({
            "total": 68,
            "named": 67,
            "oneToOne": 51,
            "aliasCollision": 3,
            "oneToMany": 8,
            "literalOnly": 5,
            "fallback": 1
        })
    );
    let actual_names = projection
        .iter()
        .map(|row| {
            row["literalName"]
                .as_str()
                .ok_or("missing literalName".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(actual_names, EXPECTED_LITERAL_NAMES);
    assert_eq!(literal_projections().len(), 68);
    assert_eq!(collision_resolutions().len(), 34);
    assert_eq!(
        value["detectionPrecedence"],
        serde_json::json!({
            "orderedKinds": ["exactBasename", "compoundSuffix", "extension"],
            "sameKindTieBreak": "longestValue"
        })
    );
    Ok(())
}

#[test]
fn language_id_bounds_reject_zero_and_above_registry() -> TestResult {
    assert!(NonZeroU16::new(0).is_none());
    let Some(first) = NonZeroU16::new(1) else {
        return Err("one must be a non-zero registry index".to_owned());
    };
    let Some(last) = NonZeroU16::new(160) else {
        return Err("160 must be a non-zero registry index".to_owned());
    };
    let Some(too_high) = NonZeroU16::new(161) else {
        return Err("161 must be a non-zero registry index".to_owned());
    };
    assert_eq!(
        LanguageId::try_from_registry_index(first),
        Ok(LanguageId::from_registry_index(first))
    );
    assert_eq!(
        LanguageId::try_from_registry_index(last),
        Ok(LanguageId::from_registry_index(last))
    );
    assert_eq!(
        LanguageId::try_from_registry_index(too_high),
        Err(InvalidLanguageId)
    );
    Ok(())
}

#[test]
fn metadata_has_typed_dispositions_and_collision_winners() -> TestResult {
    let value = manifest_value()?;
    for identity in must_option(value["identities"].as_array(), "identities")? {
        assert!(identity["canonicalName"].is_string());
        assert!(identity["aliases"].is_array());
        assert!(identity["detection"]["matchers"].is_array());
        assert!(identity["literalDisposition"]["kind"].is_string());
    }
    for resolution in must_option(
        value["collisionResolutions"].as_array(),
        "collision resolutions",
    )? {
        let members = must_option(resolution["members"].as_array(), "members")?;
        assert!(members.len() >= 2);
        assert!(resolution["winnerRef"].is_object());
        assert!(
            members
                .iter()
                .any(|member| member == &resolution["winnerRef"]),
            "winnerRef must be one of the typed members"
        );
        assert!(resolution.get("priority").is_none());
    }
    let unsupported = must_option(value["identities"].as_array(), "identities")?
        .iter()
        .filter(|identity| {
            matches!(
                identity["literalDisposition"]["kind"].as_str(),
                Some("unsupported" | "notApplicable")
            )
        })
        .count();
    assert_eq!(unsupported, 85);
    Ok(())
}

#[test]
fn regeneration_is_deterministic_and_schema_is_valid() -> TestResult {
    must_result(validate_source(MANIFEST), "manifest must validate")?;
    assert_eq!(
        must_result(render_source(MANIFEST), "manifest must render")?,
        GENERATED
    );
    Ok(())
}

fn normalized_matcher_key(matcher: DetectionMatcher) -> String {
    match matcher {
        DetectionMatcher::Extension(value) => format!("extension:{}", value.to_ascii_lowercase()),
        DetectionMatcher::ExactBasename(value) => {
            format!("exactBasename:{}", value.to_ascii_lowercase())
        }
        DetectionMatcher::CompoundSuffix(value) => {
            format!("compoundSuffix:{}", value.to_ascii_lowercase())
        }
    }
}

#[test]
fn typed_literal_projection_preserves_manifest_matchers() -> TestResult {
    let value = manifest_value()?;
    let manifest_rows = must_option(value["literalProjection"].as_array(), "literal projection")?;
    assert_eq!(manifest_rows.len(), 68);
    assert_eq!(literal_projections().len(), 68);

    let mut literal_only_count = 0;
    let mut fallback_count = 0;
    for (manifest_row, generated_row) in manifest_rows.iter().zip(literal_projections()) {
        let LiteralProjection::Row(name, disposition, _, matchers, _) = generated_row;
        assert_eq!(manifest_row["literalName"].as_str(), Some(*name));
        let expected = must_option(manifest_row["matcherKeys"].as_array(), "matcher keys")?
            .iter()
            .map(|key| {
                key.as_str()
                    .map(ToOwned::to_owned)
                    .ok_or("matcher key must be a string".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let actual = matchers
            .iter()
            .copied()
            .map(normalized_matcher_key)
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "typed matcher drift for {name}");
        match disposition {
            LiteralProjectionDisposition::LiteralOnly => literal_only_count += 1,
            LiteralProjectionDisposition::Fallback => {
                fallback_count += 1;
                assert_eq!(*name, "unknown");
                assert!(matchers.is_empty());
            }
            LiteralProjectionDisposition::Registered => {}
        }
    }
    assert_eq!(literal_only_count, 5);
    assert_eq!(fallback_count, 1);
    Ok(())
}

#[test]
fn duplicate_canonical_name_is_rejected() -> TestResult {
    let mut value = manifest_value()?;
    let identities = must_option(value["identities"].as_array_mut(), "identities")?;
    let name = identities[0]["canonicalName"].clone();
    identities[1]["canonicalName"] = name;
    expect_error(&value, "DuplicateCanonicalName")
}

#[test]
fn duplicate_alias_is_rejected() -> TestResult {
    let mut value = manifest_value()?;
    value["identities"][1]["aliases"] = serde_json::json!(["Rust"]);
    expect_error(&value, "DuplicateAlias")
}

#[test]
fn duplicate_normalized_matcher_is_rejected() -> TestResult {
    let mut value = manifest_value()?;
    let matcher = value["identities"][0]["detection"]["matchers"][0].clone();
    let matchers = must_option(
        value["identities"][0]["detection"]["matchers"].as_array_mut(),
        "matchers",
    )?;
    matchers.push(matcher);
    expect_error(&value, "DuplicateMatcher")
}

#[test]
fn missing_collision_resolution_is_rejected() -> TestResult {
    let mut value = manifest_value()?;
    must_option(
        value["collisionResolutions"].as_array_mut(),
        "collision resolutions",
    )?
    .remove(0);
    expect_error(&value, "CollisionResolution")
}

#[test]
fn duplicate_collision_winner_member_is_rejected() -> TestResult {
    let mut value = manifest_value()?;
    let resolution = &mut value["collisionResolutions"][0];
    let winner = resolution["winnerRef"].clone();
    must_option(resolution["members"].as_array_mut(), "members")?.push(winner);
    expect_error(&value, "CollisionResolution")
}

#[test]
fn invalid_winner_reference_is_rejected() -> TestResult {
    let mut value = manifest_value()?;
    value["collisionResolutions"][0]["winnerRef"] =
        serde_json::json!({"kind": "parserId", "parserId": 999});
    expect_error(&value, "InvalidReference")
}

#[test]
fn incomplete_crosswalk_is_rejected() -> TestResult {
    let mut value = manifest_value()?;
    must_option(
        value["literalProjection"].as_array_mut(),
        "literalProjection",
    )?
    .remove(0);
    expect_error(&value, "LiteralProjectionCount")
}

#[test]
fn malformed_supplemental_row_is_rejected() -> TestResult {
    let mut value = manifest_value()?;
    let projection = must_option(value["literalProjection"].as_array(), "literalProjection")?;
    let row = must_option(
        projection
            .iter()
            .position(|row| row["classification"] == "literalOnly"),
        "literal-only row",
    )?;
    value["literalProjection"][row]["parserIds"] = serde_json::json!([1]);
    expect_error(&value, "InvalidReference")
}

#[test]
fn unknown_parser_variant_fails_generation_validation() -> TestResult {
    let mut value = manifest_value()?;
    value["identities"][0]["parserVariant"] = "UnknownLanguage".into();
    expect_error(&value, "UnknownParserVariant")
}

#[test]
fn invalid_detection_precedence_is_rejected() -> TestResult {
    let mut value = manifest_value()?;
    value["detectionPrecedence"]["orderedKinds"] =
        serde_json::json!(["extension", "compoundSuffix", "exactBasename"]);
    expect_error(&value, "DetectionPrecedence")
}

#[test]
fn longest_compound_suffix_tie_rule_is_explicit() -> TestResult {
    let value = manifest_value()?;
    assert_eq!(
        value["detectionPrecedence"]["sameKindTieBreak"],
        "longestValue"
    );
    assert_eq!(
        detection_precedence().same_kind_tie_break(),
        DetectionPrecedenceTieBreak::LongestValue
    );
    assert!(".env.local".len() > ".env".len());
    Ok(())
}

#[test]
fn no_literal_identities_are_not_fabricated() -> TestResult {
    let value = manifest_value()?;
    let unmatched = must_option(value["unmatchedParserIds"].as_array(), "unmatchedParserIds")?;
    for id in unmatched {
        let index = must_option(id.as_u64(), "numeric parser ID")? as usize - 1;
        let identity = &value["identities"][index];
        assert!(must_option(identity["detection"]["matchers"].as_array(), "matchers")?.is_empty());
        assert!(matches!(
            identity["literalDisposition"]["kind"].as_str(),
            Some("unsupported" | "notApplicable")
        ));
    }
    for record in language_registry() {
        if matches!(
            record.literal_disposition(),
            LiteralDisposition::Unsupported | LiteralDisposition::NotApplicable
        ) {
            assert!(record.matchers().is_empty());
        }
    }
    Ok(())
}
