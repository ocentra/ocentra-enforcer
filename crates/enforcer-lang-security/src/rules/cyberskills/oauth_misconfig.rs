//! `CYBER-OAUTH.1` (T1) — harvest target: `vendor/anthropic-cybersecurity-skills/
//! skills/exploiting-oauth-misconfiguration/SKILL.md` (Step 1 "Identify the
//! grant type: ... Implicit: response_type=token"; Step 2 "Test Redirect URI
//! Manipulation" and its `scripts/agent.py::test_redirect_uri_bypasses`; the
//! "Key Concepts" table rows "Implicit Flow" and "Open Redirect").
//!
//! Harvest note: the vendor skill is a live-target penetration-testing
//! workflow — curl bypass lists, Burp Suite interception, and a
//! `requests`-based `agent.py` that probes a *running* OAuth provider
//! (`test_redirect_uri_bypasses`, `test_state_parameter`,
//! `test_pkce_requirement`). None of that survives as a static-source
//! predicate. This validator instead narrows the vendor's riskiest
//! MISCONFIGURATION SHAPES to deterministic, offline, line-scan patterns a
//! static source file can actually exhibit:
//!
//! 1. **Implicit flow** — `response_type` (or camelCase `responseType`) set
//!    to the bare value `token`, across bare/quoted/JSON spellings
//!    (`response_type=token`, `response_type: "token"`,
//!    `"response_type":"token"`, `responseType: 'token'`). This is the
//!    vendor's "Implicit Flow" concept-table row: deprecated, returns the
//!    access token directly in the URL fragment (SKILL.md Scenario 3
//!    "Implicit Flow Token Theft").
//! 2. **Wildcard `redirect_uri`** — a `redirect_uri`/`redirectUri`/
//!    `redirect_uris` value that is a bare `*`, contains a `*` glob, or is a
//!    scheme+wildcard (`https://*`). This is the registration-time root
//!    cause the vendor's bypass list
//!    (`agent.py::test_redirect_uri_bypasses`, SKILL.md Scenario 1
//!    "Redirect URI Subdomain Bypass") exploits at request time.
//! 3. **User-controlled `redirect_uri`** — `redirect_uri`/`redirectUri`
//!    assigned directly from request input (`req.query.*`, `req.body.*`,
//!    `req.params.*`, or a `request.args.get("redirect_uri")`-style
//!    accessor). This is the vendor's "Open Redirect" concept-table entry:
//!    using the OAuth callback parameter itself as an open redirect.
//!
//! Deliberately NOT ported (too ambiguous for a line scan, per the h11
//! workpack's false-positive budget): missing/predictable `state`
//! (`agent.py::test_state_parameter`), missing PKCE
//! (`agent.py::test_pkce_requirement`), scope escalation, and
//! code-reuse/token-leakage checks — each requires live-request semantics or
//! cross-request state a static scanner cannot observe. `response_type=code`
//! (with or without PKCE), a hybrid `response_type` value (e.g.
//! `code token`, which does not match the bare `token` shape above), an
//! exact-match allowlisted `redirect_uri` literal, and `state`/
//! `code_challenge` presence are therefore never flagged. A `redirect_uris`
//! value nested inside a JSON array of quoted strings is also not scanned
//! for a wildcard (crossing the array's inner quotes would widen the match
//! window enough to risk false positives on unrelated trailing content); only
//! the plain bare/single-string spellings above are covered.

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// One high-confidence OAuth-misconfiguration line-scan pattern.
struct OauthMisconfigPattern {
    regex: &'static str,
    label: &'static str,
}

/// The three deterministic misconfiguration shapes named in the module doc
/// comment above.
const OAUTH_MISCONFIG_PATTERNS_SRC: &[OauthMisconfigPattern] = &[
    // 1. Implicit flow: response_type/responseType set to the bare value
    // `token` (bare, quoted, or JSON key/value spelling).
    OauthMisconfigPattern {
        regex: r#"(?i)response[_-]?type["']?\s*[:=]\s*["']?token\b"#,
        label: "implicit grant flow (response_type=token) — returns the access token in the URL fragment",
    },
    // 2. Wildcard redirect_uri: bare `*`, a value containing `*`, or a
    // scheme+wildcard (`https://*`), for redirect_uri/redirectUri/redirect_uris.
    OauthMisconfigPattern {
        regex: r#"(?i)(?:redirect_uris?|redirectUri)["']?\s*[:=]\s*["']?[^"'\n]{0,80}\*"#,
        label: "wildcard redirect_uri — accepts any callback URL",
    },
    // 3. User-controlled redirect_uri: assigned directly from request input
    // (Express `req.query`/`req.body`/`req.params`, or a Flask/Django-style
    // `request.args.get("redirect_uri")` accessor).
    OauthMisconfigPattern {
        regex: r#"(?i)(?:redirect_uris?|redirectUri)["']?\s*[:=]\s*req(?:uest)?\.(?:query|body|params|args)\b|req(?:uest)?\.(?:query|body|params|args)\.redirect(?:_uri)?\b|(?:query|body|params|args|GET|POST|values)\.get\(\s*["'](?:redirect_uri|redirect)["']"#,
        label: "user-controlled redirect_uri — assigned directly from request input (open redirect)",
    },
];

/// `CYBER-OAUTH.1` — flags OAuth misconfigurations: the implicit grant flow
/// (`response_type=token`), a wildcard `redirect_uri`, and a `redirect_uri`
/// assigned directly from user-controlled request input.
pub struct OauthMisconfigValidator {
    rule_id: RuleId,
    patterns: Vec<(Regex, &'static str)>,
}

impl OauthMisconfigValidator {
    pub fn new() -> Result<Self, DecodeError> {
        let mut patterns = Vec::with_capacity(OAUTH_MISCONFIG_PATTERNS_SRC.len());
        for entry in OAUTH_MISCONFIG_PATTERNS_SRC {
            let regex = Regex::new(entry.regex).map_err(|err| {
                DecodeError::new("cyberskillsOauthMisconfigPattern", err.to_string())
            })?;
            patterns.push((regex, entry.label));
        }
        Ok(Self {
            rule_id: "CYBER-OAUTH.1".parse()?,
            patterns,
        })
    }
}

impl Validator for OauthMisconfigValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (index, line) in input.source.lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
            let mut matched_labels: Vec<&str> = Vec::new();
            for (regex, label) in &self.patterns {
                if regex.is_match(line) && !matched_labels.contains(label) {
                    matched_labels.push(*label);
                }
            }
            if !matched_labels.is_empty() {
                findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "OAuth misconfiguration".to_owned(),
                    detail: format!(
                        "Line contains an OAuth misconfiguration: {}. Fix: use the \
                         authorization-code flow with PKCE, register an exact-match \
                         allowlisted redirect_uri, and never derive redirect_uri from \
                         user-controlled request input.",
                        matched_labels.join(", ")
                    ),
                    file: input.file.clone(),
                    line: line_number,
                    snippet: Some(line.to_owned()),
                });
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::OauthMisconfigValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_oauth_misconfig() -> Result<(), Box<dyn std::error::Error>> {
        let v = OauthMisconfigValidator::new()?;
        run_fixture_parity(
            &v,
            &manifest_dir(),
            "tests/fixtures/cyberskills/web.oauth-misconfig/bad/vuln.js",
            "tests/fixtures/cyberskills/web.oauth-misconfig/good/safe.js",
        )?;
        Ok(())
    }
}
