//! Closed manifest decoder and deterministic registry renderer for UL06 P1A1.
//!
//! BOUNDARY-INVARIANT: this module accepts only the reviewed JSON manifest,
//! validates its closed shape at the build boundary, and emits typed static
//! registry values; it never performs source parsing or policy decisions.
//!
//! The manifest envelope is the only raw wire type. Nested JSON values are
//! validated manually before they become private typed validation values.

use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

const EXPECTED_SCHEMA_VERSION: u16 = 2;
const EXPECTED_IDENTITY_COUNT: usize = 160;
const EXPECTED_STRUCTURAL_COUNT: usize = 156;
const EXPECTED_NO_PARSE_COUNT: usize = 4;
const EXPECTED_LITERAL_COUNT: usize = 68;
const EXPECTED_NAMED_LITERAL_COUNT: usize = 67;
const EXPECTED_NO_LITERAL_COUNT: usize = 85;
const PARSER_ENUM_SOURCE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/parsers/mod.rs"));

const KEY_ID: &str = "id";
const KEY_PARSER_VARIANT: &str = "parserVariant";
const KEY_STRUCTURAL: &str = "structural";
const KEY_CANONICAL_NAME: &str = "canonicalName";
const KEY_ALIASES: &str = "aliases";
const KEY_DETECTION: &str = "detection";
const KEY_MATCHERS: &str = "matchers";
const KEY_LITERAL_DISPOSITION: &str = "literalDisposition";
const KEY_LITERAL_NAME: &str = "literalName";
const KEY_KIND: &str = "kind";
const KEY_VALUE: &str = "value";
const KEY_PARSER_IDS: &str = "parserIds";
const KEY_MATCHER_KEYS: &str = "matcherKeys";
const KEY_WINNER_REFS: &str = "winnerRefs";
const KEY_MATCHER_KEY: &str = "matcherKey";
const KEY_WINNER_REF: &str = "winnerRef";
const KEY_MEMBERS: &str = "members";
const KEY_TOTAL: &str = "total";
const KEY_NAMED: &str = "named";
const KEY_ONE_TO_ONE: &str = "oneToOne";
const KEY_ALIAS_COLLISION: &str = "aliasCollision";
const KEY_ONE_TO_MANY: &str = "oneToMany";
const KEY_LITERAL_ONLY: &str = "literalOnly";
const KEY_FALLBACK: &str = "fallback";
const KEY_PARSER_ID: &str = "parserId";

const KIND_EXTENSION: &str = "extension";
const KIND_EXACT_BASENAME: &str = "exactBasename";
const KIND_COMPOUND_SUFFIX: &str = "compoundSuffix";
const KIND_REGISTERED: &str = "registered";
const KIND_UNSUPPORTED: &str = "unsupported";
const KIND_NOT_APPLICABLE: &str = "notApplicable";
const CLASS_ONE_TO_ONE: &str = "oneToOne";
const CLASS_ALIAS_COLLISION: &str = "aliasCollision";
const CLASS_ONE_TO_MANY: &str = "oneToMany";
const CLASS_LITERAL_ONLY: &str = "literalOnly";
const CLASS_FALLBACK: &str = "fallback";
const REFERENCE_PARSER_ID: &str = "parserId";
const REFERENCE_SUPPLEMENTAL: &str = "supplementalLiteralName";

const ERROR_OBJECT: &str = "manifest value must be an object";
const ERROR_ARRAY: &str = "manifest value must be an array";
const ERROR_STRING: &str = "manifest value must be a string";
const ERROR_BOOLEAN: &str = "manifest value must be a boolean";
const ERROR_INTEGER: &str = "manifest value must be a non-negative integer";
const ERROR_UNKNOWN_KIND: &str = "manifest contains an unknown kind";
const ERROR_UNKNOWN_CLASS: &str = "manifest contains an unknown classification";
const ERROR_UNKNOWN_DISPOSITION: &str = "manifest contains an unknown disposition";
const ERROR_UNKNOWN_REFERENCE: &str = "manifest contains an unknown reference kind";
const ERROR_EMPTY_MATCHER: &str = "empty matcher value";
const ERROR_FALLBACK_REFERENCE: &str = "fallback reference is only valid for the fallback row";
const ERROR_FALLBACK_ROW: &str = "fallback row must be unknown with no parser IDs";
const ERROR_UNMATCHED_COUNT: &str = "expected 85 unique parser IDs";
const ERROR_DEBUG: &str = "{self:?}";
const ENUM_START: &str = "pub enum Language {";
const ENUM_END: &str = "}";
const EMPTY: &str = "";
const LIST_PREFIX: &str = "&[";
const LIST_SUFFIX: &str = "]";
const COMMA_SPACE: &str = ", ";
const INDENT: &str = "    ";
const RECORD_PREFIX: &str = "LanguageRecord { id: language_id_from_registry_index(require_nonzero(NonZeroU16::new(";
const RECORD_PARSER: &str = "))), parser: Language::";
const RECORD_STRUCTURAL: &str = ", structural: ";
const RECORD_CANONICAL: &str = ", canonical_name: ";
const RECORD_ALIASES: &str = ", aliases: ";
const RECORD_MATCHERS: &str = ", matchers: ";
const RECORD_LITERAL: &str = ", literal_disposition: ";
const RECORD_SUFFIX: &str = " },\n";
const LITERAL_PREFIX: &str = "LiteralProjection::Row(";
const LITERAL_DISPOSITION: &str = ", ";
const LITERAL_PARSER_IDS: &str = ", ";
const LITERAL_MATCHER_KEYS: &str = ", ";
const LITERAL_WINNERS: &str = ", ";
const LITERAL_SUFFIX: &str = "),\n";
const COLLISION_PREFIX: &str = "CollisionResolution::Group(";
const COLLISION_KEY: &str = ", ";
const COLLISION_MEMBERS: &str = ", ";
const COLLISION_WINNER: &str = ", ";
const COLLISION_SUFFIX: &str = "),\n";
const MATCHER_EXTENSION_PREFIX: &str = "DetectionMatcher::Extension(";
const MATCHER_EXACT_BASENAME_PREFIX: &str = "DetectionMatcher::ExactBasename(";
const MATCHER_COMPOUND_SUFFIX_PREFIX: &str = "DetectionMatcher::CompoundSuffix(";
const MATCHER_SUFFIX: &str = ")";
const WINNER_PREFIX: &str = "MatcherWinner::Key(";
const WINNER_SEPARATOR: &str = ", ";
const WINNER_SUFFIX: &str = ")";
const PARSER_ID_PREFIX: &str = "language_id_from_registry_index(require_nonzero(NonZeroU16::new(";
const PARSER_ID_SUFFIX: &str = ")))";
const REFERENCE_PARSER_PREFIX: &str = "LiteralReference::ParserId(language_id_from_registry_index(require_nonzero(NonZeroU16::new(";
const REFERENCE_PARSER_SUFFIX: &str = "))))";
const REFERENCE_SUPPLEMENTAL_PREFIX: &str = "LiteralReference::SupplementalLiteralName(";
const REFERENCE_SUFFIX: &str = ")";
const REFERENCE_FALLBACK: &str = "LiteralReference::Fallback";
const REGISTERED_PREFIX: &str = "LiteralDisposition::Registered { literal_name: ";
const REGISTERED_SUFFIX: &str = " }";
const STRUCTURAL_PARSE: &str = "StructuralLanguageSupport::ParseFile";
const STRUCTURAL_NO_PARSE: &str = "StructuralLanguageSupport::NoParseFile";
const MATCHER_EXTENSION_RENDERED: &str = "DetectionMatcherKind::Extension";
const MATCHER_EXACT_BASENAME_RENDERED: &str = "DetectionMatcherKind::ExactBasename";
const MATCHER_COMPOUND_SUFFIX_RENDERED: &str = "DetectionMatcherKind::CompoundSuffix";
const LITERAL_UNSUPPORTED_RENDERED: &str = "LiteralDisposition::Unsupported";
const LITERAL_NOT_APPLICABLE_RENDERED: &str = "LiteralDisposition::NotApplicable";
const LITERAL_REGISTERED: &str = "LiteralProjectionDisposition::Registered";
const LITERAL_ONLY_RENDERED: &str = "LiteralProjectionDisposition::LiteralOnly";
const FALLBACK_RENDERED: &str = "LiteralProjectionDisposition::Fallback";
const UNKNOWN_LITERAL_NAME: &str = "unknown";
const MESSAGE_UNKNOWN_PARSER: &str = "unknown parserId ";
const MESSAGE_UNKNOWN_SUPPLEMENTAL: &str = "unknown supplemental literalName ";
const MESSAGE_ROW_PREFIX: &str = "row ";
const MESSAGE_ROW_UNKNOWN_PARSER: &str = " contains an unknown parserId";
const MESSAGE_ROW_REPEAT_PARSER: &str = " repeats a parserId";
const MESSAGE_ROW_LITERAL_ONLY_IDS: &str = " has parser IDs";
const MESSAGE_ROW_KEY_MISSING: &str = " key ";
const MESSAGE_KEY_MISSING_IDENTITY: &str = " is absent from parser identity ";
const MESSAGE_ROW_WINNER_COUNT: &str = " has one winnerRef per matcher key";
const MESSAGE_ROW_WINNER_UNKNOWN: &str = " has winnerRef for unknown key ";
const MESSAGE_ROW_WINNER_MEMBER: &str = " winnerRef is not a row member";
const MESSAGE_ROW_SUPPLEMENTAL: &str = " has an invalid supplemental winnerRef";
const MESSAGE_ROW_FALLBACK: &str = " has an invalid fallback winnerRef";
const MESSAGE_COMPLEMENT: &str = "parserId ";
const MESSAGE_COMPLEMENT_SUFFIX: &str = " is not an exact projection complement";
const MESSAGE_UNMATCHED_MATCHERS: &str = "unmatched parserId ";
const MESSAGE_HAS_MATCHERS: &str = " has matchers";
const MESSAGE_PROJECTED: &str = "projected parserId ";
const MESSAGE_NO_MATCHERS: &str = " has no matchers";
const MESSAGE_DUPLICATE_COLLISION: &str = "duplicate collision resolution ";
const MESSAGE_WINNER_NOT_MEMBER: &str = "winnerRef is not a member for ";
const MESSAGE_COLLISION_MISMATCH: &str = "collision resolution mismatch for ";

const GENERATED_LANGUAGE_HEADER: &str = "pub static LANGUAGE_RECORDS: &[LanguageRecord] = &[\n";
const GENERATED_LITERAL_HEADER: &str = "pub static LITERAL_PROJECTIONS: &[LiteralProjection] = &[\n";
const GENERATED_COLLISION_HEADER: &str = "pub static COLLISION_RESOLUTIONS: &[CollisionResolution] = &[\n";
const GENERATED_TABLE_END: &str = "];\n\n";
const GENERATED_FINAL_END: &str = "];\n";

#[derive(Debug, Deserialize)]
struct ManifestWire {
    #[serde(rename = "schemaVersion")]
    schema_version: u16,
    identities: Vec<Value>,
    #[serde(rename = "literalProjection")]
    literal_projection: Vec<Value>,
    #[serde(rename = "collisionResolutions")]
    collision_resolutions: Vec<Value>,
    #[serde(rename = "unmatchedParserIds")]
    unmatched_parser_ids: Vec<u16>,
    #[serde(rename = "crosswalkCounts")]
    crosswalk_counts: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MatcherKind {
    Extension,
    ExactBasename,
    CompoundSuffix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiteralDisposition {
    Registered,
    Unsupported,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CrosswalkClass {
    OneToOne,
    AliasCollision,
    OneToMany,
    LiteralOnly,
    Fallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectionDisposition {
    Registered,
    LiteralOnly,
    Fallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Reference {
    ParserId(u16),
    Supplemental(String),
    Fallback,
}

#[derive(Debug, Clone)]
struct MatcherValue {
    kind: MatcherKind,
    value: String,
}

#[derive(Debug, Clone)]
struct IdentityValue {
    id: u16,
    parser_variant: String,
    structural: bool,
    canonical_name: String,
    aliases: Vec<String>,
    matchers: Vec<MatcherValue>,
    literal_disposition: LiteralDisposition,
    literal_name: Option<String>,
}

#[derive(Debug, Clone)]
struct WinnerValue {
    matcher_key: String,
    winner_ref: Reference,
}

#[derive(Debug, Clone)]
struct ProjectionValue {
    literal_name: String,
    classification: CrosswalkClass,
    disposition: ProjectionDisposition,
    parser_ids: Vec<u16>,
    matcher_keys: Vec<String>,
    winner_refs: Vec<WinnerValue>,
}

#[derive(Debug, Clone)]
struct CollisionValue {
    kind: MatcherKind,
    normalized_key: String,
    members: Vec<Reference>,
    winner_ref: Reference,
}

/// A deterministic manifest validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// JSON did not match the closed manifest shape.
    Json(String),
    /// The schema version is unsupported.
    SchemaVersion(u16),
    /// The identity count drifted.
    IdentityCount { expected: usize, actual: usize },
    /// An identity is not at its required one-based position.
    NonSequentialId { position: usize, actual: u16 },
    /// Two rows use the same identity.
    DuplicateId(u16),
    /// Two rows use the same parser variant.
    DuplicateParserVariant(String),
    /// A parser variant is not present in the real enum source.
    UnknownParserVariant(String),
    /// A parser variant is not a Rust identifier.
    InvalidParserVariant(String),
    /// Canonical names collide after case folding.
    DuplicateCanonicalName(String),
    /// Aliases collide with canonical names or other aliases.
    DuplicateAlias(String),
    /// A matcher key repeats within one identity.
    DuplicateMatcher(String),
    /// The structural denominator drifted.
    StructuralCount { expected: usize, actual: usize },
    /// A literal projection row count drifted.
    LiteralProjectionCount { expected: usize, actual: usize },
    /// A crosswalk category count drifted.
    CrosswalkCounts(String),
    /// A literal row name repeats.
    DuplicateLiteralName(String),
    /// A typed reference is invalid for the manifest.
    InvalidReference(String),
    /// The unmatched parser denominator or complement drifted.
    UnmatchedParserIds(String),
    /// Collision resolutions are missing, duplicated, or malformed.
    CollisionResolution(String),
    /// A generated file could not be written.
    Io(String),
    /// Cargo did not provide an output directory.
    MissingOutputDirectory,
}

impl Display for ManifestError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(ERROR_DEBUG)
    }
}

impl std::error::Error for ManifestError {}

fn object(value: &Value) -> Result<&Map<String, Value>, ManifestError> {
    value.as_object().ok_or_else(|| ManifestError::Json(ERROR_OBJECT.to_owned()))
}

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, ManifestError> {
    object(value)?.get(key).ok_or_else(|| ManifestError::Json(format!("missing required field {key}")))
}

fn string_value(value: &Value) -> Result<String, ManifestError> {
    value.as_str().map(ToOwned::to_owned).ok_or_else(|| ManifestError::Json(ERROR_STRING.to_owned()))
}

fn string_field(value: &Value, key: &str) -> Result<String, ManifestError> {
    string_value(field(value, key)?)
}

fn bool_field(value: &Value, key: &str) -> Result<bool, ManifestError> {
    field(value, key)?.as_bool().ok_or_else(|| ManifestError::Json(ERROR_BOOLEAN.to_owned()))
}

fn integer_field(value: &Value, key: &str) -> Result<u16, ManifestError> {
    field(value, key)?
        .as_u64()
        .and_then(|number| u16::try_from(number).ok())
        .ok_or_else(|| ManifestError::Json(ERROR_INTEGER.to_owned()))
}

fn array_field<'a>(value: &'a Value, key: &str) -> Result<&'a [Value], ManifestError> {
    field(value, key)?.as_array().map(Vec::as_slice).ok_or_else(|| ManifestError::Json(ERROR_ARRAY.to_owned()))
}

fn fold(value: &str) -> String {
    value.to_ascii_lowercase()
}

fn message(parts: &[&str]) -> String {
    parts.join(EMPTY)
}

fn number_message(prefix: &str, number: u16, suffix: &str) -> String {
    let number_text = number.to_string();
    message(&[prefix, &number_text, suffix])
}

fn parse_matcher_kind(value: &str) -> Result<MatcherKind, ManifestError> {
    match value {
        value if value == KIND_EXTENSION => Ok(MatcherKind::Extension),
        value if value == KIND_EXACT_BASENAME => Ok(MatcherKind::ExactBasename),
        value if value == KIND_COMPOUND_SUFFIX => Ok(MatcherKind::CompoundSuffix),
        _ => Err(ManifestError::Json(ERROR_UNKNOWN_KIND.to_owned())),
    }
}

fn matcher_kind_name(kind: MatcherKind) -> &'static str {
    match kind {
        MatcherKind::Extension => KIND_EXTENSION,
        MatcherKind::ExactBasename => KIND_EXACT_BASENAME,
        MatcherKind::CompoundSuffix => KIND_COMPOUND_SUFFIX,
    }
}

fn matcher_key(kind: MatcherKind, value: &str) -> String {
    format!("{}:{}", matcher_kind_name(kind), fold(value))
}

fn parse_literal_disposition(value: &Value) -> Result<(LiteralDisposition, Option<String>), ManifestError> {
    let kind = string_field(value, KEY_KIND)?;
    match kind.as_str() {
        kind if kind == KIND_REGISTERED => Ok((LiteralDisposition::Registered, Some(string_field(value, KEY_LITERAL_NAME)?))),
        kind if kind == KIND_UNSUPPORTED => Ok((LiteralDisposition::Unsupported, None)),
        kind if kind == KIND_NOT_APPLICABLE => Ok((LiteralDisposition::NotApplicable, None)),
        _ => Err(ManifestError::Json(ERROR_UNKNOWN_DISPOSITION.to_owned())),
    }
}

fn parse_reference(value: &Value) -> Result<Reference, ManifestError> {
    let kind = string_field(value, KEY_KIND)?;
    match kind.as_str() {
        kind if kind == REFERENCE_PARSER_ID => Ok(Reference::ParserId(integer_field(value, KEY_PARSER_ID)?)),
        kind if kind == REFERENCE_SUPPLEMENTAL => Ok(Reference::Supplemental(string_field(value, KEY_LITERAL_NAME)?)),
        kind if kind == KEY_FALLBACK => Ok(Reference::Fallback),
        _ => Err(ManifestError::InvalidReference(ERROR_UNKNOWN_REFERENCE.to_owned())),
    }
}

fn parse_identity(value: &Value) -> Result<IdentityValue, ManifestError> {
    let detection = field(value, KEY_DETECTION)?;
    let matchers = array_field(detection, KEY_MATCHERS)?
        .iter()
        .map(|matcher| {
            Ok(MatcherValue {
                kind: parse_matcher_kind(&string_field(matcher, KEY_KIND)?)?,
                value: string_field(matcher, KEY_VALUE)?,
            })
        })
        .collect::<Result<Vec<_>, ManifestError>>()?;
    let aliases = array_field(value, KEY_ALIASES)?.iter().map(string_value).collect::<Result<Vec<_>, ManifestError>>()?;
    let (literal_disposition, literal_name) = parse_literal_disposition(field(value, KEY_LITERAL_DISPOSITION)?)?;
    Ok(IdentityValue {
        id: integer_field(value, KEY_ID)?,
        parser_variant: string_field(value, KEY_PARSER_VARIANT)?,
        structural: bool_field(value, KEY_STRUCTURAL)?,
        canonical_name: string_field(value, KEY_CANONICAL_NAME)?,
        aliases,
        matchers,
        literal_disposition,
        literal_name,
    })
}

fn parse_class(value: &str) -> Result<CrosswalkClass, ManifestError> {
    match value {
        value if value == CLASS_ONE_TO_ONE => Ok(CrosswalkClass::OneToOne),
        value if value == CLASS_ALIAS_COLLISION => Ok(CrosswalkClass::AliasCollision),
        value if value == CLASS_ONE_TO_MANY => Ok(CrosswalkClass::OneToMany),
        value if value == CLASS_LITERAL_ONLY => Ok(CrosswalkClass::LiteralOnly),
        value if value == CLASS_FALLBACK => Ok(CrosswalkClass::Fallback),
        _ => Err(ManifestError::Json(ERROR_UNKNOWN_CLASS.to_owned())),
    }
}

fn parse_projection_disposition(value: &str) -> Result<ProjectionDisposition, ManifestError> {
    match value {
        value if value == KIND_REGISTERED => Ok(ProjectionDisposition::Registered),
        value if value == CLASS_LITERAL_ONLY => Ok(ProjectionDisposition::LiteralOnly),
        value if value == KEY_FALLBACK => Ok(ProjectionDisposition::Fallback),
        _ => Err(ManifestError::Json(ERROR_UNKNOWN_DISPOSITION.to_owned())),
    }
}

fn parse_projection(value: &Value) -> Result<ProjectionValue, ManifestError> {
    let parser_ids = array_field(value, KEY_PARSER_IDS)?
        .iter()
        .map(|id| id.as_u64().and_then(|number| u16::try_from(number).ok()).ok_or_else(|| ManifestError::Json(ERROR_INTEGER.to_owned())))
        .collect::<Result<Vec<_>, ManifestError>>()?;
    let matcher_keys = array_field(value, KEY_MATCHER_KEYS)?.iter().map(string_value).collect::<Result<Vec<_>, ManifestError>>()?;
    let winner_refs = array_field(value, KEY_WINNER_REFS)?
        .iter()
        .map(|winner| {
            Ok(WinnerValue {
                matcher_key: string_field(winner, KEY_MATCHER_KEY)?,
                winner_ref: parse_reference(field(winner, KEY_WINNER_REF)?)?,
            })
        })
        .collect::<Result<Vec<_>, ManifestError>>()?;
    Ok(ProjectionValue {
        literal_name: string_field(value, KEY_LITERAL_NAME)?,
        classification: parse_class(&string_field(value, KEY_CLASSIFICATION)?)?,
        disposition: parse_projection_disposition(&string_field(value, KEY_DISPOSITION)?)?,
        parser_ids,
        matcher_keys,
        winner_refs,
    })
}

fn parse_collision(value: &Value) -> Result<CollisionValue, ManifestError> {
    let members = array_field(value, KEY_MEMBERS)?.iter().map(parse_reference).collect::<Result<Vec<_>, ManifestError>>()?;
    Ok(CollisionValue {
        kind: parse_matcher_kind(&string_field(value, KEY_KIND)?)?,
        normalized_key: string_field(value, KEY_NORMALIZED_KEY)?,
        members,
        winner_ref: parse_reference(field(value, KEY_WINNER_REF)?)?,
    })
}

fn parser_variants() -> HashSet<String> {
    let mut variants = HashSet::new();
    let mut inside = false;
    for line in PARSER_ENUM_SOURCE.lines() {
        if line.starts_with(ENUM_START) {
            inside = true;
            continue;
        }
        if inside && line == ENUM_END {
            break;
        }
        if inside {
            let candidate = line.trim().trim_end_matches(',');
            if !candidate.is_empty() && candidate.chars().all(|character| character == '_' || character.is_ascii_alphanumeric()) {
                variants.insert(candidate.to_owned());
            }
        }
    }
    variants
}

fn validate_reference(reference: &Reference, parser_ids: &HashSet<u16>, literal_only_names: &HashSet<String>, allow_fallback: bool) -> Result<(), ManifestError> {
    match reference {
        Reference::ParserId(parser_id) if parser_ids.contains(parser_id) => Ok(()),
        Reference::ParserId(parser_id) => Err(ManifestError::InvalidReference(number_message(MESSAGE_UNKNOWN_PARSER, *parser_id, EMPTY))),
        Reference::Supplemental(literal_name) if literal_only_names.contains(literal_name) => Ok(()),
        Reference::Supplemental(literal_name) => Err(ManifestError::InvalidReference(message(&[MESSAGE_UNKNOWN_SUPPLEMENTAL, literal_name]))),
        Reference::Fallback if allow_fallback => Ok(()),
        Reference::Fallback => Err(ManifestError::InvalidReference(ERROR_FALLBACK_REFERENCE.to_owned())),
    }
}

fn value_u64(value: &Value, key: &str) -> Result<usize, ManifestError> {
    field(value, key)?
        .as_u64()
        .and_then(|number| usize::try_from(number).ok())
        .ok_or_else(|| ManifestError::Json(ERROR_INTEGER.to_owned()))
}

fn validate_manifest(manifest: &ManifestWire) -> Result<(), ManifestError> {
    if manifest.schema_version != EXPECTED_SCHEMA_VERSION {
        return Err(ManifestError::SchemaVersion(manifest.schema_version));
    }
    if manifest.identities.len() != EXPECTED_IDENTITY_COUNT {
        return Err(ManifestError::IdentityCount {
            expected: EXPECTED_IDENTITY_COUNT,
            actual: manifest.identities.len(),
        });
    }

    let known_variants = parser_variants();
    let identities = manifest.identities.iter().map(parse_identity).collect::<Result<Vec<_>, ManifestError>>()?;
    let mut parser_ids = HashSet::with_capacity(EXPECTED_IDENTITY_COUNT);
    let mut parser_variants_seen = HashSet::with_capacity(EXPECTED_IDENTITY_COUNT);
    let mut canonical_names = HashSet::with_capacity(EXPECTED_IDENTITY_COUNT);
    let mut all_names = HashSet::with_capacity(EXPECTED_IDENTITY_COUNT);
    let mut identity_matchers = HashMap::<u16, HashSet<String>>::new();
    let mut structural_count = 0;
    for (offset, identity) in identities.iter().enumerate() {
        let position = offset + 1;
        if identity.id != position as u16 {
            return Err(ManifestError::NonSequentialId { position, actual: identity.id });
        }
        if !parser_ids.insert(identity.id) {
            return Err(ManifestError::DuplicateId(identity.id));
        }
        if !parser_variants_seen.insert(identity.parser_variant.clone()) {
            return Err(ManifestError::DuplicateParserVariant(identity.parser_variant.clone()));
        }
        if !known_variants.contains(&identity.parser_variant) {
            return Err(ManifestError::UnknownParserVariant(identity.parser_variant.clone()));
        }
        if !is_rust_identifier(&identity.parser_variant) {
            return Err(ManifestError::InvalidParserVariant(identity.parser_variant.clone()));
        }
        let canonical_key = fold(&identity.canonical_name);
        if !canonical_names.insert(canonical_key.clone()) || !all_names.insert(canonical_key) {
            return Err(ManifestError::DuplicateCanonicalName(identity.canonical_name.clone()));
        }
        for alias in &identity.aliases {
            if !all_names.insert(fold(alias)) {
                return Err(ManifestError::DuplicateAlias(alias.clone()));
            }
        }
        if identity.structural {
            structural_count += 1;
        }
        let mut matchers = HashSet::new();
        for matcher in &identity.matchers {
            if matcher.value.is_empty() {
                return Err(ManifestError::DuplicateMatcher(ERROR_EMPTY_MATCHER.to_owned()));
            }
            let key = matcher_key(matcher.kind, &matcher.value);
            if !matchers.insert(key.clone()) {
                return Err(ManifestError::DuplicateMatcher(key));
            }
        }
        identity_matchers.insert(identity.id, matchers);
    }
    if structural_count != EXPECTED_STRUCTURAL_COUNT {
        return Err(ManifestError::StructuralCount {
            expected: EXPECTED_STRUCTURAL_COUNT,
            actual: structural_count,
        });
    }
    if EXPECTED_IDENTITY_COUNT - structural_count != EXPECTED_NO_PARSE_COUNT {
        return Err(ManifestError::StructuralCount {
            expected: EXPECTED_NO_PARSE_COUNT,
            actual: EXPECTED_IDENTITY_COUNT - structural_count,
        });
    }

    if manifest.literal_projection.len() != EXPECTED_LITERAL_COUNT {
        return Err(ManifestError::LiteralProjectionCount {
            expected: EXPECTED_LITERAL_COUNT,
            actual: manifest.literal_projection.len(),
        });
    }
    let projections = manifest.literal_projection.iter().map(parse_projection).collect::<Result<Vec<_>, ManifestError>>()?;
    let mut literal_names = HashSet::with_capacity(EXPECTED_LITERAL_COUNT);
    let mut literal_only_names = HashSet::new();
    let mut projection_parser_ids = HashSet::new();
    let mut projection_members = HashMap::<String, HashSet<String>>::new();
    let mut category_counts = [0usize; 7];
    for row in &projections {
        if !literal_names.insert(row.literal_name.clone()) {
            return Err(ManifestError::DuplicateLiteralName(row.literal_name.clone()));
        }
        category_counts[0] += 1;
        match row.classification {
            CrosswalkClass::OneToOne => category_counts[2] += 1,
            CrosswalkClass::AliasCollision => category_counts[3] += 1,
            CrosswalkClass::OneToMany => category_counts[4] += 1,
            CrosswalkClass::LiteralOnly => category_counts[5] += 1,
            CrosswalkClass::Fallback => category_counts[6] += 1,
        }
        if !matches!(row.classification, CrosswalkClass::Fallback) {
            category_counts[1] += 1;
        }
        if row.parser_ids.iter().any(|id| !parser_ids.contains(id)) {
            return Err(ManifestError::InvalidReference(message(&[MESSAGE_ROW_PREFIX, &row.literal_name, MESSAGE_ROW_UNKNOWN_PARSER])));
        }
        if row.parser_ids.iter().collect::<HashSet<_>>().len() != row.parser_ids.len() {
            return Err(ManifestError::InvalidReference(message(&[MESSAGE_ROW_PREFIX, &row.literal_name, MESSAGE_ROW_REPEAT_PARSER])));
        }
        for id in &row.parser_ids {
            projection_parser_ids.insert(*id);
        }
        if matches!(row.classification, CrosswalkClass::LiteralOnly) {
            if !row.parser_ids.is_empty() {
                return Err(ManifestError::InvalidReference(message(&[MESSAGE_ROW_PREFIX, &row.literal_name, MESSAGE_ROW_LITERAL_ONLY_IDS])));
            }
            literal_only_names.insert(row.literal_name.clone());
        }
        if matches!(row.classification, CrosswalkClass::Fallback) && (row.literal_name != UNKNOWN_LITERAL_NAME || !row.parser_ids.is_empty()) {
            return Err(ManifestError::InvalidReference(ERROR_FALLBACK_ROW.to_owned()));
        }
        let mut row_keys = HashSet::new();
        for key in &row.matcher_keys {
            if !row_keys.insert(key.clone()) {
                return Err(ManifestError::DuplicateMatcher(key.clone()));
            }
            let (kind, value) = key.split_once(':').ok_or_else(|| ManifestError::DuplicateMatcher(key.clone()))?;
            let kind = parse_matcher_kind(kind)?;
            if matcher_key(kind, value) != *key {
                return Err(ManifestError::DuplicateMatcher(key.clone()));
            }
            let members = projection_members.entry(key.clone()).or_default();
            for id in &row.parser_ids {
                members.insert(format!("parser:{id}"));
                if !identity_matchers.get(id).is_some_and(|set| set.contains(key)) {
                    return Err(ManifestError::InvalidReference(message(&[
                        MESSAGE_ROW_PREFIX,
                        &row.literal_name,
                        MESSAGE_ROW_KEY_MISSING,
                        key,
                        MESSAGE_KEY_MISSING_IDENTITY,
                        &id.to_string(),
                    ])));
                }
            }
            if matches!(row.classification, CrosswalkClass::LiteralOnly) {
                members.insert(format!("supplemental:{}", row.literal_name));
            }
        }
        if row.winner_refs.len() != row.matcher_keys.len() {
            return Err(ManifestError::InvalidReference(message(&[MESSAGE_ROW_PREFIX, &row.literal_name, MESSAGE_ROW_WINNER_COUNT])));
        }
        for winner in &row.winner_refs {
            if !row.matcher_keys.contains(&winner.matcher_key) {
                return Err(ManifestError::InvalidReference(message(&[
                    MESSAGE_ROW_PREFIX,
                    &row.literal_name,
                    MESSAGE_ROW_WINNER_UNKNOWN,
                    &winner.matcher_key,
                ])));
            }
            let allow_fallback = matches!(row.classification, CrosswalkClass::Fallback);
            validate_reference(&winner.winner_ref, &parser_ids, &literal_only_names, allow_fallback)?;
            match &winner.winner_ref {
                Reference::ParserId(parser_id) if !row.parser_ids.contains(parser_id) && !matches!(row.classification, CrosswalkClass::Fallback) => {
                    return Err(ManifestError::InvalidReference(message(&[MESSAGE_ROW_PREFIX, &row.literal_name, MESSAGE_ROW_WINNER_MEMBER])));
                }
                Reference::Supplemental(literal_name) if !matches!(row.classification, CrosswalkClass::LiteralOnly) || literal_name != &row.literal_name => {
                    return Err(ManifestError::InvalidReference(message(&[MESSAGE_ROW_PREFIX, &row.literal_name, MESSAGE_ROW_SUPPLEMENTAL])));
                }
                Reference::Fallback if !matches!(row.classification, CrosswalkClass::Fallback) => {
                    return Err(ManifestError::InvalidReference(message(&[MESSAGE_ROW_PREFIX, &row.literal_name, MESSAGE_ROW_FALLBACK])));
                }
                _ => {}
            }
        }
    }
    let counts = [
        value_u64(&manifest.crosswalk_counts, KEY_TOTAL)?,
        value_u64(&manifest.crosswalk_counts, KEY_NAMED)?,
        value_u64(&manifest.crosswalk_counts, KEY_ONE_TO_ONE)?,
        value_u64(&manifest.crosswalk_counts, KEY_ALIAS_COLLISION)?,
        value_u64(&manifest.crosswalk_counts, KEY_ONE_TO_MANY)?,
        value_u64(&manifest.crosswalk_counts, KEY_LITERAL_ONLY)?,
        value_u64(&manifest.crosswalk_counts, KEY_FALLBACK)?,
    ];
    if category_counts != counts
        || category_counts[0] != EXPECTED_LITERAL_COUNT
        || category_counts[1] != EXPECTED_NAMED_LITERAL_COUNT
        || category_counts[2] != 51
        || category_counts[3] != 3
        || category_counts[4] != 8
        || category_counts[5] != 5
        || category_counts[6] != 1
    {
        return Err(ManifestError::CrosswalkCounts(format!("{category_counts:?}")));
    }

    if manifest.unmatched_parser_ids.len() != EXPECTED_NO_LITERAL_COUNT || manifest.unmatched_parser_ids.iter().collect::<HashSet<_>>().len() != EXPECTED_NO_LITERAL_COUNT {
        return Err(ManifestError::UnmatchedParserIds(ERROR_UNMATCHED_COUNT.to_owned()));
    }
    let unmatched: HashSet<u16> = manifest.unmatched_parser_ids.iter().copied().collect();
    for id in 1..=EXPECTED_IDENTITY_COUNT as u16 {
        let listed = unmatched.contains(&id);
        let projected = projection_parser_ids.contains(&id);
        if listed == projected {
            return Err(ManifestError::UnmatchedParserIds(number_message(MESSAGE_COMPLEMENT, id, MESSAGE_COMPLEMENT_SUFFIX)));
        }
        let identity = &identities[usize::from(id - 1)];
        if listed && !identity.matchers.is_empty() {
            return Err(ManifestError::UnmatchedParserIds(number_message(MESSAGE_UNMATCHED_MATCHERS, id, MESSAGE_HAS_MATCHERS)));
        }
        if !listed && identity.matchers.is_empty() {
            return Err(ManifestError::UnmatchedParserIds(number_message(MESSAGE_PROJECTED, id, MESSAGE_NO_MATCHERS)));
        }
    }

    let collisions = manifest.collision_resolutions.iter().map(parse_collision).collect::<Result<Vec<_>, ManifestError>>()?;
    let mut resolutions = HashMap::new();
    for resolution in &collisions {
        let key = matcher_key(resolution.kind, &resolution.normalized_key);
        if resolutions.insert(key.clone(), resolution).is_some() {
            return Err(ManifestError::CollisionResolution(message(&[MESSAGE_DUPLICATE_COLLISION, &key])));
        }
        if resolution.members.len() < 2 || resolution.members.iter().collect::<HashSet<_>>().len() != resolution.members.len() {
            return Err(ManifestError::CollisionResolution(key));
        }
        validate_reference(&resolution.winner_ref, &parser_ids, &literal_only_names, false)?;
        if !resolution.members.contains(&resolution.winner_ref) {
            return Err(ManifestError::CollisionResolution(message(&[MESSAGE_WINNER_NOT_MEMBER, &key])));
        }
        for member in &resolution.members {
            validate_reference(member, &parser_ids, &literal_only_names, false)?;
        }
    }
    for (key, members) in projection_members {
        if (members.len() > 1) != resolutions.contains_key(&key) {
            return Err(ManifestError::CollisionResolution(message(&[MESSAGE_COLLISION_MISMATCH, &key])));
        }
    }
    Ok(())
}

fn parse_manifest(source: &str) -> Result<ManifestWire, ManifestError> {
    let manifest: ManifestWire = serde_json::from_str(source).map_err(|error| ManifestError::Json(error.to_string()))?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

fn rust_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn render_matcher_kind(kind: MatcherKind) -> &'static str {
    match kind {
        MatcherKind::Extension => MATCHER_EXTENSION_RENDERED,
        MatcherKind::ExactBasename => MATCHER_EXACT_BASENAME_RENDERED,
        MatcherKind::CompoundSuffix => MATCHER_COMPOUND_SUFFIX_RENDERED,
    }
}

fn render_matcher_constructor(kind: MatcherKind) -> &'static str {
    match kind {
        MatcherKind::Extension => MATCHER_EXTENSION_PREFIX,
        MatcherKind::ExactBasename => MATCHER_EXACT_BASENAME_PREFIX,
        MatcherKind::CompoundSuffix => MATCHER_COMPOUND_SUFFIX_PREFIX,
    }
}

fn render_reference(reference: &Reference) -> String {
    match reference {
        Reference::ParserId(parser_id) => message(&[REFERENCE_PARSER_PREFIX, &parser_id.to_string(), REFERENCE_PARSER_SUFFIX]),
        Reference::Supplemental(literal_name) => message(&[REFERENCE_SUPPLEMENTAL_PREFIX, &rust_string(literal_name), REFERENCE_SUFFIX]),
        Reference::Fallback => REFERENCE_FALLBACK.to_owned(),
    }
}

fn render_list(values: Vec<String>) -> String {
    let mut result = String::from(LIST_PREFIX);
    result.push_str(&values.join(COMMA_SPACE));
    result.push_str(LIST_SUFFIX);
    result
}

fn render_str_slice(values: &[String]) -> String {
    render_list(values.iter().map(|value| rust_string(value)).collect())
}

fn render_parser_id_slice(values: &[u16]) -> String {
    render_list(values.iter().map(|value| message(&[PARSER_ID_PREFIX, &value.to_string(), PARSER_ID_SUFFIX])).collect())
}

fn render_matchers(values: &[MatcherValue]) -> String {
    render_list(
        values
            .iter()
            .map(|matcher| message(&[render_matcher_constructor(matcher.kind), &rust_string(&matcher.value), MATCHER_SUFFIX]))
            .collect(),
    )
}

fn render_winner_slice(values: &[WinnerValue]) -> String {
    render_list(
        values
            .iter()
            .map(|winner| message(&[WINNER_PREFIX, &rust_string(&winner.matcher_key), WINNER_SEPARATOR, &render_reference(&winner.winner_ref), WINNER_SUFFIX]))
            .collect(),
    )
}

fn render_reference_slice(values: &[Reference]) -> String {
    render_list(values.iter().map(render_reference).collect())
}

fn render_literal_disposition(disposition: LiteralDisposition, literal_name: Option<&str>) -> String {
    match disposition {
        LiteralDisposition::Registered => message(&[REGISTERED_PREFIX, &rust_string(literal_name.unwrap_or(EMPTY)), REGISTERED_SUFFIX]),
        LiteralDisposition::Unsupported => LITERAL_UNSUPPORTED_RENDERED.to_owned(),
        LiteralDisposition::NotApplicable => LITERAL_NOT_APPLICABLE_RENDERED.to_owned(),
    }
}

fn render_projection_disposition(disposition: ProjectionDisposition) -> &'static str {
    match disposition {
        ProjectionDisposition::Registered => LITERAL_REGISTERED,
        ProjectionDisposition::LiteralOnly => LITERAL_ONLY_RENDERED,
        ProjectionDisposition::Fallback => FALLBACK_RENDERED,
    }
}

fn render_structural(structural: bool) -> &'static str {
    if structural { STRUCTURAL_PARSE } else { STRUCTURAL_NO_PARSE }
}

fn render_record_line(identity: &IdentityValue) -> String {
    let mut line = String::from(INDENT);
    line.push_str(RECORD_PREFIX);
    line.push_str(&identity.id.to_string());
    line.push_str(RECORD_PARSER);
    line.push_str(&identity.parser_variant);
    line.push_str(RECORD_STRUCTURAL);
    line.push_str(render_structural(identity.structural));
    line.push_str(RECORD_CANONICAL);
    line.push_str(&rust_string(&identity.canonical_name));
    line.push_str(RECORD_ALIASES);
    line.push_str(&render_str_slice(&identity.aliases));
    line.push_str(RECORD_MATCHERS);
    line.push_str(&render_matchers(&identity.matchers));
    line.push_str(RECORD_LITERAL);
    line.push_str(&render_literal_disposition(identity.literal_disposition, identity.literal_name.as_deref()));
    line.push_str(RECORD_SUFFIX);
    line
}

fn render_projection_line(row: &ProjectionValue) -> String {
    let mut line = String::from(INDENT);
    line.push_str(LITERAL_PREFIX);
    line.push_str(&rust_string(&row.literal_name));
    line.push_str(LITERAL_DISPOSITION);
    line.push_str(render_projection_disposition(row.disposition));
    line.push_str(LITERAL_PARSER_IDS);
    line.push_str(&render_parser_id_slice(&row.parser_ids));
    line.push_str(LITERAL_MATCHER_KEYS);
    line.push_str(&render_str_slice(&row.matcher_keys));
    line.push_str(LITERAL_WINNERS);
    line.push_str(&render_winner_slice(&row.winner_refs));
    line.push_str(LITERAL_SUFFIX);
    line
}

fn render_collision_line(resolution: &CollisionValue) -> String {
    let mut line = String::from(INDENT);
    line.push_str(COLLISION_PREFIX);
    line.push_str(render_matcher_kind(resolution.kind));
    line.push_str(COLLISION_KEY);
    line.push_str(&rust_string(&resolution.normalized_key));
    line.push_str(COLLISION_MEMBERS);
    line.push_str(&render_reference_slice(&resolution.members));
    line.push_str(COLLISION_WINNER);
    line.push_str(&render_reference(&resolution.winner_ref));
    line.push_str(COLLISION_SUFFIX);
    line
}

fn render_registry(manifest: &ManifestWire) -> Result<String, ManifestError> {
    let identities = manifest.identities.iter().map(parse_identity).collect::<Result<Vec<_>, ManifestError>>()?;
    let projections = manifest.literal_projection.iter().map(parse_projection).collect::<Result<Vec<_>, ManifestError>>()?;
    let collisions = manifest.collision_resolutions.iter().map(parse_collision).collect::<Result<Vec<_>, ManifestError>>()?;
    let mut output = String::new();
    output.push_str(GENERATED_LANGUAGE_HEADER);
    for identity in &identities {
        output.push_str(&render_record_line(identity));
    }
    output.push_str(GENERATED_TABLE_END);
    output.push_str(GENERATED_LITERAL_HEADER);
    for row in &projections {
        output.push_str(&render_projection_line(row));
    }
    output.push_str(GENERATED_TABLE_END);
    output.push_str(GENERATED_COLLISION_HEADER);
    for resolution in &collisions {
        output.push_str(&render_collision_line(resolution));
    }
    output.push_str(GENERATED_FINAL_END);
    Ok(output)
}

fn is_rust_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic()) && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

const KEY_CLASSIFICATION: &str = "classification";
const KEY_DISPOSITION: &str = "disposition";
const KEY_NORMALIZED_KEY: &str = "normalizedKey";

/// Validate one raw reviewed manifest at the syntax boundary.
pub fn validate_source(source: &str) -> Result<(), ManifestError> {
    parse_manifest(source).map(|_| ())
}

/// Render one raw reviewed manifest into deterministic Rust source.
pub fn render_source(source: &str) -> Result<String, ManifestError> {
    let manifest = parse_manifest(source)?;
    render_registry(&manifest)
}
