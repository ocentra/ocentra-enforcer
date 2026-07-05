//! `CYBER-HEADERS-HSTS.1` + `CYBER-HEADERS-CSP.1` +
//! `CYBER-COOKIE-SECURE.1` (all T1) — harvest target 6 (h11 workpack):
//! HSTS/CSP/cookie predicates ported from
//! `vendor/anthropic-cybersecurity-skills/skills/performing-security-headers-audit/scripts/agent.py`
//! (L46-80). The original agent.py performs a live HTTP GET
//! (`SecurityHeadersAgent.fetch_headers`) then inspects the response
//! headers/cookies; these validators run the SAME predicates over a
//! generic JSON snapshot of `{ headers, cookies }` (the shape a captured
//! response, an HTTP archive entry, or an already-fetched header dump
//! would carry), dropping the network call entirely.

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;
use std::collections::BTreeMap;

#[derive(Debug, Default, serde::Deserialize)]
struct CookieSnapshot {
    #[serde(default)]
    name: String,
    #[serde(default)]
    secure: bool,
    #[serde(default)]
    httponly: bool,
    #[serde(default)]
    samesite: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct HeadersSnapshot {
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    cookies: Vec<CookieSnapshot>,
}

fn parse(source: &str) -> Option<HeadersSnapshot> {
    serde_json::from_str(source).ok()
}

/// Case-insensitive header lookup (HTTP header names are case-insensitive
/// on the wire; agent.py mirrors both cased forms with `.get(a, .get(b))`).
fn header<'a>(snapshot: &'a HeadersSnapshot, name: &str) -> Option<&'a str> {
    snapshot
        .headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

/// `CYBER-HEADERS-HSTS.1` — `Strict-Transport-Security` missing, or present
/// with `max-age < 31536000` (agent.py `check_hsts`: severity High if
/// absent, Medium if `max_age` under one year).
pub struct HstsMissingOrWeakValidator {
    rule_id: RuleId,
    max_age: Regex,
}

impl HstsMissingOrWeakValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-HEADERS-HSTS.1".parse()?,
            max_age: Regex::new(r"(?i)max-age=(\d+)")
                .map_err(|err| DecodeError::new("cyberskillsHstsMaxAgeRegex", err.to_string()))?,
        })
    }
}

impl Validator for HstsMissingOrWeakValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(snapshot) = parse(input.source) else {
            return Vec::new();
        };
        let hsts = header(&snapshot, "Strict-Transport-Security");
        let detail = match hsts {
            None => Some(
                "no `Strict-Transport-Security` header present. Fix: add \
                 `Strict-Transport-Security: max-age=31536000; includeSubDomains; preload`."
                    .to_owned(),
            ),
            Some(value) => {
                let max_age: Option<u64> = self
                    .max_age
                    .captures(value)
                    .and_then(|c| c.get(1))
                    .and_then(|m| m.as_str().parse().ok());
                match max_age {
                    Some(seconds) if seconds < 31_536_000 => Some(format!(
                        "`Strict-Transport-Security: {value}` has `max-age={seconds}`, under \
                         one year. Fix: increase `max-age` to at least 31536000."
                    )),
                    Some(_) => None,
                    None => Some(format!(
                        "`Strict-Transport-Security: {value}` has no parseable `max-age` \
                         directive. Fix: include `max-age=31536000` (or greater)."
                    )),
                }
            }
        };
        detail
            .map(|detail| {
                vec![Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "HSTS header missing or weak".to_owned(),
                    detail,
                    file: input.file.clone(),
                    line: 1,
                    snippet: None,
                }]
            })
            .unwrap_or_default()
    }
}

/// `CYBER-HEADERS-CSP.1` — `Content-Security-Policy` missing, or present
/// with `'unsafe-inline'`/`'unsafe-eval'` (agent.py `check_csp`: severity
/// High for either condition).
pub struct CspMissingValidator {
    rule_id: RuleId,
}

impl CspMissingValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-HEADERS-CSP.1".parse()?,
        })
    }
}

impl Validator for CspMissingValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(snapshot) = parse(input.source) else {
            return Vec::new();
        };
        let csp = header(&snapshot, "Content-Security-Policy");
        let detail = match csp {
            None => Some(
                "no `Content-Security-Policy` header present. Fix: add a restrictive CSP \
                 (e.g. `default-src 'self'`)."
                    .to_owned(),
            ),
            Some(value) => {
                let mut issues = Vec::new();
                if value.contains("'unsafe-inline'") {
                    issues.push("`'unsafe-inline'` allows inline script execution (XSS risk)");
                }
                if value.contains("'unsafe-eval'") {
                    issues.push("`'unsafe-eval'` allows eval() calls (XSS risk)");
                }
                if issues.is_empty() {
                    None
                } else {
                    Some(format!(
                        "Content-Security-Policy has weaknesses: {}. Fix: remove \
                         `'unsafe-inline'`/`'unsafe-eval'` and use nonces/hashes instead.",
                        issues.join("; ")
                    ))
                }
            }
        };
        detail
            .map(|detail| {
                vec![Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "CSP header missing or weak".to_owned(),
                    detail,
                    file: input.file.clone(),
                    line: 1,
                    snippet: None,
                }]
            })
            .unwrap_or_default()
    }
}

/// `CYBER-COOKIE-SECURE.1` — a cookie missing `Secure`, `HttpOnly`, or a
/// recognized `SameSite` value is flagged (agent.py's cookie-attribute
/// capture in `fetch_headers`, generalized into an explicit predicate — the
/// original script only records the attributes, this validator is the
/// enforcement gate the corpus leaves as an exercise).
pub struct CookieSecureHttponlySamesiteValidator {
    rule_id: RuleId,
}

impl CookieSecureHttponlySamesiteValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-COOKIE-SECURE.1".parse()?,
        })
    }
}

fn samesite_ok(samesite: &Option<String>) -> bool {
    matches!(
        samesite.as_deref().map(str::to_ascii_lowercase).as_deref(),
        Some("strict") | Some("lax") | Some("none")
    )
}

impl Validator for CookieSecureHttponlySamesiteValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(snapshot) = parse(input.source) else {
            return Vec::new();
        };
        let mut findings = Vec::new();
        for cookie in &snapshot.cookies {
            let mut missing = Vec::new();
            if !cookie.secure {
                missing.push("Secure");
            }
            if !cookie.httponly {
                missing.push("HttpOnly");
            }
            if !samesite_ok(&cookie.samesite) {
                missing.push("SameSite");
            }
            if missing.is_empty() {
                continue;
            }
            findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "Cookie missing Secure/HttpOnly/SameSite".to_owned(),
                detail: format!(
                    "Cookie `{}` is missing: {}. Fix: set `Secure`, `HttpOnly`, and an explicit \
                     `SameSite=Strict|Lax|None` attribute on every session/auth cookie.",
                    cookie.name,
                    missing.join(", ")
                ),
                file: input.file.clone(),
                line: 1,
                snippet: None,
            });
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::{
        CookieSecureHttponlySamesiteValidator, CspMissingValidator, HstsMissingOrWeakValidator,
    };

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_headers_hsts() -> Result<(), Box<dyn std::error::Error>> {
        let validator = HstsMissingOrWeakValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/web.headers.hsts-missing-or-weak/bad/missing.json",
            "tests/fixtures/cyberskills/web.headers.hsts-missing-or-weak/good/strong.json",
        )?;
        Ok(())
    }

    #[test]
    fn cyberskills_headers_csp() -> Result<(), Box<dyn std::error::Error>> {
        let validator = CspMissingValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/web.headers.csp-missing/bad/missing.json",
            "tests/fixtures/cyberskills/web.headers.csp-missing/good/restrictive.json",
        )?;
        Ok(())
    }

    #[test]
    fn cyberskills_cookie_secure() -> Result<(), Box<dyn std::error::Error>> {
        let validator = CookieSecureHttponlySamesiteValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/web.cookie.secure-httponly-samesite/bad/insecure.json",
            "tests/fixtures/cyberskills/web.cookie.secure-httponly-samesite/good/secure.json",
        )?;
        Ok(())
    }
}
