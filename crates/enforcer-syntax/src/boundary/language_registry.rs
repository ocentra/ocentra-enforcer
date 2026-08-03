//! Deterministic language-registry generation boundary.
//!
//! BOUNDARY-INVARIANT: this module decodes only the closed reviewed manifest
//! and renders deterministic registry records; it never parses source files.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};

/// Canonical parser identity denominator.
pub const EXPECTED_IDENTITY_COUNT: usize = 160;
const EXPECTED_SCHEMA_VERSION: u16 = 1;
const EXPECTED_NON_STRUCTURAL_COUNT: usize = 4;

/// Closed representation of the reviewed language registry manifest.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct ManifestDto {
    /// Manifest schema version.
    #[serde(rename = "schemaVersion")]
    pub schema_version: u16,
    /// Stable parser identity rows.
    pub identities: Vec<ManifestIdentityDto>,
}

/// One closed manifest identity row.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct ManifestIdentityDto {
    /// One-based stable identity.
    pub id: u16,
    /// Existing `Language` enum variant name.
    #[serde(rename = "parserVariant")]
    pub parser_variant: String,
    /// Whether parser dispatch produces a structural result.
    pub structural: bool,
}

/// A deterministic manifest validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// JSON did not match the closed manifest shape.
    Json(String),
    /// The manifest schema version is unsupported.
    SchemaVersion(u16),
    /// The manifest does not contain the canonical number of identities.
    IdentityCount { expected: usize, actual: usize },
    /// An identity is not at its required one-based position.
    NonSequentialId { position: usize, actual: u16 },
    /// Two rows use the same identity.
    DuplicateId(u16),
    /// Two rows use the same parser variant.
    DuplicateParserVariant(String),
    /// The parser variant cannot be emitted as a Rust identifier.
    InvalidParserVariant(String),
    /// The structural denominator drifted.
    NonStructuralCount { expected: usize, actual: usize },
    /// Cargo did not provide the generator output directory.
    MissingOutputDirectory,
    /// The deterministic generated file could not be written.
    Io(String),
}

impl Display for ManifestError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "language manifest JSON is invalid: {error}"),
            Self::SchemaVersion(version) => write!(
                formatter,
                "language manifest schema version {version} is unsupported"
            ),
            Self::IdentityCount { expected, actual } => write!(
                formatter,
                "language manifest has {actual} identities; expected {expected}"
            ),
            Self::NonSequentialId { position, actual } => write!(
                formatter,
                "language manifest row {position} has id {actual}; expected {position}"
            ),
            Self::DuplicateId(id) => write!(formatter, "language manifest duplicates id {id}"),
            Self::DuplicateParserVariant(variant) => write!(
                formatter,
                "language manifest duplicates parser variant {variant}"
            ),
            Self::InvalidParserVariant(variant) => write!(
                formatter,
                "language manifest parser variant {variant} is not a Rust identifier"
            ),
            Self::NonStructuralCount { expected, actual } => write!(
                formatter,
                "language manifest has {actual} non-structural identities; expected {expected}"
            ),
            Self::MissingOutputDirectory => write!(formatter, "Cargo did not provide OUT_DIR"),
            Self::Io(error) => write!(formatter, "language registry generation failed: {error}"),
        }
    }
}

impl std::error::Error for ManifestError {}

/// Parse and validate the manifest's closed shape and stable denominators.
pub(crate) fn parse_manifest(source: &str) -> Result<ManifestDto, ManifestError> {
    let manifest: ManifestDto =
        serde_json::from_str(source).map_err(|error| ManifestError::Json(error.to_string()))?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

/// Validate one already-decoded manifest.
pub(crate) fn validate_manifest(manifest: &ManifestDto) -> Result<(), ManifestError> {
    if manifest.schema_version != EXPECTED_SCHEMA_VERSION {
        return Err(ManifestError::SchemaVersion(manifest.schema_version));
    }
    if manifest.identities.len() != EXPECTED_IDENTITY_COUNT {
        return Err(ManifestError::IdentityCount {
            expected: EXPECTED_IDENTITY_COUNT,
            actual: manifest.identities.len(),
        });
    }
    let mut ids = HashSet::with_capacity(manifest.identities.len());
    let mut variants = HashSet::with_capacity(manifest.identities.len());
    let mut non_structural_count = 0;
    for (offset, identity) in manifest.identities.iter().enumerate() {
        let position = offset + 1;
        let expected_id = u16::try_from(position).map_err(|_| ManifestError::NonSequentialId {
            position,
            actual: identity.id,
        })?;
        if !ids.insert(identity.id) {
            return Err(ManifestError::DuplicateId(identity.id));
        }
        if identity.id != expected_id {
            return Err(ManifestError::NonSequentialId {
                position,
                actual: identity.id,
            });
        }
        if !variants.insert(identity.parser_variant.as_str()) {
            return Err(ManifestError::DuplicateParserVariant(
                identity.parser_variant.clone(),
            ));
        }
        if !is_rust_identifier(&identity.parser_variant) {
            return Err(ManifestError::InvalidParserVariant(
                identity.parser_variant.clone(),
            ));
        }
        if !identity.structural {
            non_structural_count += 1;
        }
    }
    if non_structural_count != EXPECTED_NON_STRUCTURAL_COUNT {
        return Err(ManifestError::NonStructuralCount {
            expected: EXPECTED_NON_STRUCTURAL_COUNT,
            actual: non_structural_count,
        });
    }
    Ok(())
}

/// Render the deterministic generated Rust registry projection.
pub(crate) fn render_registry(manifest: &ManifestDto) -> String {
    let mut output =
        String::from("// Source owner: crates/enforcer-syntax/registry/languages.json.\n\n");
    output.push_str("pub static LANGUAGE_RECORDS: &[LanguageRecord] = &[\n");
    for identity in &manifest.identities {
        let structural = if identity.structural {
            "StructuralLanguageSupport::ParseFile"
        } else {
            "StructuralLanguageSupport::NoParseFile"
        };
        output.push_str(&format!(
            "    LanguageRecord {{ id: LanguageId::from_registry_index({}), parser: Language::{}, structural: {} }},\n",
            identity.id, identity.parser_variant, structural
        ));
    }
    output.push_str("];\n");
    output
}

/// Validate one raw reviewed manifest at the syntax boundary.
pub fn validate_source(source: &str) -> Result<(), ManifestError> {
    parse_manifest(source).map(|_| ())
}

/// Render one raw reviewed manifest into deterministic registry source.
pub fn render_source(source: &str) -> Result<String, ManifestError> {
    parse_manifest(source).map(|manifest| render_registry(&manifest))
}

fn is_rust_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::ManifestIdentityDto;
    use serde::{de::DeserializeOwned, Serialize};

    const MANIFEST: &str = include_str!("../../registry/languages.json");

    fn round_trip<T>(value: &T) -> Result<T, serde_json::Error>
    where
        T: Serialize + DeserializeOwned,
    {
        let encoded = serde_json::to_string(value)?;
        serde_json::from_str(&encoded)
    }

    #[test]
    fn manifest_dto_round_trip_json() -> Result<(), Box<dyn std::error::Error>> {
        let manifest: super::ManifestDto = serde_json::from_str(MANIFEST)?;
        let decoded: super::ManifestDto = round_trip::<super::ManifestDto>(&manifest)?;
        assert_eq!(decoded, manifest);
        let identity = ManifestIdentityDto {
            id: 1,
            parser_variant: "Rust".to_owned(),
            structural: true,
        };
        let identity_decoded: ManifestIdentityDto = round_trip::<ManifestIdentityDto>(&identity)?;
        assert_eq!(identity_decoded, identity);
        Ok(())
    }
}
