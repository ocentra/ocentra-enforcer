//! `CYBER-CORS.1` (T1) — harvest target: CORS misconfiguration predicate
//! for `vendor/anthropic-cybersecurity-skills/skills/testing-cors-misconfiguration`.
//!
//! Harvest note: this vendor skill has no `scripts/agent.py` (or any other
//! inline-predicate script) — its `SKILL.md` is a curl/Burp Suite
//! penetration-testing *workflow* document (manual `Origin` header probing,
//! preflight inspection, exploit-PoC crafting). There is no source
//! predicate to port verbatim. Per the h11 workpack fallback, this
//! validator instead implements the well-known deterministic CORS check
//! the skill's own "Key Concepts" table and Step 2/Step 7 curl comments
//! describe: an `Access-Control-Allow-Origin` (ACAO) value of `*` or
//! `null` combined ANYWHERE in the source with
//! `Access-Control-Allow-Credentials: true` is a spec-violating,
//! browser-exploitable combination (SKILL.md "Wildcard CORS" concept row:
//! "allows any origin but prohibits credentials"; Scenario 1/2: origin
//! reflection / `null` origin + credentials => data theft). The two
//! headers may be set on different lines/statements (e.g. two separate
//! `res.header(...)` calls), so the whole source is scanned, not a single
//! line in isolation; a single line setting both is naturally covered by
//! the same cross-source check. A bare wildcard/`null` ACAO with no
//! credentials header is still permissive (any origin can read
//! non-credentialed responses) but is not the spec-violating combination,
//! so it is downgraded to `Severity::Warning`. A scoped (real-domain)
//! origin combined with credentials is the correct, spec-compliant
//! pattern and is not flagged.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// `CYBER-CORS.1` — ACAO wildcard/`null` + ACAC `true` CORS misconfiguration.
pub struct CorsMisconfigValidator {
    rule_id: RuleId,
    acao_wildcard_or_null: Regex,
    acac_true: Regex,
}

impl CorsMisconfigValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-CORS.1".parse()?,
            // Matches `Access-Control-Allow-Origin` set to a bare `*` or
            // `null`, across header-text (`Name: value`), JSON
            // (`"Name": "value"`), and code-call (`'Name', 'value'`)
            // shapes — the separator class covers quotes, `:`, `=`, `,`,
            // and plain whitespace (e.g. nginx `add_header Name value;`).
            // The trailing-delimiter group (quote/whitespace/`;`/`,`/`)`/
            // `}`/`]`/end-of-line) stands in for a word boundary: `\b` does not
            // work after a non-word `*` character, and it also excludes a
            // subdomain-glob value like `*.example.com`, which is not a
            // literal wildcard origin.
            acao_wildcard_or_null: Regex::new(
                r#"(?i)Access-Control-Allow-Origin[\s'":=,]+(\*|null)(?:['"]|[\s;,)}\]]|$)"#,
            )
            .map_err(|err| DecodeError::new("cyberskillsCorsAcaoRegex", err.to_string()))?,
            acac_true: Regex::new(
                r#"(?i)Access-Control-Allow-Credentials[\s'":=,]+true(?:['"]|[\s;,)}\]]|$)"#,
            )
            .map_err(|err| DecodeError::new("cyberskillsCorsAcacRegex", err.to_string()))?,
        })
    }
}

impl Validator for CorsMisconfigValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut acao_hit: Option<(u32, &'static str)> = None;
        let mut acac_line: Option<u32> = None;

        for (index, line) in input.source.lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
            if let (None, Some(captures)) = (acao_hit, self.acao_wildcard_or_null.captures(line)) {
                let value = captures.get(1).map(|m| m.as_str()).unwrap_or_default();
                let label = if value.eq_ignore_ascii_case("null") {
                    "null"
                } else {
                    "*"
                };
                acao_hit = Some((line_number, label));
            }
            if acac_line.is_none() && self.acac_true.is_match(line) {
                acac_line = Some(line_number);
            }
        }

        match (acao_hit, acac_line) {
            (Some((acao_at, label)), Some(acac_at)) => {
                let line = acao_at.min(acac_at);
                vec![Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "CORS wildcard/null origin combined with credentials".to_owned(),
                    detail: format!(
                        "`Access-Control-Allow-Origin: {label}` is combined with \
                         `Access-Control-Allow-Credentials: true`. Any website can send \
                         credentialed cross-origin requests and read the response, exfiltrating \
                         authenticated data. Fix: replace the wildcard/`null` origin with an \
                         explicit allowlist of trusted origins, or drop \
                         `Access-Control-Allow-Credentials` if credentials are not required."
                    ),
                    file: input.file.clone(),
                    line,
                    snippet: None,
                }]
            }
            (Some((acao_at, label)), None) => {
                vec![Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Warning,
                    title: "Permissive CORS origin".to_owned(),
                    detail: format!(
                        "`Access-Control-Allow-Origin: {label}` allows any origin to read \
                         non-credentialed responses. Fix: scope `Access-Control-Allow-Origin` to \
                         an explicit allowlist of trusted origins unless the endpoint is \
                         intentionally public."
                    ),
                    file: input.file.clone(),
                    line: acao_at,
                    snippet: None,
                }]
            }
            (None, _) => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::CorsMisconfigValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_web_cors() -> Result<(), Box<dyn std::error::Error>> {
        let validator = CorsMisconfigValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/web.cors-misconfig/bad/wildcard_creds.txt",
            "tests/fixtures/cyberskills/web.cors-misconfig/good/scoped.txt",
        )?;
        Ok(())
    }
}
