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
//! - `subdomain` must be in the 46-entry canonical allowlist (see
//!   [`super::vocab::ALLOWED_SUBDOMAINS`]);
//! - `tags` must have >=2 entries;
//! - EXTENSION (the gap `validate-skill.py` leaves open): every
//!   `mitre_attack` id must match `T\d{4}(\.\d{3})?` and every `nist_csf`
//!   id must match `(GV|ID|PR|DE|RS|RC)\.[A-Z]{2}(-\d{2})?`, AND both must
//!   be members of the [`super::vocab`] dictionary this same pack seeds —
//!   a malformed or unknown-vocabulary id is flagged, not silently
//!   accepted the way the Python validator accepts any string here.
//!
//! No YAML-parser dependency is added: frontmatter in this corpus is a
//! flat `key: scalar` / `key:\n- item` shape (never nested mappings), so a
//! purpose-built line scanner (matching this crate's existing text-scan
//! style) parses it without pulling in a general YAML engine.

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
}

/// Extract and parse the `---`-delimited frontmatter block at the top of
/// `source`. Returns `None` if there is no well-formed `---`/`---` block
/// (mirrors the "not this validator's concern" silent-skip convention
/// every other validator in this workspace uses for non-applicable input).
fn parse_frontmatter(source: &str) -> Option<Frontmatter> {
    let mut lines = source.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }

    let mut body_lines = Vec::new();
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        body_lines.push(line);
    }

    let mut frontmatter = Frontmatter::default();
    let mut current_list: Option<&str> = None;

    for line in body_lines {
        if let Some(item) = line.trim_start().strip_prefix("- ") {
            let value = item.trim().trim_matches(['\'', '"']).to_owned();
            match current_list {
                Some("tags") => frontmatter.tags.push(value),
                Some("mitre_attack") => frontmatter.mitre_attack.push(value),
                Some("nist_csf") => frontmatter.nist_csf.push(value),
                _ => {}
            }
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            if value.is_empty() {
                current_list = match key {
                    "tags" => Some("tags"),
                    "mitre_attack" => Some("mitre_attack"),
                    "nist_csf" => Some("nist_csf"),
                    _ => None,
                };
                continue;
            }
            current_list = None;
            let cleaned = value.trim_matches(['\'', '"']).to_owned();
            frontmatter.scalars.insert(key.to_owned(), cleaned);
        }
    }

    Some(frontmatter)
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
