//! Typed CyberSkills disposition ledger boundary.
//!
//! This module deliberately models source identity and decomposition state
//! separately.  An available but unreviewed source is not given a guessed
//! component kind, and the protected source-unavailable identity is never
//! treated as reviewed content.
//!
//! BOUNDARY-INVARIANT: raw CyberSkills JSON is decoded here, then validated
//! into the closed disposition vocabulary before any derived count is used.
//! NEGATIVE-TEST: `tests/cyberskills_disposition.rs` rejects malformed,
//! duplicate, unavailable-source, and incomplete component cases.
//! ROUNDTRIP-TEST: `tests/cyberskills_disposition.rs::manifest_round_trips_without_count_drift`
//! serializes and reparses the complete typed contract before validation.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTECTED_CATALOG_ID: &str = "detecting-fileless-malware-techniques";
pub const PROTECTED_SOURCE_PATH: &str =
    "vendor/anthropic-cybersecurity-skills/skills/detecting-fileless-malware-techniques/SKILL.md";
pub const PROTECTED_TRACKED_BLOB: &str = "df48fa4149dd25956e730443d3582693a3f825a8";

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
/// Decoded CP00 CyberSkills disposition manifest.
pub struct CyberSkillsDispositionManifestDto {
    pub schema_version: u32,
    pub source_catalog: String,
    pub source_vendor_root: String,
    pub mapping_policy: String,
    pub records: Vec<CyberSkillDispositionRecordDto>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
/// Decoded per-skill identity and decomposition record.
pub struct CyberSkillDispositionRecordDto {
    pub catalog_id: String,
    pub source_path: String,
    pub source_availability: SourceAvailability,
    pub decomposition_state: DecompositionState,
    // DEFAULT-JUSTIFICATION: absent source hash/anchors are permitted only for sourceUnavailable rows and rejected by validation.
    #[serde(default)]
    pub source_sha256: Option<String>,
    // DEFAULT-JUSTIFICATION: absent source hash/anchors are permitted only for sourceUnavailable rows and rejected by validation.
    #[serde(default)]
    pub source_anchors: Vec<String>,
    pub attribution: Value,
    // DEFAULT-JUSTIFICATION: legacy v1 fields remain optional during typed migration; record validation requires disposition/rationale.
    #[serde(default)]
    pub legacy_disposition: Option<LegacyDisposition>,
    // DEFAULT-JUSTIFICATION: legacy v1 fields remain optional during typed migration; record validation requires disposition/rationale.
    #[serde(default)]
    pub legacy_rationale: Option<String>,
    // DEFAULT-JUSTIFICATION: legacy v1 fields remain optional during typed migration; record validation requires disposition/rationale.
    #[serde(default)]
    pub legacy: Option<Value>,
    // DEFAULT-JUSTIFICATION: only sourceUnavailable rows may omit this object; validation enforces the protected identity contract.
    #[serde(default)]
    pub unavailable_source: Option<Value>,
    // DEFAULT-JUSTIFICATION: unreviewed rows intentionally carry no guessed components.
    #[serde(default)]
    pub components: Vec<CyberSkillComponentDto>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
/// Decoded independently evidenced disposition component.
pub struct CyberSkillComponentDto {
    pub component_id: String,
    pub kind: ComponentKind,
    pub tier: ComponentTier,
    pub status: ComponentStatus,
    pub coverage_kind: CoverageKind,
    // DEFAULT-JUSTIFICATION: component predicate is required for mechanical kinds by validation and absent for retained guidance.
    #[serde(default)]
    pub predicate: Option<String>,
    // DEFAULT-JUSTIFICATION: retained advisory/manual components require purpose by validation.
    #[serde(default)]
    pub purpose: Option<String>,
    // DEFAULT-JUSTIFICATION: blocked components may defer implementation until a named dependency is resolved.
    #[serde(default)]
    pub implementation_ref: Option<Value>,
    // DEFAULT-JUSTIFICATION: evidence is accumulated as each component proof closes.
    #[serde(default)]
    pub evidence_refs: Vec<Value>,
    pub not_proved: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
// SERDE-TAG-JUSTIFICATION: this closed scalar vocabulary intentionally uses its stable string representation.
/// Source presence state for a catalog identity.
pub enum SourceAvailability {
    Available,
    SourceUnavailable,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
// SERDE-TAG-JUSTIFICATION: this closed scalar vocabulary intentionally uses its stable string representation.
/// Review state for a catalog identity.
pub enum DecompositionState {
    Unreviewed,
    Reviewed,
    Unavailable,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
// SERDE-TAG-JUSTIFICATION: this closed scalar vocabulary intentionally uses its stable string representation.
/// Legacy triage label retained for migration.
pub enum LegacyDisposition {
    Native,
    Unported,
    AdapterDeferred,
    AdvisoryProse,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
// SERDE-TAG-JUSTIFICATION: this closed scalar vocabulary intentionally uses its stable string representation.
/// Closed decomposition component vocabulary.
pub enum ComponentKind {
    NativePredicate,
    ExternalEngine,
    Advisory,
    Manual,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
// SERDE-TAG-JUSTIFICATION: this closed scalar vocabulary intentionally uses its stable string representation.
/// Catalog planning tier associated with a component.
pub enum ComponentTier {
    #[serde(rename = "T1")]
    T1,
    #[serde(rename = "T2")]
    T2,
    #[serde(rename = "T3")]
    T3,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
// SERDE-TAG-JUSTIFICATION: this closed scalar vocabulary intentionally uses its stable string representation.
/// Evidence lifecycle state for a component.
pub enum ComponentStatus {
    Proposed,
    Implemented,
    Proved,
    Retained,
    Blocked,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
// SERDE-TAG-JUSTIFICATION: this closed scalar vocabulary intentionally uses its stable string representation.
/// Scope of what a component actually proves.
pub enum CoverageKind {
    NarrowedPredicate,
    Component,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
// SERDE-TAG-JUSTIFICATION: this closed scalar vocabulary intentionally uses its stable string representation.
/// Legacy conversion estimate retained for compatibility.
pub enum ConversionDifficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
// SERDE-TAG-JUSTIFICATION: this closed scalar vocabulary intentionally uses its stable string representation.
/// Closed evidence artifact vocabulary.
pub enum EvidenceKind {
    SourceAttribution,
    Validator,
    FailFixture,
    PassFixture,
    MalformedFixture,
    BoundaryFixture,
    Cli,
    Mcp,
    Ci,
    AdapterRecorded,
    AdapterLive,
    ManualRetention,
}

/// Decode the CP00 manifest JSON at the boundary.
pub fn parse_manifest(raw: &str) -> Result<CyberSkillsDispositionManifestDto, serde_json::Error> {
    serde_json::from_str(raw)
}

#[derive(Debug, Default, PartialEq, Eq)]
/// Counts derived from validated rows and components.
pub struct DerivedDispositionCounts {
    pub identity_rows: usize,
    pub readable_sources: usize,
    pub source_unavailable: usize,
    pub reviewed_rows: usize,
    pub decomposed_rows: usize,
    pub implemented_components: usize,
    pub proved_components: usize,
    pub advisory_retained: usize,
    pub manual_retained: usize,
    pub unexplained_rows: usize,
}

fn object_field<'a>(object: &'a Value, field: &str) -> Result<&'a Value, String> {
    object
        .as_object()
        .and_then(|fields| fields.get(field))
        .ok_or_else(|| format!("boundary object field missing: {field}"))
}

fn string_field<'a>(object: &'a Value, field: &str) -> Result<&'a str, String> {
    object_field(object, field)?
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("boundary object field must be a non-empty string: {field}"))
}

fn known_evidence_kind(value: &str) -> bool {
    matches!(
        value,
        "source-attribution"
            | "validator"
            | "fail-fixture"
            | "pass-fixture"
            | "malformed-fixture"
            | "boundary-fixture"
            | "cli"
            | "mcp"
            | "ci"
            | "adapter-recorded"
            | "adapter-live"
            | "manual-retention"
    )
}

/// Validate the semantic CP00 contract after serde has checked the closed
/// vocabularies and unknown-field boundary.
/// Validate identity, source, decomposition, and evidence invariants.
pub fn validate_manifest(
    manifest: &CyberSkillsDispositionManifestDto,
) -> Result<DerivedDispositionCounts, String> {
    if manifest.schema_version != 2 {
        return Err(format!(
            "unsupported CyberSkills disposition schema version: {}",
            manifest.schema_version
        ));
    }
    if manifest.records.len() != 817 {
        return Err(format!(
            "CyberSkills disposition must retain exactly 817 identities, got {}",
            manifest.records.len()
        ));
    }

    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut component_ids = BTreeSet::new();
    let mut counts = DerivedDispositionCounts {
        identity_rows: manifest.records.len(),
        ..DerivedDispositionCounts::default()
    };

    for record in &manifest.records {
        let expected_path = format!(
            "vendor/anthropic-cybersecurity-skills/skills/{}/SKILL.md",
            record.catalog_id
        );
        if record.source_path != expected_path {
            return Err(format!(
                "non-canonical sourcePath for {}: {}",
                record.catalog_id, record.source_path
            ));
        }
        if !ids.insert(record.catalog_id.clone()) {
            return Err(format!("duplicate catalogId: {}", record.catalog_id));
        }
        if !paths.insert(record.source_path.clone()) {
            return Err(format!("duplicate sourcePath: {}", record.source_path));
        }
        if string_field(&record.attribution, "sourceCatalog").is_err()
            || string_field(&record.attribution, "vendor").is_err()
            || string_field(&record.attribution, "sourcePath")? != record.source_path
        {
            return Err(format!("attribution path drift for {}", record.catalog_id));
        }
        if record.legacy_disposition.is_none() {
            return Err(format!(
                "legacyDisposition missing for {}",
                record.catalog_id
            ));
        }
        if record
            .legacy_rationale
            .as_deref()
            .is_none_or(|reason| reason.trim().is_empty())
        {
            return Err(format!("legacyRationale missing for {}", record.catalog_id));
        }

        match record.source_availability {
            SourceAvailability::Available => {
                counts.readable_sources += 1;
                let sha = record
                    .source_sha256
                    .as_deref()
                    .ok_or_else(|| format!("sourceSha256 missing for {}", record.catalog_id))?;
                if sha.len() != 64
                    || !sha
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
                {
                    return Err(format!(
                        "sourceSha256 must be lowercase hex for {}",
                        record.catalog_id
                    ));
                }
                if record.source_anchors.is_empty()
                    || record
                        .source_anchors
                        .iter()
                        .any(|anchor| anchor.trim().is_empty())
                {
                    return Err(format!("sourceAnchors missing for {}", record.catalog_id));
                }
                if record.unavailable_source.is_some() {
                    return Err(format!(
                        "available row {} cannot carry unavailableSource",
                        record.catalog_id
                    ));
                }
                match record.decomposition_state {
                    DecompositionState::Unreviewed => {
                        if !record.components.is_empty() {
                            return Err(format!(
                                "unreviewed row {} must have empty components",
                                record.catalog_id
                            ));
                        }
                        counts.unexplained_rows += 1;
                    }
                    DecompositionState::Reviewed => {
                        if record.components.is_empty() {
                            return Err(format!(
                                "reviewed row {} must have components",
                                record.catalog_id
                            ));
                        }
                        counts.reviewed_rows += 1;
                        counts.decomposed_rows += 1;
                    }
                    DecompositionState::Unavailable => {
                        return Err(format!(
                            "available row {} cannot be decompositionState unavailable",
                            record.catalog_id
                        ));
                    }
                }
            }
            SourceAvailability::SourceUnavailable => {
                counts.source_unavailable += 1;
                if record.catalog_id != PROTECTED_CATALOG_ID
                    || record.source_path != PROTECTED_SOURCE_PATH
                    || record.decomposition_state != DecompositionState::Unavailable
                    || record.source_sha256.is_some()
                    || !record.source_anchors.is_empty()
                    || !record.components.is_empty()
                {
                    return Err(format!(
                        "sourceUnavailable row is not the protected empty identity: {}",
                        record.catalog_id
                    ));
                }
                let unavailable = record.unavailable_source.as_ref().ok_or_else(|| {
                    format!("unavailableSource missing for {}", record.catalog_id)
                })?;
                if string_field(unavailable, "trackedBlob")? != PROTECTED_TRACKED_BLOB
                    || string_field(unavailable, "observation").is_err()
                    || string_field(unavailable, "ownerDecisionRef").is_err()
                {
                    return Err(format!(
                        "protected tracked blob drift for {}",
                        record.catalog_id
                    ));
                }
            }
        }

        for component in &record.components {
            if !component_ids.insert(component.component_id.clone()) {
                return Err(format!("duplicate componentId: {}", component.component_id));
            }
            if component.not_proved.is_empty()
                || component
                    .not_proved
                    .iter()
                    .any(|item| item.trim().is_empty())
            {
                return Err(format!("notProved missing for {}", component.component_id));
            }
            if component.status == ComponentStatus::Blocked
                && record.decomposition_state != DecompositionState::Reviewed
            {
                return Err(format!(
                    "blocked component {} is not a reviewed component",
                    component.component_id
                ));
            }
            match component.kind {
                ComponentKind::NativePredicate | ComponentKind::ExternalEngine => {
                    if component
                        .predicate
                        .as_deref()
                        .is_none_or(|value| value.trim().is_empty())
                    {
                        return Err(format!(
                            "mechanical predicate missing for {}",
                            component.component_id
                        ));
                    }
                    if component.implementation_ref.is_none()
                        && component.status != ComponentStatus::Blocked
                    {
                        return Err(format!(
                            "implementationRef missing for {}",
                            component.component_id
                        ));
                    }
                    if component.status != ComponentStatus::Blocked {
                        let implementation =
                            component.implementation_ref.as_ref().ok_or_else(|| {
                                format!("implementationRef missing for {}", component.component_id)
                            })?;
                        string_field(implementation, "executorRuleId")?;
                        string_field(implementation, "validatorPath")?;
                    }
                }
                ComponentKind::Advisory | ComponentKind::Manual => {
                    if component.status == ComponentStatus::Retained
                        && component
                            .purpose
                            .as_deref()
                            .is_none_or(|value| value.trim().is_empty())
                    {
                        return Err(format!(
                            "retained purpose missing for {}",
                            component.component_id
                        ));
                    }
                }
            }
            for evidence in &component.evidence_refs {
                let kind = string_field(evidence, "kind")?;
                if !known_evidence_kind(kind) {
                    return Err(format!(
                        "unknown evidence kind for {}: {kind}",
                        component.component_id
                    ));
                }
                string_field(evidence, "path")?;
            }
            match component.status {
                ComponentStatus::Implemented => counts.implemented_components += 1,
                ComponentStatus::Proved => {
                    counts.implemented_components += 1;
                    counts.proved_components += 1;
                }
                ComponentStatus::Retained => match component.kind {
                    ComponentKind::Advisory => counts.advisory_retained += 1,
                    ComponentKind::Manual => counts.manual_retained += 1,
                    ComponentKind::NativePredicate | ComponentKind::ExternalEngine => {
                        return Err(format!(
                            "mechanical component {} cannot be retained",
                            component.component_id
                        ));
                    }
                },
                ComponentStatus::Proposed | ComponentStatus::Blocked => {}
            }
        }
    }

    if counts.source_unavailable != 1 {
        return Err(format!(
            "exactly one sourceUnavailable identity is required, got {}",
            counts.source_unavailable
        ));
    }
    Ok(counts)
}
