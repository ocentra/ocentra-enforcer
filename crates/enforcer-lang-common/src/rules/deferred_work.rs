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

use crate::error::DeferredAnnotationError;

/// One deferral marker in the fixed cross-language vocabulary. `needle` is
/// the literal substring this rule scans for; matching is deliberately
/// substring-based (mirrors [`crate::pattern::PatternValidator`]'s
/// detection shape) rather than a per-language parser, since the same
/// marker vocabulary must span Rust/TS/Py/Dart/CFML source uniformly.
const DEFERRAL_MARKERS: &[&str] = &[
    "TODO",
    "FIXME",
    "unimplemented!",
    "todo!",
    "raise NotImplementedError",
    "throw new Error(\"not implemented\")",
    "throw new Error('not implemented')",
    "pass  # TODO",
];

/// The `DEFER-1.1` deferred-work-gate `Validator`.
pub struct DeferredWorkValidator {
    rule_id: RuleId,
}

impl DeferredWorkValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary, mirroring every other bespoke
    /// validator in this crate/workspace).
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "DEFER-1.1".parse()?,
        })
    }
}

impl Validator for DeferredWorkValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (line_idx, line) in input.source.lines().enumerate() {
            let Some(marker) = find_marker(line) else {
                continue;
            };
            let line_no = (line_idx as u32).saturating_add(1);
            match extract_annotation(line) {
                None => findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "unmarked deferred-work marker".to_owned(),
                    detail: format!(
                        "found deferral marker `{marker}` with no `DEFERRED(#<ref>)[revisit:<value>]` \
                         annotation; either resolve this stub or annotate it with a structured \
                         DEFERRED marker."
                    ),
                    file: input.file.clone(),
                    line: line_no,
                    snippet: Some(line.trim().to_owned()),
                }),
                Some(Ok(_deferred)) => {
                    // Well-formed exemption: silent.
                }
                Some(Err(parse_error)) => findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "malformed DEFERRED annotation".to_owned(),
                    detail: format!(
                        "found deferral marker `{marker}` with a malformed DEFERRED annotation: \
                         {parse_error}"
                    ),
                    file: input.file.clone(),
                    line: line_no,
                    snippet: Some(line.trim().to_owned()),
                }),
            }
        }
        findings
    }
}

/// Return the first deferral marker literal found in `line`, if any.
fn find_marker(line: &str) -> Option<&'static str> {
    DEFERRAL_MARKERS
        .iter()
        .copied()
        .find(|marker| line.contains(marker))
}

/// A successfully parsed `DEFERRED(#<ref>)[revisit:<value>]` exemption.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DeferredAnnotation {
    #[allow(dead_code)] // Captured for future use (e.g. surfacing in a report); not read yet.
    reference: String,
    #[allow(dead_code)]
    revisit: String,
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
fn extract_annotation(line: &str) -> Option<Result<DeferredAnnotation, DeferredAnnotationError>> {
    let start = line.find("DEFERRED(")?;
    let rest = line.get(start..)?;
    Some(parse_deferred_annotation(rest))
}

/// Parse-at-boundary grammar: `DEFERRED(#<ref>)[revisit:<value>]`, where
/// `<ref>` and `<value>` must both be non-empty. `raw` may have trailing
/// content after the closing `]` (e.g. more source text on the same line);
/// only the leading annotation token is parsed.
fn parse_deferred_annotation(raw: &str) -> Result<DeferredAnnotation, DeferredAnnotationError> {
    let annotation_form_error = || DeferredAnnotationError::NotDeferredForm {
        raw: raw.to_owned(),
    };
    let after_prefix = raw
        .strip_prefix("DEFERRED(")
        .ok_or_else(annotation_form_error)?;

    let (ref_body, after_ref) =
        after_prefix
            .split_once(')')
            .ok_or_else(|| DeferredAnnotationError::MissingOrEmptyRef {
                raw: raw.to_owned(),
            })?;
    let reference = ref_body.strip_prefix('#').unwrap_or(ref_body).trim();
    if reference.is_empty() {
        return Err(DeferredAnnotationError::MissingOrEmptyRef {
            raw: raw.to_owned(),
        });
    }

    let (between, after_bracket) = after_ref.split_once('[').ok_or_else(|| {
        DeferredAnnotationError::MissingOrEmptyRevisit {
            raw: raw.to_owned(),
        }
    })?;
    // Only whitespace may separate `)` and `[`; anything else means this
    // was not actually a well-formed `DEFERRED(#ref)[revisit:...]` token
    // (e.g. stray characters between the two components).
    if !between.trim().is_empty() {
        return Err(DeferredAnnotationError::MissingOrEmptyRevisit {
            raw: raw.to_owned(),
        });
    }
    let (revisit_body, _) = after_bracket.split_once(']').ok_or_else(|| {
        DeferredAnnotationError::MissingOrEmptyRevisit {
            raw: raw.to_owned(),
        }
    })?;
    let revisit_value = revisit_body
        .strip_prefix("revisit:")
        .ok_or_else(|| DeferredAnnotationError::MissingOrEmptyRevisit {
            raw: raw.to_owned(),
        })?
        .trim();
    if revisit_value.is_empty() {
        return Err(DeferredAnnotationError::MissingOrEmptyRevisit {
            raw: raw.to_owned(),
        });
    }

    Ok(DeferredAnnotation {
        reference: reference.to_owned(),
        revisit: revisit_value.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;
    use enforcer_validator::validator::Validator;

    use super::{parse_deferred_annotation, DeferredWorkValidator};
    use crate::error::DeferredAnnotationError;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

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
            source,
            scope: enforcer_domain::findings::ScanScope::Diff,
        });
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id.as_str(), "DEFER-1.1");
        assert!(findings[0].title.contains("malformed"));
        Ok(())
    }

    #[test]
    fn missing_revisit_still_fails() -> Result<(), Box<dyn std::error::Error>> {
        let validator = DeferredWorkValidator::new()?;
        let file: enforcer_domain::paths::RelPath = "crates/x/src/lib.rs".parse()?;
        let source = "// FIXME DEFERRED(#123) missing revisit bracket\n";
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source,
            scope: enforcer_domain::findings::ScanScope::Diff,
        });
        assert_eq!(findings.len(), 1);
        assert!(findings[0].title.contains("malformed"));
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
                source,
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
            source: "fn main() {}\n",
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
        let parsed =
            parse_deferred_annotation("DEFERRED(#ARC-1)[revisit:2027-01-01] trailing text")?;
        assert_eq!(parsed.reference, "ARC-1");
        assert_eq!(parsed.revisit, "2027-01-01");
        Ok(())
    }
}
