//! Component vocabulary and projection-shape validation for CP00.
//!
//! BOUNDARY-INVARIANT: component status is constrained by its closed kind
//! matrix before counts are accumulated.
//! NEGATIVE-TEST: unsupported, retained-mechanical, and missing-evidence
//! components are rejected.

use std::collections::BTreeSet;

use serde_json::Value;

use super::types::{ComponentKind, ComponentStatus, DecompositionState};
use super::wire::manifest::{CyberSkillComponentDto, CyberSkillDispositionRecordDto};
use super::{require, DerivedDispositionCounts};

pub(super) fn object_field<'a>(object: &'a Value, field: &str) -> Result<&'a Value, String> {
    object
        .as_object()
        .and_then(|fields| fields.get(field))
        .ok_or_else(|| format!("boundary object field missing: {field}"))
}

pub(super) fn string_field<'a>(object: &'a Value, field: &str) -> Result<&'a str, String> {
    object_field(object, field)?
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("boundary object field must be a non-empty string: {field}"))
}

pub(super) fn validate_component(
    component: &CyberSkillComponentDto,
    record: &CyberSkillDispositionRecordDto,
    counts: &mut DerivedDispositionCounts,
    component_ids: &mut BTreeSet<String>,
    known_evidence_kind: fn(&str) -> bool,
) -> Result<(), String> {
    require(component_ids.insert(component.component_id.clone()), || {
        format!("duplicate componentId: {}", component.component_id)
    })?;
    require(
        !component.not_proved.is_empty()
            && component
                .not_proved
                .iter()
                .all(|item| !item.trim().is_empty()),
        || format!("notProved missing for {}", component.component_id),
    )?;
    require(
        component.status != ComponentStatus::Blocked
            || record.decomposition_state == DecompositionState::Reviewed,
        || {
            format!(
                "blocked component {} is not a reviewed component",
                component.component_id
            )
        },
    )?;
    validate_component_kind(component)?;
    validate_evidence(component, known_evidence_kind)?;
    apply_status_count(component, counts)
}

fn validate_evidence(
    component: &CyberSkillComponentDto,
    known_evidence_kind: fn(&str) -> bool,
) -> Result<(), String> {
    component.evidence_refs.iter().try_for_each(|evidence| {
        let kind = string_field(evidence, "kind")?;
        require(known_evidence_kind(kind), || {
            format!(
                "unknown evidence kind for {}: {kind}",
                component.component_id
            )
        })?;
        string_field(evidence, "path")?;
        Ok(())
    })
}

fn apply_status_count(
    component: &CyberSkillComponentDto,
    counts: &mut DerivedDispositionCounts,
) -> Result<(), String> {
    let retained = component.status == ComponentStatus::Retained;
    let advisory = component.kind == ComponentKind::Advisory;
    let manual = component.kind == ComponentKind::Manual;
    require(!retained || advisory || manual, || {
        format!(
            "mechanical component {} cannot be retained",
            component.component_id
        )
    })?;
    counts.implemented_components += usize::from(matches!(
        component.status,
        ComponentStatus::Implemented | ComponentStatus::Proved
    ));
    counts.proved_components += usize::from(component.status == ComponentStatus::Proved);
    if retained && advisory {
        counts.advisory_retained += 1;
    }
    if retained && manual {
        counts.manual_retained += 1;
    }
    Ok(())
}

fn validate_component_kind(component: &CyberSkillComponentDto) -> Result<(), String> {
    match component.kind {
        ComponentKind::NativePredicate | ComponentKind::ExternalEngine => {
            validate_mechanical(component)
        }
        ComponentKind::Advisory | ComponentKind::Manual => validate_retained(component),
    }
}

fn validate_mechanical(component: &CyberSkillComponentDto) -> Result<(), String> {
    require(
        component
            .predicate
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
        || {
            format!(
                "mechanical predicate missing for {}",
                component.component_id
            )
        },
    )?;
    require(
        component.implementation_ref.is_some() || component.status == ComponentStatus::Blocked,
        || format!("implementationRef missing for {}", component.component_id),
    )?;
    component
        .implementation_ref
        .as_ref()
        .filter(|_| component.status != ComponentStatus::Blocked)
        .map(|implementation| {
            string_field(implementation, "executorRuleId")?;
            string_field(implementation, "validatorPath")
        })
        .transpose()?;
    Ok(())
}

fn validate_retained(component: &CyberSkillComponentDto) -> Result<(), String> {
    require(
        component.status != ComponentStatus::Retained
            || component
                .purpose
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty()),
        || format!("retained purpose missing for {}", component.component_id),
    )?;
    Ok(())
}
