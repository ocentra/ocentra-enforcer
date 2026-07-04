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

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// A single common-family rule detector: fires when its source text
/// contains any of `fail_markers` (case-sensitive substring match).
///
/// `fail_markers` is intentionally a list (not one string) because several
/// ported rules trip on more than one literal spelling of the same
/// violation (e.g. both `.bat` and `.cmd` extensions for a Windows-only
/// script check).
pub struct PatternValidator {
    rule_id: RuleId,
    title: &'static str,
    severity: Severity,
    fail_markers: Vec<&'static str>,
}

impl PatternValidator {
    /// Build a pattern validator for one rule id. `fail_markers` takes
    /// anything convertible to `Vec<&'static str>` (an array literal, a
    /// single-element slice, etc.) so family tables can register either a
    /// single marker or a small hand-picked set without heap-allocating a
    /// `'static` slice at a shared location.
    pub fn new(
        rule_id: RuleId,
        title: &'static str,
        severity: Severity,
        fail_markers: impl Into<Vec<&'static str>>,
    ) -> Self {
        Self {
            rule_id,
            title,
            severity,
            fail_markers: fail_markers.into(),
        }
    }
}

impl Validator for PatternValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        for (line_idx, line) in input.source.lines().enumerate() {
            for marker in &self.fail_markers {
                if line.contains(marker) {
                    return vec![Finding {
                        rule_id: self.rule_id.clone(),
                        severity: self.severity,
                        title: self.title.to_owned(),
                        detail: format!("matched pattern `{marker}`"),
                        file: input.file.clone(),
                        line: (line_idx as u32).saturating_add(1),
                        snippet: Some(line.trim().to_owned()),
                    }];
                }
            }
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::PatternValidator;
    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::severity::Severity;
    use enforcer_validator::validator::{ValidationInput, Validator};

    #[test]
    fn fires_on_any_marker_and_reports_line() -> Result<(), Box<dyn std::error::Error>> {
        let validator = PatternValidator::new(
            "TEST-9.1".parse()?,
            "sample rule",
            Severity::Error,
            ["FORBIDDEN_A", "FORBIDDEN_B"],
        );
        let file = "src/lib.rs".parse()?;
        let source = "line one\nsecond FORBIDDEN_B here\nthird";
        let findings = validator.validate(ValidationInput {
            file: &file,
            source,
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, 2);
        Ok(())
    }

    #[test]
    fn silent_when_no_marker_present() -> Result<(), Box<dyn std::error::Error>> {
        let validator = PatternValidator::new(
            "TEST-9.2".parse()?,
            "sample rule",
            Severity::Error,
            ["FORBIDDEN"],
        );
        let file = "src/lib.rs".parse()?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: "clean source text",
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }
}
