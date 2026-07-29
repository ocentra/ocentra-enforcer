//! The doc-rule parity oracle (d09): cross-checks per-stack agent persona
//! prose (`docs/agents/**`, T3, human-canonical) against the structured
//! rule registry (`enforcer-rules`, d01/arc-04).
//!
//! Per RUST_ARCHITECTURE, the AI consumer reads the STRUCTURED rule; `.md`
//! prose is the human-canonical reading only, never the machine-consumed
//! source. This module is the T1 citation-parity check that keeps that
//! prose honest: every `must`/`never` bullet in a persona doc is required
//! to cite a `[ruleId]` token that resolves to a real, registered
//! [`enforcer_rules::registry::RuleRecord`]. A bullet with no citation, or
//! one that cites a dangling id, is prose PRETENDING to be enforcement —
//! this module fails it closed with a [`Finding`].
//!
//! Only `must`/`never` imperative bullets are gated. Persona free-text
//! (headers, explanatory prose, examples) is deliberately left alone: the
//! gate's job is to keep imperative claims honest, not to constrain how
//! the surrounding prose reads.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::findings::{Finding, FindingDetail, FindingLine, FindingSnippet};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;
use enforcer_rules::registry::RuleRegistry;

/// One `must`/`never` bullet extracted from a persona doc, plus whatever
/// `[ruleId]`-shaped token(s) it cited (zero, one, or — maliciously —
/// several; only the first is used for the resolvability check, but an
/// uncited bullet is still distinguished from a cited-but-dangling one for
/// clearer [`Finding::detail`] text).
#[derive(Debug, Clone, PartialEq, Eq)]
struct ImperativeBullet {
    line: SourceLine,
    text: FindingSnippet,
    cited_id: Option<FindingSnippet>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CitationFailure {
    Malformed,
    Unregistered,
}

impl CitationFailure {
    const fn message(self) -> FindingDetailMessage {
        match self {
            Self::Malformed => FindingDetailMessage::MalformedCitation,
            Self::Unregistered => FindingDetailMessage::UnregisteredCitation,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FindingDetailMessage {
    MalformedCitation,
    UnregisteredCitation,
}

impl std::fmt::Display for FindingDetailMessage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::MalformedCitation => "citation does not parse as a RuleId",
            Self::UnregisteredCitation => "citation parses but no matching RuleId is registered",
        })
    }
}

/// Extract every `must`/`never` imperative bullet from one doc's raw text.
///
/// A bullet is a markdown list line (`-`, `*`, or `+` marker, optionally
/// indented) whose body contains the standalone word `must` or `never`
/// (case-insensitive). A citation is the FIRST `[token]` bracket group on
/// that line; bracket groups are treated as citation candidates regardless
/// of whether `token` itself parses as a valid [`RuleId`] — an invalid
/// token is reported as a dangling citation, not silently ignored as "no
/// citation".
fn extract_imperative_bullets(
    source: ValidationSource<'_>,
) -> Result<Vec<ImperativeBullet>, DecodeError> {
    let mut bullets = Vec::new();
    for (idx, raw_line) in source.as_str().lines().enumerate() {
        let trimmed = ValidationSource::from_text(raw_line.trim_start());
        let Some(body) = strip_bullet_marker(trimmed) else {
            continue;
        };
        let mentions_imperative = body
            .as_str()
            .split(|character: char| !character.is_ascii_alphanumeric())
            .filter(|word| !word.is_empty())
            .any(|word| word.eq_ignore_ascii_case("must") || word.eq_ignore_ascii_case("never"));
        if !mentions_imperative {
            continue;
        }
        let cited_id = extract_first_bracket_token(body)?;
        let line = u32::try_from(idx + 1).map_err(|source| {
            DecodeError::new("docLine", format!("line number exceeds u32: {source}"))
        })?;
        let line = std::num::NonZeroU32::new(line)
            .ok_or_else(|| DecodeError::new("docLine", "line number must be non-zero"))?;
        bullets.push(ImperativeBullet {
            line: SourceLine::try_new(line),
            // ALLOC-JUSTIFICATION: the parsed bullet must outlive this borrowed source line.
            text: FindingSnippet::new(body.as_str().trim().to_owned())?,
            cited_id,
        });
    }
    Ok(bullets)
}

/// Strip a markdown list marker (`-`, `*`, `+` followed by whitespace) from
/// the front of an already-trimmed line, returning the bullet body. Lines
/// that are not list items return `None`.
fn strip_bullet_marker(trimmed: ValidationSource<'_>) -> Option<ValidationSource<'_>> {
    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = trimmed.as_str().strip_prefix(marker) {
            return Some(ValidationSource::from_text(rest));
        }
    }
    None
}

/// True when the bullet body contains the standalone word `must` or
/// `never` (case-insensitive, word-boundary matched so `mustard` or
/// `nevertheless` do not false-positive).
/// Extract the contents of the first `[...]` bracket group on the line, if
/// any.
fn extract_first_bracket_token(
    body: ValidationSource<'_>,
) -> Result<Option<FindingSnippet>, DecodeError> {
    let Some(start) = body.as_str().find('[') else {
        return Ok(None);
    };
    let Some(rest) = body.as_str().get(start + 1..) else {
        return Ok(None);
    };
    let Some(end) = rest.find(']') else {
        return Ok(None);
    };
    let Some(token) = rest.get(..end) else {
        return Ok(None);
    };
    if token.trim().is_empty() {
        Ok(None)
    } else {
        // ALLOC-JUSTIFICATION: the parsed citation must outlive this borrowed source line.
        FindingSnippet::new(token.trim().to_owned()).map(Some)
    }
}

/// The fixed literal this oracle's own findings carry as `ruleId`.
/// `RR-9.1` names the doc-rule-parity check itself (an `RR-` family record
/// this crate's own doc-rule-parity fixtures register — see
/// `crates/enforcer-validator/tests/fixtures/doc_rule_parity/**`), so its
/// findings are distinguishable from the per-stack rules it is checking.
/// Covered by [`tests::doc_parity_rule_id_literal_is_valid`].
const DOC_PARITY_RULE_ID: &str = "RR-9.1";

/// Parse [`DOC_PARITY_RULE_ID`]. Fallible because [`RuleId::from_str`] is
/// fallible in general, but the literal is fixed and its validity is
/// pinned by [`tests::doc_parity_rule_id_literal_is_valid`] — callers
/// propagate the error with `?` per this crate's `unwrap_used`/
/// `expect_used`-denied lint policy rather than assume infallibility.
fn doc_parity_finding_rule_id(
) -> Result<RuleId, enforcer_domain::boundary::decode_error::DecodeError> {
    DOC_PARITY_RULE_ID.parse()
}

/// Check one persona doc's imperative bullets against the rule registry,
/// producing a [`Finding`] for every bullet that fails closed:
///
/// - No `[ruleId]` citation at all (bare imperative prose).
/// - A citation whose token does not parse as a [`RuleId`] (malformed
///   citation shape).
/// - A citation that parses but does not resolve to any record in
///   `registry` (dangling id — the doc claims enforcement that does not
///   exist).
///
/// `doc_path` is the repo-relative path used to populate [`Finding::file`].
pub fn check_doc_against_registry(
    doc_path: &RelPath,
    source: ValidationSource<'_>,
    registry: &RuleRegistry,
) -> Result<Vec<Finding>, DecodeError> {
    let mut findings = Vec::new();
    for bullet in extract_imperative_bullets(source)? {
        if let Some(finding) = bullet_finding(doc_path, bullet, registry)? {
            findings.push(finding);
        }
    }
    Ok(findings)
}

fn bullet_finding(
    doc_path: &RelPath,
    bullet: ImperativeBullet,
    registry: &RuleRegistry,
) -> Result<Option<Finding>, DecodeError> {
    let Some(cited) = &bullet.cited_id else {
        return Ok(Some(uncited_finding(doc_path, bullet)?));
    };
    let Ok(rule_id) = cited.as_str().parse::<RuleId>() else {
        return Ok(Some(dangling_finding(
            doc_path,
            bullet,
            CitationFailure::Malformed,
        )?));
    };
    if registry.get(&rule_id).is_none() {
        return Ok(Some(dangling_finding(
            doc_path,
            bullet,
            CitationFailure::Unregistered,
        )?));
    }
    Ok(None)
}

fn uncited_finding(doc_path: &RelPath, bullet: ImperativeBullet) -> Result<Finding, DecodeError> {
    Ok(Finding {
        rule_id: doc_parity_finding_rule_id()?,
        severity: Severity::Error,
        title: "must/never bullet has no [ruleId] citation".parse()?,
        // ALLOC-JUSTIFICATION: the diagnostic owns rendered context after the parsed doc is dropped.
        detail: FindingDetail::new(format!(
            "bullet `{}` uses an imperative (must/never) with no [ruleId] citation; prose pretending to be enforcement",
            bullet.text.as_str()
        ))?,
        // CLONE-JUSTIFICATION: each returned finding independently owns the borrowed document path.
        file: doc_path.clone(),
        line: FindingLine::known(bullet.line),
        snippet: Some(bullet.text),
    })
}

fn dangling_finding(
    doc_path: &RelPath,
    bullet: ImperativeBullet,
    reason: CitationFailure,
) -> Result<Finding, DecodeError> {
    let cited = bullet
        .cited_id
        .as_ref()
        .ok_or_else(|| DecodeError::new("ruleCitation", "missing dangling citation token"))?;
    Ok(Finding {
        rule_id: doc_parity_finding_rule_id()?,
        severity: Severity::Error,
        title: "must/never bullet cites an unregistered ruleId".parse()?,
        // ALLOC-JUSTIFICATION: the diagnostic owns rendered context after the parsed doc is dropped.
        detail: FindingDetail::new(format!(
            "bullet `{}` cites [`{}`] but {}",
            bullet.text.as_str(),
            cited.as_str(),
            reason.message()
        ))?,
        // CLONE-JUSTIFICATION: each returned finding independently owns the borrowed document path.
        file: doc_path.clone(),
        line: FindingLine::known(bullet.line),
        snippet: Some(bullet.text),
    })
}
/// A T2 advisory (non-blocking) reverse check: rules registered in
/// `registry` that are never mentioned (by `ruleId` citation) in any of
/// `doc_sources`. Returns advisory [`Finding`]s at [`Severity::Warning`],
/// attributed to `advisory_doc_path` (the doc set being checked as a
/// whole, since an absent mention has no single doc/line to point at) —
/// these do not fail the T1 gate, they surface coverage gaps for a human
/// to triage.
pub fn find_undocumented_rules<'a>(
    doc_sources: impl IntoIterator<Item = ValidationSource<'a>>,
    registry: &RuleRegistry,
    advisory_doc_path: &RelPath,
) -> Result<Vec<Finding>, DecodeError> {
    let mut cited_ids = std::collections::BTreeSet::new();
    for source in doc_sources {
        for bullet in extract_imperative_bullets(source)? {
            if let Some(cited) = bullet.cited_id {
                if let Ok(rule_id) = cited.as_str().parse::<RuleId>() {
                    cited_ids.insert(rule_id);
                }
            }
        }
    }
    registry
        .iter()
        .filter(|record| !cited_ids.contains(&record.rule_id))
        .map(|record| {
            Ok(Finding {
                // CLONE-JUSTIFICATION: the advisory outlives the borrowed registry record.
                rule_id: record.rule_id.clone(),
                severity: Severity::Warning,
                title: "rule has no agent-doc mention".parse()?,
                // ALLOC-JUSTIFICATION: the advisory owns rendered registry context.
                detail: FindingDetail::new(format!(
                    "rule `{}` ({}) is registered but no persona doc cites it; consider adding must/never guidance",
                    record.rule_id, record.title
                ))?,
                // CLONE-JUSTIFICATION: each advisory independently owns the aggregate document path.
                file: advisory_doc_path.clone(),
                line: FindingLine::Unspecified,
                snippet: None,
            })
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::{check_doc_against_registry, doc_parity_finding_rule_id, find_undocumented_rules};
    use enforcer_domain::boundary::validation::ValidationSource;
    use enforcer_domain::config_types::CrateName;
    use enforcer_domain::ids::RuleId;
    use enforcer_domain::rules_types::{RuleParameters, RuleVersion};
    use enforcer_domain::severity::Tier;
    use enforcer_rules::registry::{FixtureRef, RuleRecord, RuleRegistry, ValidatorRef};

    #[test]
    fn doc_parity_rule_id_literal_is_valid() -> Result<(), Box<dyn std::error::Error>> {
        doc_parity_finding_rule_id()?;
        Ok(())
    }

    fn sample_registry(rule_id: RuleId) -> Result<RuleRegistry, Box<dyn std::error::Error>> {
        let record = RuleRecord {
            rule_id,
            version: RuleVersion::try_new(std::num::NonZeroU32::MIN),
            title: "Sample rule".parse()?,
            tier: Tier::T1,
            validator: ValidatorRef {
                crate_name: "enforcer-lang-rust".parse::<CrateName>()?,
                path: "sample::SampleValidator".parse()?,
            },
            fixtures: FixtureRef {
                fail: "crates/enforcer-lang-rust/fixtures/sample/fail.rs".parse()?,
                pass: "crates/enforcer-lang-rust/fixtures/sample/pass.rs".parse()?,
            },
            doc_anchor: "docs/rules/SAMPLE.md#SAMPLE-1".parse()?,
            tags: vec![],
            params: RuleParameters::default(),
        };
        Ok(RuleRegistry::from_records(vec![record])?)
    }

    #[test]
    fn cited_and_registered_bullet_passes() -> Result<(), Box<dyn std::error::Error>> {
        let registry = sample_registry("RR-1.1".parse()?)?;
        let doc_path = "docs/agents/rust.md".parse()?;
        let source = "- must never leak secrets [RR-1.1]\n";
        let findings =
            check_doc_against_registry(&doc_path, ValidationSource::from_text(source), &registry)?;
        assert!(findings.is_empty(), "{findings:?}");
        Ok(())
    }

    #[test]
    fn uncited_bullet_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
        let registry = sample_registry("RR-1.1".parse()?)?;
        let doc_path = "docs/agents/rust.md".parse()?;
        let source = "- must never leak secrets\n";
        let findings =
            check_doc_against_registry(&doc_path, ValidationSource::from_text(source), &registry)?;
        assert_eq!(findings.len(), 1);
        assert!(findings[0].detail.as_str().contains("no [ruleId] citation"));
        Ok(())
    }

    #[test]
    fn dangling_citation_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
        let registry = sample_registry("RR-1.1".parse()?)?;
        let doc_path = "docs/agents/rust.md".parse()?;
        let source = "- must never leak secrets [RR-9.9]\n";
        let findings =
            check_doc_against_registry(&doc_path, ValidationSource::from_text(source), &registry)?;
        assert_eq!(findings.len(), 1);
        assert!(findings[0].detail.as_str().contains("no matching RuleId"));
        Ok(())
    }

    #[test]
    fn free_text_is_ignored_by_the_gate() -> Result<(), Box<dyn std::error::Error>> {
        let registry = sample_registry("RR-1.1".parse()?)?;
        let doc_path = "docs/agents/rust.md".parse()?;
        let source = "This paragraph mentions must and never in prose, not as a bullet.\n";
        let findings =
            check_doc_against_registry(&doc_path, ValidationSource::from_text(source), &registry)?;
        assert!(findings.is_empty(), "{findings:?}");
        Ok(())
    }

    #[test]
    fn mustard_and_nevertheless_do_not_false_positive() -> Result<(), Box<dyn std::error::Error>> {
        let registry = sample_registry("RR-1.1".parse()?)?;
        let doc_path = "docs/agents/rust.md".parse()?;
        let source = "- prefer mustard over nevertheless in prose examples\n";
        let findings =
            check_doc_against_registry(&doc_path, ValidationSource::from_text(source), &registry)?;
        assert!(findings.is_empty(), "{findings:?}");
        Ok(())
    }

    #[test]
    fn undocumented_rule_surfaces_as_advisory_warning() -> Result<(), Box<dyn std::error::Error>> {
        let registry = sample_registry("RR-1.1".parse()?)?;
        let advisory_path = "docs/agents".parse()?;
        let findings = find_undocumented_rules(std::iter::empty(), &registry, &advisory_path)?;
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].severity,
            enforcer_domain::severity::Severity::Warning
        );
        Ok(())
    }

    #[test]
    fn documented_rule_has_no_advisory() -> Result<(), Box<dyn std::error::Error>> {
        let registry = sample_registry("RR-1.1".parse()?)?;
        let advisory_path = "docs/agents".parse()?;
        let docs = [ValidationSource::from_text(
            "- must never leak secrets [RR-1.1]\n",
        )];
        let findings = find_undocumented_rules(docs, &registry, &advisory_path)?;
        assert!(findings.is_empty(), "{findings:?}");
        Ok(())
    }
}
