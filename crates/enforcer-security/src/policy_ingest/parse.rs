//! `.mdc`/spec-doc parser: raw text -> typed [`super::PolicySpec`] (h08,
//! POLICY-SPEC-INGESTION).
//!
//! The ingestable convention is deliberately small and generic (never tied
//! to one project's markdown dialect): `## <Section Header>` lines open a
//! section; `- token · token · token` or `- token, token` lines under
//! `Required test categories` / `Invariants` sections list bare category
//! or invariant names; lines under a `Rules` section look like
//! `- <RULE-ID> (T1|T2|T3)` — a rule id token followed by a parenthesized
//! tier. Anything else is ignored (comment prose, blank lines) rather than
//! rejected — only a document with NO recognizable sections, or a
//! recognized section with an unparseable entry, is a
//! [`PolicyIngestError`].

use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Tier;

use super::error::PolicyIngestError;
use super::spec::{AssertedRule, PolicySpec};

const REQUIRED_CATEGORIES_HEADER: &str = "Required test categories";
const INVARIANTS_HEADER: &str = "Invariants";
const RULES_HEADER: &str = "Rules";

/// Parse `source` (the raw `.mdc` text) into a typed [`PolicySpec`],
/// tagging any error with `source_name` (a fixture path or logical name)
/// for the caller's diagnostics.
///
/// Fails closed: a document with zero recognizable `## <Section>` headers,
/// or a recognized section with a line that cannot be parsed as an entry
/// for that section, returns [`PolicyIngestError`] rather than silently
/// skipping the malformed line or defaulting to an empty spec.
pub fn parse_spec(source_name: &str, source: &str) -> Result<PolicySpec, PolicyIngestError> {
    let mut current_section: Option<String> = None;
    let mut spec = PolicySpec::default();
    let mut saw_any_section = false;

    for (index, raw_line) in source.lines().enumerate() {
        let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
        let line = raw_line.trim();

        if let Some(header) = line.strip_prefix("## ") {
            current_section = Some(header.trim().to_owned());
            saw_any_section = true;
            continue;
        }

        let Some(section) = current_section.as_deref() else {
            continue;
        };

        let Some(item) = line.strip_prefix("- ") else {
            continue;
        };
        let item = item.trim();
        if item.is_empty() {
            continue;
        }

        match section {
            REQUIRED_CATEGORIES_HEADER => {
                push_unique(&mut spec.required_test_categories, item.to_owned());
            }
            INVARIANTS_HEADER => {
                push_unique(&mut spec.invariants, item.to_owned());
            }
            RULES_HEADER => {
                let asserted = parse_rule_line(source_name, section, item, line_number)?;
                merge_asserted_rule(source_name, &mut spec.asserted_rules, asserted)?;
            }
            _ => {
                // Unrecognized section: prose, not an ingestable list —
                // ignored, not rejected (keeps the convention forgiving of
                // narrative sections the reference spec also carries).
            }
        }
    }

    if !saw_any_section {
        return Err(PolicyIngestError::NoSections {
            spec_source: source_name.to_owned(),
            reason: "no `## <Section>` header found in the document".to_owned(),
        });
    }

    Ok(spec)
}

fn push_unique(list: &mut Vec<String>, item: String) {
    if !list.iter().any(|existing| existing == &item) {
        list.push(item);
    }
}

/// Parse one `- <RULE-ID> (T1|T2|T3)` line under the `Rules` section.
fn parse_rule_line(
    source_name: &str,
    section: &str,
    item: &str,
    line: u32,
) -> Result<AssertedRule, PolicyIngestError> {
    let open = item
        .find('(')
        .ok_or_else(|| PolicyIngestError::MalformedEntry {
            spec_source: source_name.to_owned(),
            section: section.to_owned(),
            reason: format!("rule entry `{item}` is missing a `(T1|T2|T3)` tier annotation"),
        })?;
    let close = item
        .rfind(')')
        .filter(|&close| close > open)
        .ok_or_else(|| PolicyIngestError::MalformedEntry {
            spec_source: source_name.to_owned(),
            section: section.to_owned(),
            reason: format!("rule entry `{item}` has an unterminated tier annotation"),
        })?;

    let rule_id_token = item[..open].trim();
    let tier_token = item[open + 1..close].trim();

    let rule_id: RuleId = rule_id_token.parse().map_err(
        |decode_err: enforcer_domain::boundary::decode_error::DecodeError| {
            PolicyIngestError::MalformedEntry {
                spec_source: source_name.to_owned(),
                section: section.to_owned(),
                reason: format!(
                    "`{rule_id_token}` is not a well-formed rule id: {}",
                    decode_err.reason
                ),
            }
        },
    )?;

    let tier = match tier_token {
        "T1" => Tier::T1,
        "T2" => Tier::T2,
        "T3" => Tier::T3,
        other => {
            return Err(PolicyIngestError::MalformedEntry {
                spec_source: source_name.to_owned(),
                section: section.to_owned(),
                reason: format!("`{other}` is not a recognized tier (expected T1, T2, or T3)"),
            })
        }
    };

    Ok(AssertedRule {
        rule_id,
        tier,
        line,
    })
}

/// Merge one newly-parsed [`AssertedRule`] into the accumulated list,
/// rejecting a same-rule-id re-assertion whose tier disagrees with the
/// first assertion (ambiguous input, never resolved by "last wins").
fn merge_asserted_rule(
    source_name: &str,
    rules: &mut Vec<AssertedRule>,
    asserted: AssertedRule,
) -> Result<(), PolicyIngestError> {
    if let Some(existing) = rules
        .iter()
        .find(|existing| existing.rule_id == asserted.rule_id)
    {
        if existing.tier != asserted.tier {
            return Err(PolicyIngestError::ConflictingSeverity {
                spec_source: source_name.to_owned(),
                rule_id: asserted.rule_id.as_str().to_owned(),
                first: format!("{:?}", existing.tier),
                second: format!("{:?}", asserted.tier),
            });
        }
        return Ok(());
    }
    rules.push(asserted);
    Ok(())
}
