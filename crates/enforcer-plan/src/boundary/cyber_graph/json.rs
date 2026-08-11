//! BOUNDARY-INVARIANT: JSON helpers validate field shape before graph state is
//! derived and keep source availability separate from implementation coverage.
//! NEGATIVE-TEST: missing, wrong-typed, unsafe, and invalid-hash fields are
//! rejected or surfaced as conservative graph findings.
use super::{
    is_safe_relative_path, CoverageLevel, GraphEdge, GraphError, GraphIssue, IssueLevel, NodeId,
    PROTECTED_SKILL,
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn string_field(value: &Value, path: &[&str]) -> Option<String> {
    path.iter()
        .try_fold(value, |current, segment| current.get(*segment))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

pub(crate) fn usize_field(value: &Value, path: &[&str]) -> Option<usize> {
    path.iter()
        .try_fold(value, |current, segment| current.get(*segment))
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

pub(crate) fn required_string_array(
    value: &Value,
    path: &[&str],
    owner: &str,
) -> Result<Vec<String>, GraphError> {
    let array = array_field(value, path).ok_or_else(|| {
        GraphError::InvalidValue(format!("intent family `{owner}` has no skillIds array"))
    })?;
    let values: Vec<String> = array
        .iter()
        .map(|value| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                GraphError::InvalidValue(format!(
                    "intent family `{owner}` contains a non-string skill ID"
                ))
            })
        })
        .collect::<Result<_, _>>()?;
    if values.is_empty() {
        return Err(GraphError::InvalidValue(format!(
            "intent family `{owner}` has no skills"
        )));
    }
    Ok(values)
}

pub(crate) fn validate_intent_matrix_header(matrix: &Value) -> Result<(), GraphError> {
    let schema = usize_field(matrix, &["schemaVersion"]);
    let skills = usize_field(matrix, &["skillCount"]);
    let families = usize_field(matrix, &["familyCount"]);
    if schema != Some(1) || skills != Some(816) || families != Some(34) {
        return Err(GraphError::InvalidValue(
            "intent matrix header must declare schema 1, 34 families, and 816 skills".to_owned(),
        ));
    }
    let protected = string_array(matrix, &["generatedFrom", "protectedExcluded"]);
    if protected != [PROTECTED_SKILL.to_owned()] {
        return Err(GraphError::InvalidValue(
            "intent matrix protected exclusion is not exact".to_owned(),
        ));
    }
    Ok(())
}

pub(crate) fn repository_graph_skill_ids(matrix: &Value) -> Result<BTreeSet<String>, GraphError> {
    let qualification = matrix.get("routeQualification").ok_or_else(|| {
        GraphError::InvalidValue("intent matrix route qualification is missing".to_owned())
    })?;
    if string_field(qualification, &["defaultNativeRoute"]).as_deref() != Some("CP09") {
        return Err(GraphError::InvalidValue(
            "intent matrix route qualification must default native predicates to CP09".to_owned(),
        ));
    }
    let values = array_field(qualification, &["repositoryGraphSkillIds"]).ok_or_else(|| {
        GraphError::InvalidValue(
            "intent matrix repository graph qualification must list skill IDs".to_owned(),
        )
    })?;
    let mut ids = BTreeSet::new();
    for value in values {
        let skill_id = value.as_str().ok_or_else(|| {
            GraphError::InvalidValue(
                "intent matrix repository graph qualification contains a non-string ID".to_owned(),
            )
        })?;
        if skill_id.is_empty() || skill_id == PROTECTED_SKILL || !ids.insert(skill_id.to_owned()) {
            return Err(GraphError::InvalidValue(format!(
                "intent matrix repository graph qualification contains invalid or duplicate skill `{skill_id}`"
            )));
        }
    }
    Ok(ids)
}

pub(crate) fn coverage_field(value: &Value, path: &[&str]) -> CoverageLevel {
    let level = string_field(value, path);
    if level.as_deref() == Some("complete") {
        CoverageLevel::Complete
    } else if level.as_deref() == Some("partial") {
        CoverageLevel::Partial
    } else {
        CoverageLevel::None
    }
}

pub(crate) fn coverage_name(level: CoverageLevel) -> String {
    if level == CoverageLevel::Complete {
        "complete"
    } else if level == CoverageLevel::Partial {
        "partial"
    } else {
        "none"
    }
    .to_owned()
}

pub(crate) fn array_field<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Vec<Value>> {
    path.iter()
        .try_fold(value, |current, segment| current.get(*segment))
        .and_then(Value::as_array)
}

pub(crate) fn string_array(value: &Value, path: &[&str]) -> Vec<String> {
    array_field(value, path)
        .map(|array| {
            array
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn missing_endpoint(edge: &GraphEdge, endpoint: &NodeId) -> GraphIssue {
    GraphIssue {
        level: IssueLevel::Error,
        code: "GRAPH-MISSING-NODE".to_owned(),
        node: Some(edge.from.clone()),
        message: format!("edge {:?} references missing node `{endpoint}`", edge.kind),
    }
}

pub(crate) fn partition_issue(code: &str, message: String) -> GraphIssue {
    GraphIssue {
        level: IssueLevel::Error,
        code: code.to_owned(),
        node: None,
        message,
    }
}

pub(crate) fn packet_issue(code: &str, path: &Path) -> GraphIssue {
    GraphIssue {
        level: IssueLevel::Error,
        code: code.to_owned(),
        node: None,
        message: format!(
            "CP11 retention packet `{}` violates its boundary",
            path.display()
        ),
    }
}

pub(crate) fn valid_cp11_skill(root: &Path, skill: &Value, ids: &mut BTreeSet<String>) -> bool {
    let Some(catalog_id) = string_field(skill, &["catalogId"]) else {
        return false;
    };
    let source_path = string_field(skill, &["source", "path"]);
    let source_hash = string_field(skill, &["source", "sha256"]);
    let artifact_path = string_field(skill, &["cp08Evidence", "artifactPath"]);
    let artifact_hash = string_field(skill, &["cp08Evidence", "artifactSha256"]);
    let source_ok = source_path.is_some_and(|path| !path.trim().is_empty())
        && source_hash.is_some_and(|hash| valid_sha256(hash.as_str()))
        && string_field(skill, &["source", "license"]).as_deref() == Some("Apache-2.0")
        && array_field(skill, &["source", "anchors"]).is_some_and(|anchors| !anchors.is_empty());
    let artifact_ok = artifact_path
        .is_some_and(|path| is_safe_relative_path(path.as_str()) && root.join(path).is_file())
        && artifact_hash.is_some_and(|hash| valid_sha256(hash.as_str()));
    ids.insert(catalog_id.clone())
        && catalog_id != PROTECTED_SKILL
        && source_ok
        && artifact_ok
        && valid_retention_component(skill.get("advisory"))
        && valid_retention_component(skill.get("manual"))
}

pub(crate) fn valid_retention_component(value: Option<&Value>) -> bool {
    value
        .and_then(|component| string_field(component, &["status"]))
        .is_some_and(|status| status == "retained")
        && value
            .and_then(|component| string_field(component, &["purpose"]))
            .is_some_and(|purpose| !purpose.trim().is_empty())
        && value
            .and_then(|component| array_field(component, &["notProved"]))
            .is_some_and(|not_proved| !not_proved.is_empty())
}

pub(crate) fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value == value.to_ascii_lowercase()
        && value.chars().all(|character| character.is_ascii_hexdigit())
}

pub(crate) fn count_coverage(value: Option<&String>, complete: &mut usize, partial: &mut usize) {
    if value.map(String::as_str) == Some("complete") {
        *complete += 1;
    } else if value.map(String::as_str) == Some("partial") {
        *partial += 1;
    }
}
