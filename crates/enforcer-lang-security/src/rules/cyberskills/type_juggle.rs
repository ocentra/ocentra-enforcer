//! `CYBER-TYPE-JUGGLE.1` (h11) — harvest target:
//! `vendor/anthropic-cybersecurity-skills/skills/exploiting-type-juggling-vulnerabilities/SKILL.md`
//! (`scripts/agent.py` is a dynamic HTTP-fuzzing tool with no static
//! call-site table to port; the harvest below is drawn from the SKILL.md
//! workflow/key-concepts sections instead).
//!
//! Four vendor-named PHP loose-comparison shapes, each narrowed from the
//! skill's dynamic-testing workflow to a concrete static call-site pattern:
//!
//! - **Loose comparison on request input** (Step 2 "Exploit Loose Comparison
//!   Authentication Bypass", Step 4 "Exploit Comparison in Access Control",
//!   Key Concepts "Loose Comparison (==)"): `==`/`!=` (never `===`/`!==`)
//!   with a PHP superglobal (`$_GET`/`$_POST`/`$_REQUEST`/`$_COOKIE`) on the
//!   same line — the skill's `password=0`/`password=true`/`role=true`
//!   payloads all exploit exactly this shape.
//! - **`strcmp()` NULL-return bypass** (Step 2 "PHP strcmp vulnerability:
//!   strcmp(array, string) returns NULL" / "password[]=anything", Key
//!   Concepts "strcmp Bypass", Common Scenario 3): `strcmp(...) == 0`,
//!   `strcmp(...) != 0`, or the bare-negated `!strcmp(...)` — every one of
//!   these treats strcmp()'s NULL failure return as a match, since
//!   `NULL == 0` is `TRUE` under loose comparison.
//! - **`in_array()` without strict mode on request input** (Key Concepts
//!   "Type Coercion" / Remediation "Replace all == with === (strict
//!   comparison)"; the skill's magic-hash and numeric-PIN payloads bypass
//!   exactly this idiom when a haystack lookup omits the strict third
//!   argument): `in_array($_GET[...]/$_POST[...]/..., $haystack)` with no
//!   trailing `, true)`.
//! - **Magic-hash loose comparison** (Step 3 "Exploit Magic Hash
//!   Collisions", Key Concepts "Magic Hash" / "Scientific Notation", Common
//!   Scenario 2 "Magic Hash Collision"): `md5(...)`/`sha1(...)` compared
//!   with a loose `==`/`!=`, or two hash-looking variables (name contains
//!   `hash`) compared the same way — both let a `"0e<digits>"` collision
//!   forge a match.
//!
//! Every category requires the loose-operator shape specifically: `===`
//! and `!==` never trigger any of the four checks, matching the skill's own
//! Remediation guidance ("Replace all == with === (strict comparison)...
//! Use hash_equals() for timing-safe hash comparison").

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// `CYBER-TYPE-JUGGLE.1` — flags PHP loose-comparison (`==`/`!=`) idioms
/// that are vulnerable to type-juggling bypass: request input compared
/// loosely, `strcmp()` NULL-return bypass, non-strict `in_array()` on
/// request input, and magic-hash (`"0e..."`) loose hash comparison.
#[derive(Debug)]
pub struct TypeJugglingValidator {
    rule_id: RuleId,
    /// A PHP request-input superglobal array access:
    /// `$_GET[...]`/`$_POST[...]`/`$_REQUEST[...]`/`$_COOKIE[...]`.
    request_input: Regex,
    /// A standalone loose `==` (never matches inside `===` or `!==`).
    loose_eq: Regex,
    /// A standalone loose `!=` (never matches inside `!==`).
    loose_ne: Regex,
    /// A `strcmp(` call site.
    strcmp_call: Regex,
    /// The bare-negated `!strcmp(` idiom, which always treats a NULL
    /// (array-input) return as truthy regardless of any `==`/`!=` operator.
    bang_strcmp: Regex,
    /// An `in_array(` call site.
    in_array_call: Regex,
    /// An `in_array(...)` call whose last argument is the literal strict
    /// flag `true`.
    in_array_strict: Regex,
    /// A `md5(` or `sha1(` call site.
    md5_sha1_call: Regex,
    /// A PHP variable whose name contains `hash` (case-insensitive), e.g.
    /// `$hash`, `$storedHash`, `$expected_hash`.
    hash_var: Regex,
}

impl TypeJugglingValidator {
    pub fn new() -> Result<Self, DecodeError> {
        let request_input =
            Regex::new(r"\$_(?:GET|POST|REQUEST|COOKIE)\s*\[[^\]]*\]").map_err(|err| {
                crate::boundary::regex::decode("cyberskillsTypeJuggleRequestInput", err)
            })?;
        // No lookaround is available, so a standalone `==` is matched by
        // requiring the character immediately before it to be neither `=`
        // nor `!` (ruling out the second `=` of `===` and the `=` of
        // `!==`), and the character immediately after it to not be `=`
        // (ruling out the first `=` of `===`). `^`/`$` cover the case where
        // the operator sits at a line boundary.
        let loose_eq = Regex::new(r"(?:^|[^=!])==(?:[^=]|$)")
            .map_err(|err| crate::boundary::regex::decode("cyberskillsTypeJuggleLooseEq", err))?;
        // A standalone `!=` is matched by requiring the character right
        // after it to not be `=` (ruling out `!==`).
        let loose_ne = Regex::new(r"!=(?:[^=]|$)")
            .map_err(|err| crate::boundary::regex::decode("cyberskillsTypeJuggleLooseNe", err))?;
        let strcmp_call = Regex::new(r"strcmp\s*\(").map_err(|err| {
            crate::boundary::regex::decode("cyberskillsTypeJuggleStrcmpCall", err)
        })?;
        let bang_strcmp = Regex::new(r"!\s*strcmp\s*\(").map_err(|err| {
            crate::boundary::regex::decode("cyberskillsTypeJuggleBangStrcmp", err)
        })?;
        let in_array_call = Regex::new(r"in_array\s*\(").map_err(|err| {
            crate::boundary::regex::decode("cyberskillsTypeJuggleInArrayCall", err)
        })?;
        let in_array_strict = Regex::new(r"in_array\s*\([^)]*,\s*true\s*\)").map_err(|err| {
            crate::boundary::regex::decode("cyberskillsTypeJuggleInArrayStrict", err)
        })?;
        let md5_sha1_call = Regex::new(r"\b(?:md5|sha1)\s*\(").map_err(|err| {
            crate::boundary::regex::decode("cyberskillsTypeJuggleMd5Sha1Call", err)
        })?;
        let hash_var = Regex::new(r"(?i)\$\w*hash\w*")
            .map_err(|err| crate::boundary::regex::decode("cyberskillsTypeJuggleHashVar", err))?;
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberTypeJuggle.id(),
            request_input,
            loose_eq,
            loose_ne,
            strcmp_call,
            bang_strcmp,
            in_array_call,
            in_array_strict,
            md5_sha1_call,
            hash_var,
        })
    }
}

impl Validator for TypeJugglingValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (index, line) in input.source.as_str().lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
            let mut matched_labels: Vec<&str> = Vec::new();

            let has_request_input = self.request_input.is_match(line);
            let has_loose_eq = self.loose_eq.is_match(line);
            let has_loose_ne = self.loose_ne.is_match(line);
            let has_loose_cmp = has_loose_eq || has_loose_ne;

            if has_request_input && has_loose_cmp {
                matched_labels.push(
                    "loose ==/!= comparison against PHP request input ($_GET/$_POST/\
                     $_REQUEST/$_COOKIE) instead of ===/!==",
                );
            }

            if self.bang_strcmp.is_match(line) || (self.strcmp_call.is_match(line) && has_loose_cmp)
            {
                matched_labels.push(
                    "strcmp() result compared with a loose ==/!= (or bare-negated): strcmp() \
                     returns NULL on array input and NULL loosely equals 0, bypassing the check",
                );
            }

            if has_request_input
                && self.in_array_call.is_match(line)
                && !self.in_array_strict.is_match(line)
            {
                matched_labels.push(
                    "in_array() on request input without the strict third argument `true` \
                     (loose element comparison)",
                );
            }

            let has_hash_pair = self.hash_var.find_iter(line).count() >= 2;
            if has_loose_cmp && (self.md5_sha1_call.is_match(line) || has_hash_pair) {
                matched_labels.push(
                    "hash value compared with a loose ==/!= (\"0e...\" magic-hash collision); \
                     use hash_equals() or ===",
                );
            }

            if !matched_labels.is_empty() {
                findings.extend(crate::boundary::finding::from_source(
                    (&self.rule_id, Severity::Error),
                    "PHP type-juggling vulnerable comparison",
                    format!(
                        "Line uses a type-juggling-vulnerable comparison: {}. Fix: use strict \
                         ===/!== comparison, hash_equals() for hash/token comparison, and the \
                         strict third argument on in_array().",
                        matched_labels.join(", ")
                    ),
                    input.file,
                    (line_number, Some(line)),
                ));
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use crate::boundary::fixture::run_manifest_fixture_parity;

    use super::TypeJugglingValidator;

    #[test]
    fn cyberskills_type_juggle() -> Result<(), Box<dyn std::error::Error>> {
        let v = TypeJugglingValidator::new()?;
        run_manifest_fixture_parity(
            &v,
            "tests/fixtures/cyberskills/web.type-juggling/bad/vuln.php",
            "tests/fixtures/cyberskills/web.type-juggling/good/safe.php",
        )?;
        Ok(())
    }
}
