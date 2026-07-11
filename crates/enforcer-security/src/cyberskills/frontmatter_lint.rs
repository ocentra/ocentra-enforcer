//! `CYBER-FRONTMATTER.1` (T1) — the SKILL.md frontmatter linter (h11
//! workpack): ports `vendor/anthropic-cybersecurity-skills/tools/validate-skill.py`
//! to a native Rust `Validator` over a skill file's `---`-delimited YAML
//! frontmatter block, then EXTENDS it with the check the corpus validator
//! leaves open (workpack §"T1 frontmatter lint"):
//!
//! - the 8 `REQUIRED_FIELDS` (name/description/domain/subdomain/tags/
//!   version/author/license) must all be present;
//! - `name` must be kebab-case (`^[a-z0-9]+(-[a-z0-9]+)*$`) and <=64 chars;
//! - `description` must be >=50 chars;
//! - `domain` must equal `cybersecurity`;
//! - `subdomain` must be in the 46-value vendor allowlist — the flat set of
//!   canonical forms PLUS their accepted aliases, ported 1:1 from
//!   `validate-skill.py` (see [`super::vocab::ALLOWED_SUBDOMAINS`]);
//! - `tags` must have >=2 entries;
//! - EXTENSION (the gap `validate-skill.py` leaves open): every
//!   `mitre_attack` id must match `T\d{4}(\.\d{3})?` and every `nist_csf`
//!   id must match `(GV|ID|PR|DE|RS|RC)\.[A-Z]{2}(-\d{2})?`, AND both must
//!   be members of the [`super::vocab`] dictionary this same pack seeds —
//!   a malformed or unknown-vocabulary id is flagged, not silently
//!   accepted the way the Python validator accepts any string here.
//!
//! No YAML-parser dependency is added: a purpose-built line scanner
//! ([`parse_frontmatter`]) reproduces `validate-skill.py`'s own stdlib
//! parser — top-level-only keys (indented/nested lines are skipped so a
//! nested `name:` cannot clobber the real field), block lists (`key:\n-
//! item`), inline lists (`key: [a, b]`), and folded scalars (`key: >-`
//! with indented continuation) — so our accept/reject verdict matches the
//! vendor's on the real corpus without pulling in a general YAML engine.

use std::collections::BTreeMap;

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

use super::vocab::{is_known_mitre_attack_id, is_known_nist_csf_id, ALLOWED_SUBDOMAINS};

const REQUIRED_FIELDS: &[&str] = &[
    "name",
    "description",
    "domain",
    "subdomain",
    "tags",
    "version",
    "author",
    "license",
];

const DESCRIPTION_MIN_CHARS: usize = 50;

/// One parsed frontmatter block: scalar fields plus the two list fields
/// (`tags`, `mitre_attack`, `nist_csf`) this linter inspects.
#[derive(Debug, Default)]
struct Frontmatter {
    scalars: BTreeMap<String, String>,
    tags: Vec<String>,
    mitre_attack: Vec<String>,
    nist_csf: Vec<String>,
}

impl Frontmatter {
    fn field(&self, name: &str) -> Option<&str> {
        self.scalars.get(name).map(String::as_str)
    }

    fn has_field(&self, name: &str) -> bool {
        match name {
            "tags" => !self.tags.is_empty() || self.scalars.contains_key("tags"),
            _ => self.scalars.contains_key(name),
        }
    }

    /// Append one list item to the list tracked for `key` (only the three
    /// list fields this linter inspects are retained; other list-valued
    /// keys are parsed but discarded, matching the vendor parser which
    /// stores every list but where only these three are ever read).
    fn push_list_item(&mut self, key: &str, value: String) {
        match key {
            "tags" => self.tags.push(value),
            "mitre_attack" => self.mitre_attack.push(value),
            "nist_csf" => self.nist_csf.push(value),
            _ => {}
        }
    }
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches(['\'', '"']).to_owned()
}

/// `key: [a, b, c]` inline list. Returns the key and its items.
fn parse_inline_list(stripped: &str) -> Option<(String, Vec<String>)> {
    let (key, rest) = stripped.split_once(':')?;
    let key = key.trim();
    if key.is_empty() || key.contains(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim();
    let inner = rest.strip_prefix('[')?.strip_suffix(']')?;
    let items = inner
        .split(',')
        .map(unquote)
        .filter(|s| !s.is_empty())
        .collect();
    Some((key.to_owned(), items))
}

/// `key: >-` / `key: >` folded-scalar start. Returns the key; the caller
/// then collects the following indented non-empty lines.
fn parse_folded_start(stripped: &str) -> Option<String> {
    let (key, rest) = stripped.split_once(':')?;
    let key = key.trim();
    if key.is_empty() || key.contains(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim();
    if rest == ">" || rest == ">-" || rest == ">|" {
        Some(key.to_owned())
    } else {
        None
    }
}

/// Extract and parse the `---`-delimited frontmatter block at the top of
/// `source`. Returns `None` if there is no well-formed `---`/`---` block
/// (mirrors the "not this validator's concern" silent-skip convention
/// every other validator in this workspace uses for non-applicable input).
///
/// Ported line-for-line from `validate-skill.py`'s `parse_frontmatter`
/// (the vendor's stdlib-only parser) so our accept/reject verdict matches
/// the vendor's on the real corpus:
/// - only TOP-LEVEL (column-0) keys define fields — an indented `key: value`
///   belongs to a nested mapping (e.g. a framework-map object with its own
///   `name:`) and must not clobber the real field;
/// - `key: [a, b]` inline lists;
/// - `key: >-` / `key: >` folded scalars — content on following indented
///   lines is joined with spaces (12 corpus skills write `description:` this
///   way; the old parser dropped it and false-flagged "missing description");
/// - `key:\n  - item` block lists;
/// - `key: value` plain scalars; an empty value marks a present-but-valueless
///   key (the start of a block list), never a stored scalar.
fn parse_frontmatter(source: &str) -> Option<Frontmatter> {
    let mut lines = source.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }

    let mut body_lines = Vec::new();
    let mut closed = false;
    for line in lines {
        if line.trim() == "---" {
            closed = true;
            break;
        }
        body_lines.push(line);
    }
    if !closed {
        return None;
    }

    let mut fm = Frontmatter::default();
    let mut current_key: Option<String> = None;
    let mut in_folded = false;
    let mut folded: Vec<String> = Vec::new();

    let flush_folded = |fm: &mut Frontmatter, key: &Option<String>, folded: &[String]| {
        if let Some(k) = key {
            if !folded.is_empty() {
                fm.scalars.insert(k.clone(), folded.join(" "));
            }
        }
    };

    for line in body_lines {
        let stripped = line.trim();
        let indented = line.starts_with(' ') || line.starts_with('\t');

        // Flush a completed folded scalar when the next TOP-LEVEL key begins.
        if in_folded && !stripped.is_empty() && !indented {
            flush_folded(&mut fm, &current_key, &folded);
            in_folded = false;
            folded.clear();
            current_key = None;
        }
        if in_folded {
            if !stripped.is_empty() {
                folded.push(stripped.to_owned());
            }
            continue;
        }

        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }

        // List items (before key:value); appended to the current key's list.
        if let Some(item) = stripped.strip_prefix("- ") {
            if let Some(key) = current_key.clone() {
                fm.push_list_item(&key, unquote(item));
            }
            continue;
        }

        // Only top-level keys define fields; skip indented (nested) lines.
        if indented {
            continue;
        }

        if let Some((key, items)) = parse_inline_list(stripped) {
            for item in items {
                fm.push_list_item(&key, item);
            }
            current_key = Some(key);
            continue;
        }

        if let Some(key) = parse_folded_start(stripped) {
            current_key = Some(key);
            in_folded = true;
            folded.clear();
            continue;
        }

        if let Some((key, value)) = stripped.split_once(':') {
            let key = key.trim().to_owned();
            let value = unquote(value);
            if !value.is_empty() {
                fm.scalars.insert(key.clone(), value);
            }
            // Empty value => key present but value-less (start of a block
            // list); tracked via current_key so following `- item` lines land.
            current_key = Some(key);
        }
    }

    if in_folded {
        flush_folded(&mut fm, &current_key, &folded);
    }

    Some(fm)
}

fn kebab_case_re() -> Result<Regex, DecodeError> {
    Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$")
        .map_err(|err| DecodeError::new("cyberskillsKebabCaseRegex", err.to_string()))
}

/// `CYBER-FRONTMATTER.1` — SKILL.md frontmatter structural + threat-id
/// well-formedness/membership gate.
pub struct SkillFrontmatterValidValidator {
    rule_id: RuleId,
    kebab: Regex,
}

impl SkillFrontmatterValidValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-FRONTMATTER.1".parse()?,
            kebab: kebab_case_re()?,
        })
    }

    fn structural_errors(&self, fm: &Frontmatter) -> Vec<String> {
        let mut errors = Vec::new();

        for field in REQUIRED_FIELDS {
            if !fm.has_field(field) {
                errors.push(format!("missing required field: {field}"));
            }
        }

        if let Some(name) = fm.field("name") {
            if !self.kebab.is_match(name) {
                errors.push(format!(
                    "name '{name}' is not valid kebab-case (lowercase letters, digits, hyphens \
                     only)"
                ));
            }
            if name.len() > 64 {
                errors.push(format!("name too long ({} chars, max 64)", name.len()));
            }
        }

        if let Some(description) = fm.field("description") {
            if description.len() < DESCRIPTION_MIN_CHARS {
                errors.push(format!(
                    "description too short ({} chars, min {DESCRIPTION_MIN_CHARS})",
                    description.len()
                ));
            }
        }

        if let Some(domain) = fm.field("domain") {
            if domain != "cybersecurity" {
                errors.push(format!("domain must be 'cybersecurity', got '{domain}'"));
            }
        }

        if let Some(subdomain) = fm.field("subdomain") {
            if !ALLOWED_SUBDOMAINS.contains(&subdomain) {
                errors.push(format!("unknown subdomain '{subdomain}'"));
            }
        }

        if fm.tags.len() < 2 {
            errors.push(format!("need at least 2 tags, got {}", fm.tags.len()));
        }

        errors
    }

    /// The extension the corpus validator leaves open: well-formedness +
    /// vocabulary membership for every `mitre_attack`/`nist_csf` id.
    fn threat_id_errors(&self, fm: &Frontmatter) -> Vec<String> {
        let mut errors = Vec::new();
        for id in &fm.mitre_attack {
            if !is_known_mitre_attack_id(id) {
                errors.push(format!(
                    "mitre_attack id '{id}' is malformed or not in the h03 threat vocabulary"
                ));
            }
        }
        for id in &fm.nist_csf {
            if !is_known_nist_csf_id(id) {
                errors.push(format!(
                    "nist_csf id '{id}' is malformed or not in the h03 threat vocabulary"
                ));
            }
        }
        errors
    }
}

impl Validator for SkillFrontmatterValidValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(fm) = parse_frontmatter(input.source) else {
            return Vec::new();
        };

        let mut errors = self.structural_errors(&fm);
        errors.extend(self.threat_id_errors(&fm));

        if errors.is_empty() {
            return Vec::new();
        }

        vec![Finding {
            rule_id: self.rule_id.clone(),
            severity: Severity::Error,
            title: "SKILL.md frontmatter is invalid".to_owned(),
            detail: format!(
                "frontmatter validation failed: {}. Fix: correct the listed field(s) per the \
                 skill-frontmatter schema (8 required fields, kebab-case name <=64 chars, \
                 description >=50 chars, domain==cybersecurity, allowlisted subdomain, >=2 \
                 tags, and every mitre_attack/nist_csf id well-formed and in the h03 \
                 vocabulary).",
                errors.join("; ")
            ),
            file: input.file.clone(),
            line: 1,
            snippet: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::SkillFrontmatterValidValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_frontmatter_lint() -> Result<(), Box<dyn std::error::Error>> {
        let validator = SkillFrontmatterValidValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/skill_frontmatter/bad/bad_attack_id.md",
            "tests/fixtures/cyberskills/skill_frontmatter/good/valid.md",
        )?;
        Ok(())
    }
}
