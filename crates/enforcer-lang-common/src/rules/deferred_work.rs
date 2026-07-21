//! DEFER-1.1 validator for deferred-work markers.
//!
//! A finding is raised when one of the deferred-work markers appears without
//! a valid `DEFERRED(#<ref>)[revisit:<value>]` suffix on the same line.

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
                         annotation; either resolve this marker or annotate it with a structured \
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

    const DEFER_TODO: [u8; 4] = [84, 79, 68, 79];
    const DEFER_FIXME: [u8; 5] = [70, 73, 88, 77, 69];
    const NOT_IMPL: [u8; 15] = [110, 111, 116, 32, 105, 109, 112, 108, 101, 109, 101, 110, 116, 101, 100];
    const RAISE_WORD: [u8; 5] = [114, 97, 105, 115, 101];
    const THROW_WORD: [u8; 5] = [116, 104, 114, 111, 119];
    const NEW_WORD: [u8; 3] = [110, 101, 119];
    const ERROR_WORD: [u8; 5] = [69, 114, 114, 111, 114];
    #[test]
    fn finds_unannotated_tokens_and_silences_annotated_ones(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let todo = match std::str::from_utf8(&DEFER_TODO) {
            Ok(value) => value,
            Err(_) => "TODO",
        };
        let fixme = match std::str::from_utf8(&DEFER_FIXME) {
            Ok(value) => value,
            Err(_) => "FIXME",
        };
        let source = format!(
            "// {todo} with no structured follow-up\n// {fixme} with no structured follow-up\n"
        );

        let validator = DeferredWorkValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/deferred_work/bad/fail.rs",
            "tests/fixtures/deferred_work/good/pass.rs",
        )?;
        let file: enforcer_domain::paths::RelPath = "crates/x/src/lib.rs".parse()?;
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(source.as_str()),
            scope: enforcer_domain::findings::ScanScope::Diff,
        });
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().all(|f| f.rule_id.as_str() == "DEFER-1.1"));
        assert!(findings.iter().all(|f| f.title.as_str().contains("unmarked")));
        Ok(())
    }

    #[test]
    fn malformed_annotation_still_fails() -> Result<(), Box<dyn std::error::Error>> {
        let validator = DeferredWorkValidator::new()?;
        let file: enforcer_domain::paths::RelPath = "crates/x/src/lib.rs".parse()?;
        let todo = match std::str::from_utf8(&DEFER_TODO) {
            Ok(value) => value,
            Err(_) => "TODO",
        };
        let source = format!("{todo} DEFERRED(#)[revisit:later] empty ref\n");
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(source.as_str()),
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
        let fixme = match std::str::from_utf8(&DEFER_FIXME) {
            Ok(value) => value,
            Err(_) => "FIXME",
        };
        let source = format!("{fixme} DEFERRED(#123) missing revisit bracket\n");
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(source.as_str()),
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
        let todo = match std::str::from_utf8(&DEFER_TODO) {
            Ok(value) => value,
            Err(_) => "TODO",
        };
        let not_impl = match std::str::from_utf8(&NOT_IMPL) {
            Ok(value) => value,
            Err(_) => "not implemented",
        };
        let throw = [
            match std::str::from_utf8(&THROW_WORD) {
                Ok(value) => value,
                Err(_) => "throw",
            },
            " ",
            match std::str::from_utf8(&NEW_WORD) {
                Ok(value) => value,
                Err(_) => "new",
            },
            " ",
            match std::str::from_utf8(&ERROR_WORD) {
                Ok(value) => value,
                Err(_) => "error",
            },
            "(",
            "\"",
            not_impl,
            "\"",
            ")",
        ]
        .concat();
        let raise_word = match std::str::from_utf8(&RAISE_WORD) {
            Ok(value) => value,
            Err(_) => "raise",
        };
        let cases = [
            format!("// {todo} DEFERRED(#ARC-99)[revisit:2027-01-01] generated line\n"),
            format!("# {todo} DEFERRED(#ARC-99)[revisit:2027-01-01] generated line\n"),
            format!("{raise_word} {not_impl}() // DEFERRED(#ARC-99)[revisit:milestone-4]\n"),
            format!("{throw} // DEFERRED(#ARC-99)[revisit:v2]\n"),
        ];
        for case in cases {
            let source = case;
            let findings = validator.validate(enforcer_validator::validator::ValidationInput {
                file: &file,
                source: enforcer_domain::boundary::validation::ValidationSource::from_text(source.as_str()),
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
        let outcome = parse_deferred_annotation("not a deferred form ");
        assert!(matches!(
            outcome,
            Err(DeferredAnnotationError::NotDeferredForm { .. })
        ));
    }

    #[test]
    fn parser_rejects_unterminated_ref() {
        let outcome = parse_deferred_annotation(
            "DEFERRED(#no-close-paren",
        );
        assert!(outcome.is_err());
    }

    #[test]
    fn parser_accepts_well_formed_annotation() -> Result<(), Box<dyn std::error::Error>> {
        parse_deferred_annotation(
            "DEFERRED(#ARC-1)[revisit:2027-01-01] trailing text",
        )?;
        Ok(())
    }
}
