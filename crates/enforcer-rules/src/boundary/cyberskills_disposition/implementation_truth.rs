//! Private implementation/proof truth validation.
//!
//! BOUNDARY-INVARIANT: implementation and executable-proof coverage remain
//! independent from CP08 decomposition evidence.
//! NEGATIVE-TEST: contradictory coverage projections are rejected by the
//! CP00 negative fixture suite.

use super::projection::validate_projection_snapshot;
use super::require;
use super::types::{
    ComponentIdEnvelope, ComponentKind, ComponentStatus, CoverageLevel, ProjectionStatus,
};
use super::wire::cp08::Cp08ProjectionDto;
use super::wire::manifest::CyberSkillDispositionRecordDto;
use super::DerivedDispositionCounts;

pub(super) fn validate_implementation(
    record: &CyberSkillDispositionRecordDto,
) -> Result<(), String> {
    let implementation = record
        .implementation
        .as_ref()
        .ok_or_else(|| format!("v3 implementation truth missing for {}", record.catalog_id))?;
    let native_ids = record
        .components
        .iter()
        .filter(|component| {
            component.kind == ComponentKind::NativePredicate
                && matches!(
                    component.status,
                    ComponentStatus::Implemented | ComponentStatus::Proved
                )
        })
        .map(|component| component.component_id.clone())
        .collect::<Vec<_>>();
    let expected_native = if native_ids.is_empty() {
        CoverageLevel::None
    } else {
        CoverageLevel::Complete
    };
    require(
        implementation.native.coverage == expected_native
            && implementation
                .native
                .component_ids
                .iter()
                .map(ComponentIdEnvelope::as_str)
                .eq(native_ids.iter().map(String::as_str)),
        || {
            format!(
                "native implementation projection drift for {}",
                record.catalog_id
            )
        },
    )?;
    let native_total = record
        .components
        .iter()
        .filter(|component| component.kind == ComponentKind::NativePredicate)
        .count();
    let proved_total = record
        .components
        .iter()
        .filter(|component| {
            component.kind == ComponentKind::NativePredicate
                && component.status == ComponentStatus::Proved
        })
        .count();
    let expected_proof = match proved_total {
        0 => CoverageLevel::None,
        proved if proved < native_total => CoverageLevel::Partial,
        _ => CoverageLevel::Complete,
    };
    require(
        implementation.executable_proof.coverage == expected_proof,
        || {
            format!(
                "executable proof projection drift for {}",
                record.catalog_id
            )
        },
    )
}

pub(super) fn validate_available_cp08(
    record: &CyberSkillDispositionRecordDto,
    counts: &mut DerivedDispositionCounts,
) -> Result<(), String> {
    let source = record
        .source
        .as_ref()
        .ok_or_else(|| format!("source projection missing for {}", record.catalog_id))?;
    super::source::validate_source_anchors(source.anchors.as_slice())?;
    super::require(source.license.as_str() == "Apache-2.0", || {
        format!("available source license drift for {}", record.catalog_id)
    })?;
    let projection = record
        .cp08_projection
        .as_ref()
        .ok_or_else(|| format!("v3 CP08 projection missing for {}", record.catalog_id))?;
    validate_projection_state(record, projection)?;
    super::cp08_chain::validate_chain(record, projection)?;
    counts.cp08_component_count += projection.component_count;
    counts.cp08_missing_native += usize::from(
        projection
            .missing_kinds
            .contains(&ComponentKind::NativePredicate),
    );
    counts.cp08_missing_external += usize::from(
        projection
            .missing_kinds
            .contains(&ComponentKind::ExternalEngine),
    );
    counts.cp08_complete_rows += usize::from(projection.status == ProjectionStatus::Complete);
    counts.cp08_partial_rows += usize::from(projection.status == ProjectionStatus::Partial);
    Ok(())
}

fn validate_projection_state(
    record: &CyberSkillDispositionRecordDto,
    projection: &Cp08ProjectionDto,
) -> Result<(), String> {
    super::require(projection.status != ProjectionStatus::Absent, || {
        format!(
            "available row {} has absent CP08 projection",
            record.catalog_id
        )
    })?;
    validate_projection_snapshot(
        projection.component_count,
        &projection.present_kinds,
        &projection.missing_kinds,
        &projection.kind_status,
    )?;
    let expected = [ProjectionStatus::Partial, ProjectionStatus::Complete]
        [usize::from(projection.missing_kinds.is_empty())];
    super::require(projection.status == expected, || {
        format!("CP08 projection status drift for {}", record.catalog_id)
    })
}
