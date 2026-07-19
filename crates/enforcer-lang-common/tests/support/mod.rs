use enforcer_domain::paths::RelPath;
use enforcer_domain::rules_types::{RuleCatalogJson, RuleCatalogSource};
use enforcer_rules::registry::RuleRecord;

pub(crate) fn parse_catalog(
    raw: &str,
    source: &str,
) -> Result<Vec<RuleRecord>, Box<dyn std::error::Error>> {
    let raw = RuleCatalogJson::try_from(String::from(raw))?;
    let source = RuleCatalogSource::try_from(String::from(source))?;
    Ok(enforcer_rules::loader::parse_catalog(&raw, &source)?)
}

pub(crate) fn rel_path(value: &str) -> Result<RelPath, Box<dyn std::error::Error>> {
    Ok(RelPath::try_from(String::from(value))?)
}
