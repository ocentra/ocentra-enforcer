//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Raw markdown parsing helpers for Plan validators.

#[cfg(test)]
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::plan_types::PlanCondition;

pub(crate) fn extract_line_value<'a>(lines: &[&'a str], prefix: &str) -> Option<(&'a str, usize)> {
    lines.iter().enumerate().find_map(|(index, line)| {
        line.trim()
            .strip_prefix(prefix)
            .map(|rest| (rest.trim(), index))
    })
}

pub(crate) fn workpack_id_condition(token: &str) -> PlanCondition {
    let token = token.trim().trim_matches('`');
    if !token.is_empty()
        && token
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
        && token
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic())
    {
        PlanCondition::Satisfied
    } else {
        PlanCondition::Unsatisfied
    }
}

pub(crate) fn section_text(
    lowercase_source: &str,
    start_heading: &str,
    end_heading: &str,
) -> Option<String> {
    let start = lowercase_source
        .find(start_heading)?
        .checked_add(start_heading.len())?;
    let rest = lowercase_source.get(start..)?;
    let end = rest.find(end_heading).unwrap_or(rest.len());
    rest.get(..end).map(str::to_owned)
}

pub(crate) fn one_based_line(index: usize) -> u32 {
    index
        .checked_add(1)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(u32::MAX)
}

#[cfg(test)]
pub(crate) fn fixture_path(root: &RepoRoot, relative: &RelPath) -> std::path::PathBuf {
    std::path::PathBuf::from(root.resolve(relative))
}
