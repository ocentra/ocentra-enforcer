//! CP08 projection snapshot invariants.
//!
//! BOUNDARY-INVARIANT: present and missing component kinds form a closed
//! partition with a typed status for every present kind.
//! NEGATIVE-TEST: duplicate, extra, and contradictory kinds are rejected.

use std::collections::{BTreeMap, BTreeSet};

use super::types::{ComponentKind, ComponentStatus};

fn expected_projection_kinds() -> [ComponentKind; 4] {
    [
        ComponentKind::NativePredicate,
        ComponentKind::ExternalEngine,
        ComponentKind::Advisory,
        ComponentKind::Manual,
    ]
}

pub(super) fn validate_projection_snapshot(
    component_count: usize,
    present_kinds: &[ComponentKind],
    missing_kinds: &[ComponentKind],
    kind_status: &BTreeMap<ComponentKind, ComponentStatus>,
) -> Result<(), String> {
    let expected = expected_projection_kinds();
    let expected_set = expected.into_iter().collect::<BTreeSet<_>>();
    let present_set = present_kinds.iter().copied().collect::<BTreeSet<_>>();
    let missing_set = missing_kinds.iter().copied().collect::<BTreeSet<_>>();
    super::ensure(
        present_set.len() == present_kinds.len() && missing_set.len() == missing_kinds.len(),
        "CP08 projection contains duplicate component kinds".to_owned(),
    )?;
    super::ensure(
        present_set.is_disjoint(&missing_set) && present_set.union(&missing_set).count() == 4,
        "CP08 projection present/missing kinds are not a partition".to_owned(),
    )?;
    super::ensure(
        present_set
            .union(&missing_set)
            .all(|kind| expected_set.contains(kind)),
        "CP08 projection contains an unsupported component kind".to_owned(),
    )?;
    super::ensure(
        component_count == present_set.len() && (3..=4).contains(&component_count),
        format!("CP08 projection component count does not match present kinds: {component_count}"),
    )?;
    super::ensure(
        kind_status.len() == present_set.len()
            && kind_status.keys().copied().collect::<BTreeSet<_>>() == present_set,
        "CP08 projection kind/status keys do not match present kinds".to_owned(),
    )?;
    for kind in present_kinds {
        let actual = kind_status
            .get(kind)
            .ok_or_else(|| format!("CP08 projection status missing for {kind:?}"))?;
        let expected_status = match kind {
            ComponentKind::NativePredicate => ComponentStatus::Proposed,
            ComponentKind::ExternalEngine => ComponentStatus::Blocked,
            ComponentKind::Advisory | ComponentKind::Manual => ComponentStatus::Retained,
        };
        super::ensure(
            *actual == expected_status,
            format!("CP08 projection status contradiction for {kind:?}: {actual:?}"),
        )?;
    }
    Ok(())
}
