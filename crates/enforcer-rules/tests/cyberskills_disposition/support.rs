//! Focused CP00 integration tests.

use std::collections::BTreeSet;
use std::path::PathBuf;

use super::SOURCE_CATALOG;
use enforcer_domain::rules_types::{RuleCatalogJson, RuleCatalogSource};
use enforcer_rules::loader::{load_registry_from_records, parse_catalog};
use serde::Serialize;

pub(crate) fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

pub(crate) fn registry_ids(
    json: &str,
    source: &str,
) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let raw = RuleCatalogJson::try_from(json.to_owned())?;
    let source = RuleCatalogSource::try_from(source.to_owned())?;
    Ok(load_registry_from_records(parse_catalog(&raw, &source)?)?
        .iter()
        .map(|record| record.rule_id.as_str().to_owned())
        .collect())
}

pub(crate) fn source_catalog_ids() -> BTreeSet<String> {
    let mut section = "";
    let mut ids = BTreeSet::new();
    for line in SOURCE_CATALOG.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            section = heading.split_whitespace().next().unwrap_or("");
            continue;
        }
        if matches!(section, "T1" | "T2" | "ADAPTER") {
            if let Some(id) = line
                .strip_prefix("| ")
                .and_then(|row| row.split('|').next())
                .map(str::trim)
                .filter(|id| *id != "skill" && *id != "---" && !id.is_empty())
            {
                ids.insert(id.to_owned());
            }
        } else if section == "PROSE" {
            if let Some(id) = line.strip_prefix("- ") {
                ids.insert(id.trim().to_owned());
            }
        }
    }
    ids
}

pub(super) fn sorted_strings(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort();
    values
}

pub(super) fn enum_wire_value<T: Serialize>(value: &T) -> String {
    match serde_json::to_string(value) {
        Ok(serialized) => serialized.trim_matches('"').to_owned(),
        Err(error) => format!("serialization-error:{error}"),
    }
}
