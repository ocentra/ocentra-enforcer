//! BOUNDARY-INVARIANT: CP10 mapping evidence may relate one recorded Trivy
//! output contract to an existing CP08 component, but it cannot promote that
//! relation into native implementation, executable proof, or a security result.
//!
//! NEGATIVE-TEST: malformed hashes, duplicate component identities, protected
//! source identities, wrong engine/version, and non-recorded coverage are
//! rejected by [`validate_mapping_manifest`].

use std::collections::BTreeSet;

use enforcer_core::error::Result;
use enforcer_domain::boundary::decode_error::DecodeError;
use serde::Deserialize;

const MAPPING_SCHEMA: &str = "cp10.trivy.mapping.v1";
const ENGINE_ID: &str = "trivy";
const ENGINE_VERSION: &str = "0.68.2";
const ENGINE_OUTPUT: &str = "Trivy config JSON SchemaVersion 2";
const PROTECTED_SKILL: &str = "detecting-fileless-malware-techniques";

/// Validate one CP10 mapping manifest and return its number of mapped skills.
///
/// The validator checks provenance, engine identity, component kind, coverage
/// level, output evidence, and uniqueness. It deliberately does not read a
/// vendor file or execute an external engine; the immutable CP08 artifact and
/// recorded Trivy output remain the evidence authorities.
pub fn validate_mapping_manifest(raw: &str) -> Result<usize> {
    let manifest: MappingManifest = serde_json::from_str(raw)
        .map_err(|error| DecodeError::new("cp10.trivy.mapping", error.to_string()))?;
    if manifest.schema != MAPPING_SCHEMA {
        return invalid("schema", "unsupported mapping schema");
    }
    if manifest.engine.id != ENGINE_ID {
        return invalid("engine.id", "mapping engine is not Trivy");
    }
    if manifest.engine.version != ENGINE_VERSION {
        return invalid("engine.version", "mapping engine version is not pinned");
    }
    if manifest.engine.output_format != ENGINE_OUTPUT {
        return invalid(
            "engine.outputFormat",
            "mapping output protocol is not reviewed",
        );
    }
    if manifest.mappings.is_empty() || manifest.mappings.len() > 10 {
        return invalid("mappings", "mapping count must be between one and ten");
    }

    let mut catalog_ids = BTreeSet::new();
    let mut component_ids = BTreeSet::new();
    for mapping in &manifest.mappings {
        validate_mapping(mapping)?;
        if !catalog_ids.insert(mapping.catalog_id.clone()) {
            return invalid("mappings.catalogId", "duplicate catalog identity");
        }
        if !component_ids.insert(mapping.component_id.clone()) {
            return invalid("mappings.componentId", "duplicate component identity");
        }
    }
    Ok(manifest.mappings.len())
}

fn validate_mapping(mapping: &ComponentMapping) -> Result<()> {
    if mapping.catalog_id.trim().is_empty() {
        return invalid("mappings.catalogId", "catalog identity is empty");
    }
    if mapping.catalog_id == PROTECTED_SKILL {
        return invalid("mappings.catalogId", "protected source is excluded");
    }
    let expected_component = format!("{}::external-engine", mapping.catalog_id);
    if mapping.component_id != expected_component {
        return invalid(
            "mappings.componentId",
            "mapping must target the component's external-engine identity",
        );
    }
    if mapping.component_kind != "external-engine" {
        return invalid(
            "mappings.componentKind",
            "component kind is not external-engine",
        );
    }
    if mapping.coverage != "recorded" {
        return invalid("mappings.coverage", "mapping coverage is not recorded");
    }
    validate_source(&mapping.source)?;
    validate_artifact(&mapping.cp08_artifact)?;
    if mapping.output.format != ENGINE_OUTPUT
        || mapping.output.target.trim().is_empty()
        || mapping.output.finding_ids.is_empty()
        || mapping.output.severity.trim().is_empty()
    {
        return invalid("mappings.output", "recorded output evidence is incomplete");
    }
    if mapping.not_proved.is_empty() {
        return invalid("mappings.notProved", "mapping must retain non-proofs");
    }
    Ok(())
}

fn validate_source(source: &SourceIdentity) -> Result<()> {
    if !source
        .path
        .starts_with("vendor/anthropic-cybersecurity-skills/skills/")
        || !source.path.ends_with("/SKILL.md")
        || source.path.contains(PROTECTED_SKILL)
        || source.license != "Apache-2.0"
    {
        return invalid(
            "mappings.source",
            "source identity is outside the approved role",
        );
    }
    validate_sha(&source.sha256, "mappings.source.sha256")?;
    validate_anchor(&source.anchor, "mappings.source.anchor")
}

fn validate_artifact(artifact: &ArtifactIdentity) -> Result<()> {
    if !artifact.path.starts_with("proof/cyberskills/cp08/batch-")
        || !artifact.path.ends_with("/decomposition.json")
    {
        return invalid(
            "mappings.cp08Artifact.path",
            "artifact is not an immutable CP08 packet",
        );
    }
    validate_sha(&artifact.sha256, "mappings.cp08Artifact.sha256")?;
    validate_anchor(&artifact.anchor, "mappings.cp08Artifact.anchor")
}

fn validate_sha(value: &str, field: &str) -> Result<()> {
    if value.len() != 64 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return invalid(field, "SHA-256 must be exactly 64 hexadecimal characters");
    }
    if value
        .chars()
        .any(|character| character.is_ascii_uppercase())
    {
        return invalid(field, "SHA-256 must use lowercase hexadecimal characters");
    }
    Ok(())
}

fn validate_anchor(value: &str, field: &str) -> Result<()> {
    let Some((heading, line)) = value.rsplit_once(":L") else {
        return invalid(field, "anchor must end with :L<positive line>");
    };
    if !heading.starts_with('#')
        || line.is_empty()
        || !line.chars().all(|character| character.is_ascii_digit())
        || line.parse::<u32>().ok().filter(|line| *line > 0).is_none()
    {
        return invalid(field, "anchor must use heading:Lpositive syntax");
    }
    Ok(())
}

fn invalid<T>(field: &str, detail: &str) -> Result<T> {
    Err(DecodeError::new(field, detail).into())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MappingManifest {
    schema: String,
    engine: EngineIdentity,
    mappings: Vec<ComponentMapping>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EngineIdentity {
    id: String,
    version: String,
    output_format: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComponentMapping {
    catalog_id: String,
    component_id: String,
    component_kind: String,
    coverage: String,
    source: SourceIdentity,
    cp08_artifact: ArtifactIdentity,
    output: OutputEvidence,
    not_proved: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceIdentity {
    path: String,
    sha256: String,
    anchor: String,
    license: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArtifactIdentity {
    path: String,
    sha256: String,
    anchor: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OutputEvidence {
    format: String,
    target: String,
    finding_ids: Vec<String>,
    severity: String,
}
