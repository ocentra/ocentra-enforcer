//! Native, deterministic Cargo SBOM generation.
//!
//! The legacy implementation copied the raw `cargo metadata` response into an
//! output directory. This module keeps Cargo as the resolver of record, but
//! produces an Enforcer-owned, stable document from the locked resolution and
//! the exact `Cargo.lock` bytes. There is deliberately no timestamp, host
//! path, or process-specific identifier in the rendered artifact.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Stable filename written beneath the caller-provided output directory.
pub const SBOM_FILE_NAME: &str = "cargo-sbom.json";

/// The typed, Enforcer-owned SBOM wire format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CargoSbomDto {
    pub bom_format: String,
    pub spec_version: String,
    pub serial_number: String,
    pub metadata: CargoSbomMetadataDto,
    pub components: Vec<CargoSbomComponentDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CargoSbomMetadataDto {
    pub lockfile_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CargoSbomComponentDto {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    name: String,
    version: String,
    source: Option<String>,
    license: Option<String>,
}

/// Create a deterministic SBOM from Cargo's locked metadata response and the
/// bytes of the corresponding lockfile.
pub fn build_from_metadata(metadata_json: &str, lockfile: &[u8]) -> Result<CargoSbomDto, String> {
    let metadata: CargoMetadata = serde_json::from_str(metadata_json)
        .map_err(|error| format!("invalid cargo metadata: {error}"))?;
    if metadata.packages.is_empty() {
        return Err("cargo metadata contains no resolved packages".to_owned());
    }
    let lockfile_sha256 = sha256_hex(lockfile);
    let mut components = metadata
        .packages
        .into_iter()
        .map(|package| CargoSbomComponentDto {
            name: package.name,
            version: package.version,
            source: package.source,
            license: package.license,
        })
        .collect::<Vec<_>>();
    components.sort();
    components.dedup();
    let document = CargoSbomDto {
        bom_format: "Ocentra-Cargo-SBOM".to_owned(),
        spec_version: "1.0".to_owned(),
        serial_number: format!("urn:ocentra:cargo-lock-sha256:{lockfile_sha256}"),
        metadata: CargoSbomMetadataDto { lockfile_sha256 },
        components,
    };
    validate(&document)?;
    Ok(document)
}

/// Validate an SBOM decoded from an untrusted artifact boundary.
///
/// This prevents a truncated or hand-tampered file from being represented as a
/// successfully generated SBOM.
pub fn validate(document: &CargoSbomDto) -> Result<(), String> {
    if document.bom_format != "Ocentra-Cargo-SBOM" || document.spec_version != "1.0" {
        return Err("unsupported Cargo SBOM schema".to_owned());
    }
    if !is_sha256(&document.metadata.lockfile_sha256) {
        return Err("SBOM lockfile SHA-256 is invalid".to_owned());
    }
    if document.serial_number
        != format!(
            "urn:ocentra:cargo-lock-sha256:{}",
            document.metadata.lockfile_sha256
        )
    {
        return Err("SBOM serial number does not bind the lockfile digest".to_owned());
    }
    if document.components.is_empty() {
        return Err("SBOM contains no components".to_owned());
    }
    if document.components.windows(2).any(|pair| {
        pair.first()
            .zip(pair.get(1))
            .is_some_and(|(left, right)| left >= right)
    }) {
        return Err("SBOM components are not strictly deterministic".to_owned());
    }
    if document
        .components
        .iter()
        .any(|component| component.name.is_empty() || component.version.is_empty())
    {
        return Err("SBOM component name or version is empty".to_owned());
    }
    Ok(())
}

/// Generate from the current Cargo workspace and write a validated document.
pub fn generate_current_workspace(root: &Path, output_dir: &Path) -> Result<PathBuf, String> {
    let lockfile_path = root.join("Cargo.lock");
    let lockfile = std::fs::read(&lockfile_path)
        .map_err(|error| format!("cannot read {}: {error}", lockfile_path.display()))?;
    let output = std::process::Command::new("cargo")
        .current_dir(root)
        .args(["metadata", "--format-version=1", "--locked"])
        .output()
        .map_err(|error| format!("cannot execute cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata --locked failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let metadata_json = std::str::from_utf8(&output.stdout)
        .map_err(|error| format!("cargo metadata emitted non-UTF-8 output: {error}"))?;
    let document = build_from_metadata(metadata_json, &lockfile)?;
    let encoded = serde_json::to_vec_pretty(&document)
        .map_err(|error| format!("cannot encode Cargo SBOM: {error}"))?;
    let decoded: CargoSbomDto = serde_json::from_slice(&encoded)
        .map_err(|error| format!("cannot decode generated SBOM: {error}"))?;
    validate(&decoded)?;
    std::fs::create_dir_all(output_dir)
        .map_err(|error| format!("cannot create {}: {error}", output_dir.display()))?;
    let artifact = output_dir.join(SBOM_FILE_NAME);
    std::fs::write(&artifact, encoded)
        .map_err(|error| format!("cannot write {}: {error}", artifact.display()))?;
    Ok(artifact)
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::{build_from_metadata, validate, CargoSbomDto};

    const METADATA: &str = r#"{
        "packages": [
            {"name":"zeta","version":"2.0.0","source":"registry+https://example.invalid","license":"MIT"},
            {"name":"alpha","version":"1.0.0","source":null,"license":null}
        ]
    }"#;

    #[test]
    fn deterministic_cargo_metadata_build_is_schema_valid() -> Result<(), String> {
        let first = build_from_metadata(METADATA, b"locked dependency graph")?;
        let second = build_from_metadata(METADATA, b"locked dependency graph")?;
        assert_eq!(first, second);
        assert_eq!(first.components[0].name, "alpha");
        validate(&first)
    }

    #[test]
    fn cargo_sbom_dto_round_trip_preserves_external_schema() -> Result<(), String> {
        let original: CargoSbomDto = build_from_metadata(METADATA, b"locked dependency graph")?;
        let encoded = serde_json::to_string(&original).map_err(|error| error.to_string())?;
        let decoded: CargoSbomDto =
            serde_json::from_str(&encoded).map_err(|error| error.to_string())?;
        assert_eq!(decoded, original);
        Ok(())
    }

    #[test]
    fn tampered_lock_binding_is_rejected() -> Result<(), String> {
        let original = build_from_metadata(METADATA, b"locked dependency graph")?;
        let mut tampered: CargoSbomDto =
            serde_json::from_str(&serde_json::to_string(&original).map_err(|e| e.to_string())?)
                .map_err(|error| error.to_string())?;
        tampered.metadata.lockfile_sha256 = "0".repeat(64);
        assert!(validate(&tampered).is_err());
        Ok(())
    }
}
