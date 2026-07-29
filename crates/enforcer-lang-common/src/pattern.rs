//! The generic pattern-scanner engine (`generic-scanner` in the legacy
//! `.mjs` naming: `generic-common-scanner.mjs` / `generic-scanner-shared.mjs`
//! / `generic-scanners.mjs`). Per the workpack's "Generic-scanner
//! partition" note, this ENGINE is single-owned here (arc-09); the rule
//! ROWS that run on it are owned per-language by the `language` field
//! (`common` rows live in this crate's `families/*`, `ts`/`py` rows are
//! owned by `enforcer-lang-ts`/`enforcer-lang-py`, and the `SEC-2` rows that
//! also run on this engine are semantically owned by `enforcer-lang-security`
//! per the SEC-2 decision — they depend on this module for the engine only).
//!
//! A [`PatternValidator`] is a line/keyword-oriented detector: it fires when
//! any of its `fail_markers` literal substrings appear anywhere in the
//! source text, and is silent otherwise. This mirrors the dominant detection
//! shape in the ported `.mjs` sources (`source-policy-common*.mjs`,
//! `check-*.mjs`) — literal/keyword presence scanning over file text, not a
//! full parser — kept intentionally simple and auditable per rule.

use enforcer_domain::findings::{Finding, FindingTitle};
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::source_analysis::PatternMarkers;

/// A single common-family rule detector: fires when its source text
/// contains any of `fail_markers` (case-sensitive substring match).
///
/// `fail_markers` is intentionally a list (not one string) because several
/// ported rules trip on more than one literal spelling of the same
/// violation (e.g. both `.bat` and `.cmd` extensions for a Windows-only
/// script check).
#[derive(Debug)]
pub struct PatternValidator {
    rule_id: RuleId,
    title: FindingTitle,
    severity: Severity,
    fail_markers: PatternMarkers,
}

impl PatternValidator {
    /// Build a pattern validator for one rule id. `fail_markers` takes
    /// anything convertible to `Vec<&'static str>` (an array literal, a
    /// single-element slice, etc.) so family tables can register either a
    /// single marker or a small hand-picked set without heap-allocating a
    /// `'static` slice at a shared location.
    pub(crate) fn new(
        rule_id: RuleId,
        title: FindingTitle,
        severity: Severity,
        fail_markers: PatternMarkers,
    ) -> Self {
        Self {
            rule_id,
            title,
            severity,
            fail_markers,
        }
    }
}

impl Validator for PatternValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        // The common family modules are the executable rule catalog.  Each
        // module necessarily contains the marker that its own validator is
        // looking for, so applying that validator back to its defining row
        // would report the rule declaration as a product violation.  Skip
        // only the exact family file owned by this rule; cross-family checks
        // and every non-catalog source remain fully validated.
        if self_owned_family_definition(&input, &self.rule_id).is_some() {
            return Vec::new();
        }
        for (line_idx, line) in input.source.as_str().lines().enumerate() {
            for marker in self.fail_markers.iter() {
                if line.contains(marker) {
                    return crate::boundary::finding(
                        &self.rule_id,
                        self.severity,
                        (
                            self.title.as_str(),
                            format!("matched pattern `{marker}`"),
                            Some(line.trim()),
                        ),
                        input.file,
                        crate::boundary::line_number(line_idx),
                    )
                    .into_iter()
                    .collect();
                }
            }
        }
        Vec::new()
    }
}

fn self_owned_family_definition(input: &ValidationInput<'_>, rule_id: &RuleId) -> Option<()> {
    let stem = input
        .file
        .as_str()
        .strip_prefix("crates/enforcer-lang-common/src/families/")
        .and_then(|path| path.strip_suffix(".rs"))?;
    let (family, version) = rule_id.as_str().split_once('-')?;
    let (major, _minor) = version.split_once('.')?;
    let expected = format!("{}_{}", family.to_ascii_lowercase(), major);
    (stem == expected).then_some(())
}

#[cfg(test)]
mod tests {
    use super::PatternValidator;
    use crate::boundary::{source_analysis::PatternMarkers, static_finding_title, static_rule_id};
    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::severity::Severity;
    use enforcer_validator::validator::{ValidationInput, Validator};

    #[test]
    fn fires_on_any_marker_and_reports_line() -> Result<(), Box<dyn std::error::Error>> {
        let validator = PatternValidator::new(
            static_rule_id("TEST-9.1")?,
            static_finding_title("sample rule")?,
            Severity::Error,
            PatternMarkers::new(["FORBIDDEN_A", "FORBIDDEN_B"]),
        );
        let file = crate::boundary::static_rel_path("src/lib.rs")?;
        let source = "line one\nsecond FORBIDDEN_B here\nthird";
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].line.source_line().map(|line| line.to_string()),
            Some("2".to_owned())
        );
        Ok(())
    }

    #[test]
    fn silent_when_no_marker_present() -> Result<(), Box<dyn std::error::Error>> {
        let validator = PatternValidator::new(
            static_rule_id("TEST-9.2")?,
            static_finding_title("sample rule")?,
            Severity::Error,
            PatternMarkers::new(["FORBIDDEN"]),
        );
        let file = crate::boundary::static_rel_path("src/lib.rs")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                "clean source text",
            ),
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn self_owned_family_definition_marker_is_not_reported(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let marker = concat!("ENFORCER_", "AI_1_1_MARKER");
        let validator = PatternValidator::new(
            static_rule_id("AI-1.1")?,
            static_finding_title("sample rule")?,
            Severity::Error,
            PatternMarkers::new([marker]),
        );
        let file =
            crate::boundary::static_rel_path("crates/enforcer-lang-common/src/families/ai_1.rs")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(marker),
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn cross_family_and_non_catalog_markers_remain_reported(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let marker = concat!("ENFORCER_", "ARCH_1_1_MARKER");
        let validator = PatternValidator::new(
            static_rule_id("ARCH-1.1")?,
            static_finding_title("sample rule")?,
            Severity::Error,
            PatternMarkers::new([marker]),
        );
        let cross_family =
            crate::boundary::static_rel_path("crates/enforcer-lang-common/src/families/ai_1.rs")?;
        let regular_source = crate::boundary::static_rel_path("src/lib.rs")?;
        let cross_family_findings = validator.validate(ValidationInput {
            file: &cross_family,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(marker),
            scope: ScanScope::Files,
        });
        let regular_findings = validator.validate(ValidationInput {
            file: &regular_source,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(marker),
            scope: ScanScope::Files,
        });
        assert_eq!(cross_family_findings.len(), 1);
        assert_eq!(regular_findings.len(), 1);
        Ok(())
    }
}
