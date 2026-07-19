//! Conversion boundary from common-rule observations to canonical findings.
//!
//! BOUNDARY-INVARIANT: raw validator observations are converted here into
//! typed rule identifiers, finding details, and source-line telemetry before
//! they cross into the common-language validator surface.
//! boundaryOwnerNote: enforcer-lang-common owns the common validator boundary;
//! changes to its raw-boundary surface require this crate's scoped proof.

pub(crate) mod source_analysis;

use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingSnippet, FindingTitle,
};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;
use enforcer_validator::validator::Validator;
use std::num::NonZeroU32;

#[cfg(test)]
use enforcer_domain::paths::RepoRoot;
#[cfg(test)]
use std::{error::Error, path::Path};

pub(crate) fn line_number(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1)
}

pub(crate) fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

pub(crate) fn nonnegative_count(value: i64) -> u32 {
    u32::try_from(value.max(0)).unwrap_or(u32::MAX)
}

pub(crate) fn no_snippet() -> Option<&'static str> {
    None
}

pub(crate) fn static_rule_id(
    value: &'static str,
) -> Result<RuleId, enforcer_domain::boundary::decode_error::DecodeError> {
    value.parse()
}

#[cfg(test)]
pub(crate) fn static_finding_title(
    value: &'static str,
) -> Result<FindingTitle, enforcer_domain::boundary::decode_error::DecodeError> {
    value.parse()
}

#[cfg(test)]
pub(crate) fn static_rel_path(
    value: &'static str,
) -> Result<RelPath, enforcer_domain::boundary::decode_error::DecodeError> {
    value.parse()
}

pub(crate) fn register_pattern(
    out: &mut Vec<Box<dyn Validator>>,
    rule_id: RuleId,
    title: FindingTitle,
    severity: Severity,
    marker: &'static str,
) {
    out.push(Box::new(crate::pattern::PatternValidator::new(
        rule_id,
        title,
        severity,
        source_analysis::PatternMarkers::new([marker]),
    )));
}

pub(crate) fn finding(
    rule_id: &RuleId,
    severity: Severity,
    text: (impl Into<String>, impl Into<String>, Option<&str>),
    file: &RelPath,
    line: u32,
) -> Option<Finding> {
    let (title, detail, snippet) = text;
    let line = FindingLine::known(SourceLine::try_new(NonZeroU32::new(line)?));
    Some(Finding {
        rule_id: rule_id.clone(),
        severity,
        title: FindingTitle::new(title.into()).ok()?,
        detail: FindingDetail::new(detail.into()).ok()?,
        file: file.clone(),
        line,
        snippet: snippet
            .map(|value| FindingSnippet::new(value.to_owned()))
            .transpose()
            .ok()?,
    })
}

#[cfg(test)]
pub(crate) fn run_fixture_parity(
    validator: &dyn Validator,
    manifest_dir: &Path,
    fail_fixture: &str,
    pass_fixture: &str,
) -> Result<(), Box<dyn Error>> {
    let repo_root = RepoRoot::try_from(manifest_dir)?;
    let fail_fixture = RelPath::try_from(fail_fixture.to_owned())?;
    let pass_fixture = RelPath::try_from(pass_fixture.to_owned())?;
    enforcer_validator::harness::run_fixture_parity(
        validator,
        &repo_root,
        &fail_fixture,
        &pass_fixture,
    )?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn manifest_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[cfg(test)]
pub(crate) fn check_ownerset_text(
    base: &str,
    head: &str,
    path: &RelPath,
    rule_id: &RuleId,
) -> Vec<Finding> {
    crate::rules::change_discipline::check_ownerset(
        enforcer_domain::boundary::validation::ValidationSource::from_text(base),
        enforcer_domain::boundary::validation::ValidationSource::from_text(head),
        path,
        rule_id,
    )
}

#[cfg(test)]
mod tests {
    use super::{finding, static_rel_path, static_rule_id};
    use enforcer_domain::severity::Severity;

    /// Negative invalid-input coverage for the finding conversion boundary.
    #[test]
    fn finding_rejects_invalid_empty_title_at_the_boundary(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file = static_rel_path("src/lib.rs")?;
        let rule_id = static_rule_id("BOUNDARY-TEST.1")?;

        let finding = finding(
            &rule_id,
            Severity::Error,
            ("", "valid detail", Some("source")),
            &file,
            1,
        );

        assert!(finding.is_none());
        Ok(())
    }
}
