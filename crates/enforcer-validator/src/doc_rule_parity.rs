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

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_rules::registry::RuleRegistry;

/// One `must`/`never` bullet extracted from a persona doc, plus whatever
/// `[ruleId]`-shaped token(s) it cited (zero, one, or — maliciously —
/// several; only the first is used for the resolvability check, but an
/// uncited bullet is still distinguished from a cited-but-dangling one for
/// clearer [`Finding::detail`] text).
#[derive(Debug, Clone, PartialEq, Eq)]
struct ImperativeBullet {
    line: u32,
    text: String,
    cited_id: Option<String>,
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
fn extract_imperative_bullets(source: &str) -> Vec<ImperativeBullet> {
    let mut bullets = Vec::new();
    for (idx, raw_line) in source.lines().enumerate() {
        let trimmed = raw_line.trim_start();
        let Some(body) = strip_bullet_marker(trimmed) else {
            continue;
        };
        if !mentions_imperative(body) {
            continue;
        }
        let cited_id = extract_first_bracket_token(body);
        bullets.push(ImperativeBullet {
            line: (idx + 1) as u32,
            text: body.trim().to_owned(),
            cited_id,
        });
    }
    bullets
}

/// Strip a markdown list marker (`-`, `*`, `+` followed by whitespace) from
/// the front of an already-trimmed line, returning the bullet body. Lines
/// that are not list items return `None`.
fn strip_bullet_marker(trimmed: &str) -> Option<&str> {
    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = trimmed.strip_prefix(marker) {
            return Some(rest);
        }
    }
    None
}

/// True when the bullet body contains the standalone word `must` or
/// `never` (case-insensitive, word-boundary matched so `mustard` or
/// `nevertheless` do not false-positive).
fn mentions_imperative(body: &str) -> bool {
    words(body).any(|w| {
        let lower = w.to_ascii_lowercase();
        lower == "must" || lower == "never"
    })
}

/// Split on non-alphanumeric characters to yield word tokens.
fn words(body: &str) -> impl Iterator<Item = &str> {
    body.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
}

/// Extract the contents of the first `[...]` bracket group on the line, if
/// any.
fn extract_first_bracket_token(body: &str) -> Option<String> {
    let start = body.find('[')?;
    let rest = &body[start + 1..];
    let end = rest.find(']')?;
    let token = &rest[..end];
    if token.trim().is_empty() {
        None
    } else {
        Some(token.trim().to_owned())
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
fn doc_parity_finding_rule_id() -> Result<RuleId, enforcer_core::error::DecodeError> {
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
    source: &str,
    registry: &RuleRegistry,
) -> Result<Vec<Finding>, enforcer_core::error::DecodeError> {
    let self_id = doc_parity_finding_rule_id()?;
    Ok(extract_imperative_bullets(source)
        .into_iter()
        .filter_map(|bullet| bullet_finding(doc_path, &bullet, registry, &self_id))
        .collect())
}

fn bullet_finding(
    doc_path: &RelPath,
    bullet: &ImperativeBullet,
    registry: &RuleRegistry,
    self_id: &RuleId,
) -> Option<Finding> {
    let Some(cited) = &bullet.cited_id else {
        return Some(uncited_finding(doc_path, bullet, self_id));
    };
    let Ok(rule_id) = cited.parse::<RuleId>() else {
        return Some(dangling_finding(
            doc_path,
            bullet,
            cited,
            "citation does not parse as a RuleId",
            self_id,
        ));
    };
    if registry.get(&rule_id).is_none() {
        return Some(dangling_finding(
            doc_path,
            bullet,
            cited,
            "citation parses but no matching RuleId is registered",
            self_id,
        ));
    }
    None
}

fn uncited_finding(doc_path: &RelPath, bullet: &ImperativeBullet, self_id: &RuleId) -> Finding {
    Finding {
        // CLONE-JUSTIFICATION: Finding outlives this borrowed validator rule reference.
        rule_id: self_id.clone(),
        severity: Severity::Error,
        title: "must/never bullet has no [ruleId] citation".to_owned(),
        detail: format!(
            "bullet `{}` uses an imperative (must/never) with no [ruleId] citation — \
             prose pretending to be enforcement",
            bullet.text
        ),
        // CLONE-JUSTIFICATION: Finding owns its path after this borrowed input returns.
        file: doc_path.clone(),
        line: bullet.line,
        // CLONE-JUSTIFICATION: Finding owns the diagnostic excerpt independently of the parsed bullet.
        snippet: Some(bullet.text.clone()),
    }
}

fn dangling_finding(
    doc_path: &RelPath,
    bullet: &ImperativeBullet,
    cited: &str,
    reason: &str,
    self_id: &RuleId,
) -> Finding {
    Finding {
        // CLONE-JUSTIFICATION: Finding outlives this borrowed validator rule reference.
        rule_id: self_id.clone(),
        severity: Severity::Error,
        title: "must/never bullet cites an unregistered ruleId".to_owned(),
        detail: format!("bullet `{}` cites `[{cited}]` but {reason}", bullet.text),
        // CLONE-JUSTIFICATION: Finding owns its path after this borrowed input returns.
        file: doc_path.clone(),
        line: bullet.line,
        // CLONE-JUSTIFICATION: Finding owns the diagnostic excerpt independently of the parsed bullet.
        snippet: Some(bullet.text.clone()),
    }
}

/// A T2 advisory (non-blocking) reverse check: rules registered in
/// `registry` that are never mentioned (by `ruleId` citation) in any of
/// `doc_sources`. Returns advisory [`Finding`]s at [`Severity::Warning`],
/// attributed to `advisory_doc_path` (the doc set being checked as a
/// whole, since an absent mention has no single doc/line to point at) —
/// these do not fail the T1 gate, they surface coverage gaps for a human
/// to triage.
pub fn find_undocumented_rules<'a>(
    doc_sources: impl IntoIterator<Item = &'a str>,
    registry: &RuleRegistry,
    advisory_doc_path: &RelPath,
) -> Vec<Finding> {
    let mut cited_ids = std::collections::BTreeSet::new();
    for source in doc_sources {
        for bullet in extract_imperative_bullets(source) {
            if let Some(cited) = bullet.cited_id {
                cited_ids.insert(cited);
            }
        }
    }
    registry
        .iter()
        .filter(|record| !cited_ids.contains(record.rule_id.as_str()))
        .map(|record| Finding {
            // CLONE-JUSTIFICATION: Finding owns the rule identity while the registry iterator only borrows it.
            rule_id: record.rule_id.clone(),
            severity: Severity::Warning,
            title: "rule has no agent-doc mention".to_owned(),
            detail: format!(
                "rule `{}` ({}) is registered but no persona doc cites it — consider adding \
                 must/never guidance",
                record.rule_id, record.title
            ),
            // CLONE-JUSTIFICATION: each returned Finding owns the shared advisory path beyond this borrowed input.
            file: advisory_doc_path.clone(),
            line: 0,
            snippet: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{check_doc_against_registry, doc_parity_finding_rule_id, find_undocumented_rules};
    use enforcer_domain::severity::Tier;
    use enforcer_rules::registry::{FixtureRef, RuleRecord, RuleRegistry, ValidatorRef};

    #[test]
    fn doc_parity_rule_id_literal_is_valid() -> Result<(), Box<dyn std::error::Error>> {
        doc_parity_finding_rule_id()?;
        Ok(())
    }

    fn sample_registry(rule_id: &str) -> Result<RuleRegistry, Box<dyn std::error::Error>> {
        let record = RuleRecord {
            rule_id: rule_id.parse()?,
            version: 1,
            title: "Sample rule".to_owned(),
            tier: Tier::T1,
            validator: ValidatorRef {
                crate_name: "enforcer-lang-rust".to_owned(),
                path: "sample::SampleValidator".to_owned(),
            },
            fixtures: FixtureRef {
                fail: "crates/enforcer-lang-rust/fixtures/sample/fail.rs".to_owned(),
                pass: "crates/enforcer-lang-rust/fixtures/sample/pass.rs".to_owned(),
            },
            doc_anchor: "docs/rules/SAMPLE.md#SAMPLE-1".to_owned(),
            tags: vec![],
            params: serde_json::Value::Null,
        };
        Ok(RuleRegistry::from_records(vec![record])?)
    }

    #[test]
    fn cited_and_registered_bullet_passes() -> Result<(), Box<dyn std::error::Error>> {
        let registry = sample_registry("RR-1.1")?;
        let doc_path = "docs/agents/rust.md".parse()?;
        let source = "- must never leak secrets [RR-1.1]\n";
        let findings = check_doc_against_registry(&doc_path, source, &registry)?;
        assert!(findings.is_empty(), "{findings:?}");
        Ok(())
    }

    #[test]
    fn uncited_bullet_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
        let registry = sample_registry("RR-1.1")?;
        let doc_path = "docs/agents/rust.md".parse()?;
        let source = "- must never leak secrets\n";
        let findings = check_doc_against_registry(&doc_path, source, &registry)?;
        assert_eq!(findings.len(), 1);
        assert!(findings[0].detail.contains("no [ruleId] citation"));
        Ok(())
    }

    #[test]
    fn dangling_citation_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
        let registry = sample_registry("RR-1.1")?;
        let doc_path = "docs/agents/rust.md".parse()?;
        let source = "- must never leak secrets [RR-9.9]\n";
        let findings = check_doc_against_registry(&doc_path, source, &registry)?;
        assert_eq!(findings.len(), 1);
        assert!(findings[0].detail.contains("no matching RuleId"));
        Ok(())
    }

    #[test]
    fn free_text_is_ignored_by_the_gate() -> Result<(), Box<dyn std::error::Error>> {
        let registry = sample_registry("RR-1.1")?;
        let doc_path = "docs/agents/rust.md".parse()?;
        let source = "This paragraph mentions must and never in prose, not as a bullet.\n";
        let findings = check_doc_against_registry(&doc_path, source, &registry)?;
        assert!(findings.is_empty(), "{findings:?}");
        Ok(())
    }

    #[test]
    fn mustard_and_nevertheless_do_not_false_positive() -> Result<(), Box<dyn std::error::Error>> {
        let registry = sample_registry("RR-1.1")?;
        let doc_path = "docs/agents/rust.md".parse()?;
        let source = "- prefer mustard over nevertheless in prose examples\n";
        let findings = check_doc_against_registry(&doc_path, source, &registry)?;
        assert!(findings.is_empty(), "{findings:?}");
        Ok(())
    }

    #[test]
    fn undocumented_rule_surfaces_as_advisory_warning() -> Result<(), Box<dyn std::error::Error>> {
        let registry = sample_registry("RR-1.1")?;
        let advisory_path = "docs/agents".parse()?;
        let findings = find_undocumented_rules(std::iter::empty(), &registry, &advisory_path);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].severity,
            enforcer_domain::severity::Severity::Warning
        );
        Ok(())
    }

    #[test]
    fn documented_rule_has_no_advisory() -> Result<(), Box<dyn std::error::Error>> {
        let registry = sample_registry("RR-1.1")?;
        let advisory_path = "docs/agents".parse()?;
        let docs = ["- must never leak secrets [RR-1.1]\n"];
        let findings = find_undocumented_rules(docs, &registry, &advisory_path);
        assert!(findings.is_empty(), "{findings:?}");
        Ok(())
    }
}
