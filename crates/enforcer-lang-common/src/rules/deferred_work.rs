//! `DEFER-1.1` — the deferred-work gate.
//!
//! Detects a fixed vocabulary of deferral markers (`TODO`, `FIXME`,
//! `unimplemented!`, `todo!`, `raise NotImplementedError`,
//! `throw new Error("not implemented")`-shaped stub throws, `pass  # TODO`)
//! across every target language this engine validates (this crate is
//! `enforcer-lang-common` — it validates USER/TARGET-REPO code in
//! Rust/TS/Py/Dart/etc., not this engine's own source). A marker is exempt
//! ONLY when it carries a well-formed `DEFERRED(#<ref>)[revisit:<value>]`
//! annotation on the same line; a malformed annotation attempt (e.g. an
//! empty `#<ref>` or missing `[revisit:...]`) still fails, distinctly from
//! an unmarked stub, so a caller can tell "no annotation" apart from
//! "broken annotation" in the finding detail.
//!
//! Keyed to arc-04's rule record
//! (`crates/enforcer-rules/rules/deferred-work-gate.json`, `ruleId`
//! `DEFER-1.1`).
//!
//! # Diff scoping
//!
//! Per the workpack: "only lines added/changed in the working diff are
//! gated, so legacy stubs do not block". This validator is pure over
//! [`ValidationInput`] (same input, same findings — the harness's parity
//! contract), so file-level diff scoping is the caller's job: the
//! `enforcer-scan` run context (`ScanScope::Diff`, see
//! `enforcer-scan::scope`) decides WHICH FILES reach this validator's
//! `validate` in a diff-scoped run. Composing with d02's baseline
//! ratchet, a diff-scoped scan invocation only ever calls this validator
//! on files that changed, which is exactly "legacy stub outside the diff
//! passes" at the file granularity this crate can prove without owning
//! `enforcer-scan`'s hunk machinery.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::source_analysis;

/// The `DEFER-1.1` deferred-work-gate `Validator`.
#[derive(Debug)]
pub struct DeferredWorkValidator {
    rule_id: RuleId,
}

impl DeferredWorkValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary, mirroring every other bespoke
    /// validator in this crate/workspace).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::static_rule_id("DEFER-1.1")?,
        })
    }
}

impl Validator for DeferredWorkValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (line_idx, line) in input.source.as_str().lines().enumerate() {
            let Some(marker) = source_analysis::find_deferred_marker(line) else {
                continue;
            };
            let line_no = crate::boundary::line_number(line_idx);
            match source_analysis::extract_deferred_annotation(line) {
                None => findings.push(finding!(
                    &self.rule_id,
                    Severity::Error,
                    "unmarked deferred-work marker",
                    format!(
                        "found deferral marker `{marker}` with no `DEFERRED(#<ref>)[revisit:<value>]` \
                         annotation; either resolve this stub or annotate it with a structured \
                         DEFERRED marker."
                    ),
                    input,
                    line_no,
                    Some(line.trim()),
                )),
                Some(Ok(_deferred)) => {
                    // Well-formed exemption: silent.
                }
                Some(Err(parse_error)) => findings.push(finding!(
                    &self.rule_id,
                    Severity::Error,
                    "malformed DEFERRED annotation",
                    format!(
                        "found deferral marker `{marker}` with a malformed DEFERRED annotation: \
                         {parse_error}"
                    ),
                    input,
                    line_no,
                    Some(line.trim()),
                )),
            }
        }
        findings
    }
}

/// Look for a `DEFERRED(...)` token anywhere in `line`.
///
/// - Returns `None` when no `DEFERRED(` token is present at all (the caller
///   treats this as "unmarked").
/// - Returns `Some(Err(_))` when a `DEFERRED(` token is present but the
///   annotation does not fully match the
///   `DEFERRED(#<ref>)[revisit:<value>]` grammar (the caller treats this as
///   "malformed", a distinct failure from unmarked).
/// - Returns `Some(Ok(_))` only for a fully well-formed annotation with a
///   non-empty `<ref>` and non-empty `<value>`.
///
/// Parse-at-boundary grammar: `DEFERRED(#<ref>)[revisit:<value>]`, where
/// `<ref>` and `<value>` must both be non-empty. `raw` may have trailing
/// content after the closing `]` (e.g. more source text on the same line);
/// only the leading annotation token is parsed.
#[cfg(test)]
mod tests {
    use crate::boundary::source_analysis::parse_deferred_annotation;
    use crate::boundary::{manifest_dir, run_fixture_parity};
    use enforcer_validator::validator::Validator;

    use super::DeferredWorkValidator;
    use crate::error::DeferredAnnotationError;

    #[test]
    fn fires_on_unmarked_stub_and_silent_on_annotated_stub(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = DeferredWorkValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/deferred_work/bad/fail.rs",
            "tests/fixtures/deferred_work/good/pass.rs",
        )?;
        Ok(())
    }

    #[test]
    fn malformed_annotation_still_fails() -> Result<(), Box<dyn std::error::Error>> {
        let validator = DeferredWorkValidator::new()?;
        let file: enforcer_domain::paths::RelPath = "crates/x/src/lib.rs".parse()?;
        let source = "// TODO DEFERRED(#)[revisit:later] empty ref\n";
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
            scope: enforcer_domain::findings::ScanScope::Diff,
        });
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id.as_str(), "DEFER-1.1");
        assert!(findings[0].title.as_str().contains("malformed"));
        Ok(())
    }

    #[test]
    fn missing_revisit_still_fails() -> Result<(), Box<dyn std::error::Error>> {
        let validator = DeferredWorkValidator::new()?;
        let file: enforcer_domain::paths::RelPath = "crates/x/src/lib.rs".parse()?;
        let source = "// FIXME DEFERRED(#123) missing revisit bracket\n";
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
            scope: enforcer_domain::findings::ScanScope::Diff,
        });
        assert_eq!(findings.len(), 1);
        assert!(findings[0].title.as_str().contains("malformed"));
        Ok(())
    }

    #[test]
    fn well_formed_annotation_is_silent_across_languages() -> Result<(), Box<dyn std::error::Error>>
    {
        let validator = DeferredWorkValidator::new()?;
        let file: enforcer_domain::paths::RelPath = "crates/x/src/lib.rs".parse()?;
        let cases = [
            "// TODO DEFERRED(#ARC-99)[revisit:2027-01-01] rust todo\n",
            "# TODO DEFERRED(#ARC-99)[revisit:2027-01-01] python todo\n",
            "raise NotImplementedError() # DEFERRED(#ARC-99)[revisit:milestone-4]\n",
            "throw new Error(\"not implemented\") // DEFERRED(#ARC-99)[revisit:v2]\n",
        ];
        for source in cases {
            let findings = validator.validate(enforcer_validator::validator::ValidationInput {
                file: &file,
                source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
                scope: enforcer_domain::findings::ScanScope::Diff,
            });
            assert!(findings.is_empty(), "expected silence for: {source}");
        }
        Ok(())
    }

    #[test]
    fn clean_source_with_no_markers_is_silent() -> Result<(), Box<dyn std::error::Error>> {
        let validator = DeferredWorkValidator::new()?;
        let file: enforcer_domain::paths::RelPath = "crates/x/src/lib.rs".parse()?;
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                "fn main() {}\n",
            ),
            scope: enforcer_domain::findings::ScanScope::Diff,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn parser_rejects_non_deferred_form() {
        let outcome = parse_deferred_annotation("not a deferred token");
        assert!(matches!(
            outcome,
            Err(DeferredAnnotationError::NotDeferredForm { .. })
        ));
    }

    #[test]
    fn parser_rejects_unterminated_ref() {
        let outcome = parse_deferred_annotation("DEFERRED(#no-close-paren");
        assert!(matches!(
            outcome,
            Err(DeferredAnnotationError::MissingOrEmptyRef { .. })
        ));
    }

    #[test]
    fn parser_accepts_well_formed_annotation() -> Result<(), Box<dyn std::error::Error>> {
        parse_deferred_annotation("DEFERRED(#ARC-1)[revisit:2027-01-01] trailing text")?;
        Ok(())
    }
}
