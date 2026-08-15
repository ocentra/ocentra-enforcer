//! Source and artifact anchor validation for the CP00 boundary.
//!
//! BOUNDARY-INVARIANT: source content anchors preserve their native syntax;
//! artifact anchors independently require heading-and-line syntax.
//! NEGATIVE-TEST: empty, duplicate, and role-confused anchors are rejected.

use std::collections::BTreeSet;
use std::fmt;

use super::components::string_field;
use super::types::{
    DecompositionState, Sha256ValueEnvelope, SourceAnchorEnvelope, SourceAvailability,
};
use super::wire::manifest::CyberSkillDispositionRecordDto;
use super::{
    DerivedDispositionCounts, PROTECTED_CATALOG_ID, PROTECTED_SOURCE_PATH, PROTECTED_TRACKED_BLOB,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ValidatedText(String);

impl ValidatedText {
    pub(crate) fn new(value: String) -> Self {
        Self(value)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ValidatedText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(formatter)
    }
}

pub(super) fn validate_source_anchors<T: AsRef<str>>(anchors: &[T]) -> Result<(), String> {
    super::ensure(
        !anchors.is_empty(),
        "source anchors must not be empty".to_owned(),
    )?;
    let mut seen = BTreeSet::new();
    anchors.iter().try_for_each(|anchor| {
        let value = anchor.as_ref();
        super::ensure(
            value.trim() == value && seen.insert(value),
            "source anchors contain an empty, malformed, or duplicate anchor".to_owned(),
        )
    })
}

pub(super) fn validate_source_projection(
    record: &CyberSkillDispositionRecordDto,
) -> Result<(), String> {
    let source = record.source.as_ref().ok_or(format!(
        "v3 source projection missing: {}",
        record.catalog_id
    ))?;
    let checks = [
        super::ensure(
            source.path.as_str() == record.source_path,
            format!("v3 source path drift: {}", record.catalog_id),
        ),
        super::ensure(
            source.sha256.as_ref().map(Sha256ValueEnvelope::as_str)
                == record.source_sha256.as_deref(),
            format!("v3 source hash drift: {}", record.catalog_id),
        ),
        super::ensure(
            source.availability == record.source_availability,
            format!("v3 source availability drift: {}", record.catalog_id),
        ),
        super::ensure(
            source
                .anchors
                .iter()
                .map(SourceAnchorEnvelope::as_str)
                .eq(record.source_anchors.iter().map(String::as_str)),
            format!("v3 source anchors drift: {}", record.catalog_id),
        ),
    ];
    checks.into_iter().find_map(Result::err).map_or(Ok(()), Err)
}

pub(super) fn validate_unavailable(
    record: &CyberSkillDispositionRecordDto,
    counts: &mut DerivedDispositionCounts,
) -> Result<(), String> {
    counts.source_unavailable += 1;
    validate_unavailable_identity(record)
}

pub(super) fn validate_unavailable_identity(
    record: &CyberSkillDispositionRecordDto,
) -> Result<(), String> {
    super::ensure(
        [
            record.catalog_id == PROTECTED_CATALOG_ID,
            record.source_path == PROTECTED_SOURCE_PATH,
            record.decomposition_state == DecompositionState::Unavailable,
            record.source_sha256.is_none(),
            record.source_anchors.is_empty(),
            record.components.is_empty(),
        ]
        .into_iter()
        .all(std::convert::identity),
        format!(
            "sourceUnavailable row is not the protected empty identity: {}",
            record.catalog_id
        ),
    )?;
    let unavailable = record
        .unavailable_source
        .as_ref()
        .ok_or(format!("unavailableSource missing: {}", record.catalog_id))?;
    let tracked = string_field(unavailable, "trackedBlob")? == PROTECTED_TRACKED_BLOB;
    let fields_present = [
        string_field(unavailable, "observation").is_ok(),
        string_field(unavailable, "ownerDecisionRef").is_ok(),
    ]
    .into_iter()
    .all(std::convert::identity);
    super::ensure(
        tracked && fields_present,
        format!("protected tracked blob drift: {}", record.catalog_id),
    )
}

pub(super) fn validate_artifact_anchors<S: AsRef<str>, A: AsRef<str>>(
    source_anchors: &[S],
    artifact_anchors: &[A],
) -> Result<(), String> {
    let source_values = source_anchors.iter().map(AsRef::as_ref).collect::<Vec<_>>();
    let artifact_values = artifact_anchors
        .iter()
        .map(AsRef::as_ref)
        .collect::<Vec<_>>();
    super::ensure(
        !artifact_values.is_empty() && artifact_values != source_values,
        "CP08 artifact anchors must be nonempty and distinct from source anchors".to_owned(),
    )
}

pub(super) fn validate_source_state(
    record: &CyberSkillDispositionRecordDto,
    counts: &mut DerivedDispositionCounts,
) -> Result<(), String> {
    if record.source_availability == SourceAvailability::Available {
        validate_available(record, counts)
    } else {
        validate_unavailable(record, counts)
    }
}

fn validate_available(
    record: &CyberSkillDispositionRecordDto,
    counts: &mut DerivedDispositionCounts,
) -> Result<(), String> {
    counts.readable_sources += 1;
    let sha = record
        .source_sha256
        .as_deref()
        .ok_or_else(|| format!("sourceSha256 missing: {}", record.catalog_id))?;
    super::ensure(
        sha.len() == 64
            && sha
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
        format!("sourceSha256 must be lowercase hex: {}", record.catalog_id),
    )?;
    super::ensure(
        !record.source_anchors.is_empty()
            && record
                .source_anchors
                .iter()
                .all(|anchor| !anchor.trim().is_empty()),
        format!("sourceAnchors missing: {}", record.catalog_id),
    )?;
    super::ensure(
        record.unavailable_source.is_none(),
        format!(
            "available row {} cannot carry unavailableSource",
            record.catalog_id
        ),
    )?;
    match record.decomposition_state {
        DecompositionState::Unreviewed if !record.components.is_empty() => Err(format!(
            "unreviewed row {} must have empty components",
            record.catalog_id
        )),
        DecompositionState::Unreviewed => {
            counts.unexplained_rows += 1;
            Ok(())
        }
        DecompositionState::Reviewed if record.components.is_empty() => Err(format!(
            "reviewed row {} must have components",
            record.catalog_id
        )),
        DecompositionState::Reviewed => {
            counts.reviewed_rows += 1;
            counts.decomposed_rows += 1;
            Ok(())
        }
        DecompositionState::Unavailable => Err(format!(
            "available row {} cannot be decompositionState unavailable",
            record.catalog_id
        )),
    }
}
